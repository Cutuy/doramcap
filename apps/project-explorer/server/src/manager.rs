//! Registry-backed per-app coordination and conservative restart discovery.
use crate::{
    lifecycle::{AppLifecycle, Attempt, Launch, Status},
    registry::{Registration, Registry},
};
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use workload_docker::compose::{DockerAdapter, OwnedResource};

fn nested_run(run: &str) -> Option<&str> {
    let (generation, suffix) = run.strip_prefix("gal-")?.split_once('-')?;
    if generation.len() != 32
        || !generation
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || suffix.is_empty()
        || suffix.len() > 3
        || suffix.starts_with('0')
        || !suffix.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some(generation)
}

/// Attribute a nested resource only when its generation policy and anchor agree.
fn nested_owner<'a>(
    registry: &'a Registry,
    data: &Path,
    resource: &OwnedResource,
) -> Option<&'a Registration> {
    let generation = nested_run(&resource.run)?;
    let root = crate::artifact_fs::root(data).ok()?;
    let read = |path: &str| -> Option<Vec<u8>> {
        let file = crate::artifact_fs::open(&root, path, false).ok()?;
        let mut bytes = Vec::new();
        file.take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        (bytes.len() <= 2 * 1024 * 1024).then_some(bytes)
    };
    let mut owner = None;
    for app in registry.iter().filter(|app| app.nested_runs) {
        if !app.nested_projects.iter().any(|project| {
            resource.project.as_deref()
                == Some(format!("dc-{}-{}", resource.run, project.key).as_str())
        }) {
            continue;
        }
        let directory = format!("{}-{generation}", app.id);
        let Some(policy) =
            read(&format!("{directory}/metadata/nested-policy.json")).and_then(|bytes| {
                serde_json::from_slice::<Vec<crate::registry::NestedProject>>(&bytes).ok()
            })
        else {
            continue;
        };
        if policy != app.nested_projects {
            continue;
        }
        let Some(anchor) = read(&format!(
            "{directory}/private/recovery/{}.json",
            resource.run
        ))
        .and_then(|bytes| {
            serde_json::from_slice::<workload_docker::workflows::recovery::RecoveryAnchor>(&bytes)
                .ok()
        }) else {
            continue;
        };
        if anchor.run != resource.run
            || anchor.projects.len() != policy.len()
            || anchor.projects.iter().zip(&policy).any(|(a, p)| {
                a.key != p.key
                    || a.remove_volumes != p.remove_volumes
                    || a.resolution_sha256.len() != 64
                    || !a
                        .resolution_sha256
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
        {
            continue;
        }
        if owner.replace(app).is_some() {
            return None;
        }
    }
    owner
}

/// A registered application and its independent lifecycle coordinator.
#[derive(Clone)]
pub struct AppEntry {
    pub registration: Registration,
    pub lifecycle: AppLifecycle,
}

/// Errors accepting a request from the gallery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnknownApp,
    ShuttingDown,
    Blocked(String),
    Conflict { owner: String, resource: String },
}

/// The manager exposes server-owned registrations and per-app coordination.
/// Startup discovery must run before this value is made available to clients.
#[derive(Clone)]
pub struct Manager {
    apps: BTreeMap<String, AppEntry>,
    admission: Arc<tokio::sync::Mutex<()>>,
}
impl Manager {
    /// Query the Engine before accepting launches. Any matching labeled resources
    /// block their registered app. An unrecognized ownership record blocks every
    /// app because its ownership cannot be attributed safely.
    pub async fn discover(
        registry: Registry,
        docker: &DockerAdapter,
        idle_timeout: Duration,
    ) -> workload_docker::compose::Result<Self> {
        let resources = docker.discover_runs("gal-").await?;
        Ok(Self::from_resources_at(
            registry,
            idle_timeout,
            &resources,
            None,
        ))
    }
    /// Discover resources and attribute nested runs using private generation anchors.
    /// Unknown or inconsistent ownership still blocks every registered app.
    pub async fn discover_at(
        registry: Registry,
        docker: &DockerAdapter,
        idle_timeout: Duration,
        data: &Path,
    ) -> workload_docker::compose::Result<Self> {
        let resources = docker.discover_runs("gal-").await?;
        Ok(Self::from_resources_at(
            registry,
            idle_timeout,
            &resources,
            Some(data),
        ))
    }
    #[cfg(test)]
    pub(crate) fn from_resources(
        registry: Registry,
        idle_timeout: Duration,
        resources: &[OwnedResource],
    ) -> Self {
        Self::from_resources_at(registry, idle_timeout, resources, None)
    }
    fn from_resources_at(
        registry: Registry,
        idle_timeout: Duration,
        resources: &[OwnedResource],
        data: Option<&Path>,
    ) -> Self {
        let mut issues: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        let mut ambiguous = Vec::new();
        for resource in resources {
            let owner = if resource
                .run
                .strip_prefix("gal-")
                .is_some_and(|value| value.as_bytes().get(32) == Some(&b'-'))
            {
                data.and_then(|root| nested_owner(&registry, root, resource))
            } else {
                registry.iter().find(|app| {
                    resource.project.as_deref()
                        == Some(format!("dc-{}-{}", resource.run, app.id).as_str())
                })
            };
            let description = format!("{}:{} (run {})", resource.kind, resource.id, resource.run);
            if let Some(app) = owner {
                issues.entry(&app.id).or_default().push(description);
            } else {
                ambiguous.push(description);
            }
        }
        let apps = registry
            .iter()
            .map(|app| {
                let mut owned = issues.remove(app.id.as_str()).unwrap_or_default();
                owned.extend(ambiguous.iter().cloned());
                let lifecycle = if owned.is_empty() {
                    AppLifecycle::new(idle_timeout)
                } else {
                    AppLifecycle::blocked(
                        idle_timeout,
                        format!(
                            "existing manager resources require cleanup: {}",
                            owned.join(", ")
                        ),
                    )
                };
                (
                    app.id.clone(),
                    AppEntry {
                        registration: app.clone(),
                        lifecycle,
                    },
                )
            })
            .collect();
        Self {
            apps,
            admission: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
    /// Get a registration and coordinator by public ID.
    pub fn get(&self, id: &str) -> Option<&AppEntry> {
        self.apps.get(id)
    }
    /// Iterate registered entries in stable ID order.
    pub fn iter(&self) -> impl Iterator<Item = &AppEntry> {
        self.apps.values()
    }
    /// Accept a launch for a known app, or return a visible blocked/unknown error.
    /// Acceptance returns without yielding again; cancellation before return
    /// creates no session. The caller must queue startup before its next await.
    pub async fn launch(&self, id: &str, now: Instant) -> Result<Launch, Error> {
        let app = self.get(id).ok_or(Error::UnknownApp)?;
        let mut accepted_at = now;
        loop {
            let admission = self.admission.lock().await;
            for other in self.iter().filter(|other| other.registration.id != id) {
                if let Some(resource) = app
                    .registration
                    .exclusive_resources
                    .iter()
                    .find(|resource| other.registration.exclusive_resources.contains(resource))
                    && !matches!(other.lifecycle.status().await, Status::Stopped { .. })
                {
                    return Err(Error::Conflict {
                        owner: other.registration.id.clone(),
                        resource: (*resource).clone(),
                    });
                }
            }
            match app.lifecycle.try_launch(accepted_at).await {
                Attempt::Accepted(launch) => return Ok(launch),
                Attempt::Blocked(error) => return Err(Error::Blocked(error)),
                Attempt::Stopping => {
                    drop(admission);
                    app.lifecycle.wait_until_not_stopping().await;
                    accepted_at = Instant::now();
                }
            }
        }
    }
    /// Return the current status of a known app.
    pub async fn status(&self, id: &str) -> Result<Status, Error> {
        Ok(self
            .get(id)
            .ok_or(Error::UnknownApp)?
            .lifecycle
            .status()
            .await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn discovered_project_blocks_launch_and_unknown_ids_are_rejected() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        let resources = vec![OwnedResource {
            artifacts: None,
            run: "gal-0123".into(),
            project: Some("dc-gal-0123-activity-demo".into()),
            kind: "container",
            id: "old-container".into(),
        }];
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &resources);

        // Act
        let blocked = manager.launch("activity-demo", Instant::now()).await;
        let unknown = manager.launch("other", Instant::now()).await;

        // Assert
        assert!(
            matches!(blocked, Err(Error::Blocked(message)) if message.contains("old-container"))
        );
        assert_eq!(unknown, Err(Error::UnknownApp));
    }

    #[tokio::test]
    async fn ambiguous_owned_resource_blocks_all_registered_apps() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        let resources = vec![OwnedResource {
            artifacts: None,
            run: "gal-0123".into(),
            project: None,
            kind: "network",
            id: "orphan".into(),
        }];
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &resources);

        // Act
        let result = manager.launch("activity-demo", Instant::now()).await;

        // Assert
        assert!(matches!(result, Err(Error::Blocked(message)) if message.contains("orphan")));
    }
    #[tokio::test]
    async fn anchored_nested_resources_block_only_their_owner() {
        // Arrange
        let mut registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let app = registry.get_mut("activity-demo").unwrap();
        app.nested_runs = true;
        app.nested_projects = vec![crate::registry::NestedProject {
            key: "demo".into(),
            remove_volumes: true,
        }];
        let data =
            std::env::temp_dir().join(format!("gallery-discovery-{:032x}", rand::random::<u128>()));
        let token = "0123456789abcdef0123456789abcdef";
        let run = format!("gal-{token}-1");
        let generation = data.join(format!("activity-demo-{token}"));
        std::fs::create_dir_all(generation.join("metadata")).unwrap();
        std::fs::create_dir_all(generation.join("private/recovery")).unwrap();
        std::fs::write(
            generation.join("metadata/nested-policy.json"),
            serde_json::to_vec(&app.nested_projects).unwrap(),
        )
        .unwrap();
        let anchor = serde_json::json!({"run": run, "projects": [{"key": "demo", "remove_volumes": true,
            "resolution_sha256": "a".repeat(64)}], "filesystem_baselines": {}});
        let anchor_path = generation
            .join("private/recovery")
            .join(format!("{run}.json"));
        std::fs::write(&anchor_path, serde_json::to_vec(&anchor).unwrap()).unwrap();
        let resources = vec![OwnedResource {
            artifacts: None,
            run: run.clone(),
            project: Some(format!("dc-{run}-demo")),
            kind: "volume",
            id: "nested-volume".into(),
        }];

        // Act / Assert: the known owner is blocked and another app can launch.
        let manager = Manager::from_resources_at(
            registry.clone(),
            Duration::from_secs(120),
            &resources,
            Some(&data),
        );
        assert!(matches!(
            manager.launch("activity-demo", Instant::now()).await,
            Err(Error::Blocked(_))
        ));
        assert!(manager.launch("storage-demo", Instant::now()).await.is_ok());

        // Act / Assert: an inconsistent private anchor makes ownership ambiguous.
        std::fs::write(&anchor_path, b"{}").unwrap();
        let manager =
            Manager::from_resources_at(registry, Duration::from_secs(120), &resources, Some(&data));
        assert!(matches!(
            manager.launch("activity-demo", Instant::now()).await,
            Err(Error::Blocked(_))
        ));
        assert!(matches!(
            manager.launch("storage-demo", Instant::now()).await,
            Err(Error::Blocked(_))
        ));
        std::fs::remove_dir_all(data).unwrap();
    }

    #[tokio::test]
    async fn conflicting_concurrent_launches_are_serialized_until_cleanup() {
        // Arrange
        let mut registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        registry
            .get_mut("activity-demo")
            .unwrap()
            .exclusive_resources = vec!["test-gpu".into()];
        registry
            .get_mut("storage-demo")
            .unwrap()
            .exclusive_resources = vec!["test-gpu".into()];
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &[]);

        // Act
        let (activity, storage) = tokio::join!(
            manager.launch("activity-demo", Instant::now()),
            manager.launch("storage-demo", Instant::now())
        );

        // Assert
        let (owner, other, accepted) = match (activity, storage) {
            (Ok(launch), Err(Error::Conflict { owner, resource })) => {
                assert_eq!(owner, "activity-demo");
                assert_eq!(resource, "test-gpu");
                ("activity-demo", "storage-demo", launch)
            }
            (Err(Error::Conflict { owner, resource }), Ok(launch)) => {
                assert_eq!(owner, "storage-demo");
                assert_eq!(resource, "test-gpu");
                ("storage-demo", "activity-demo", launch)
            }
            result => panic!("unexpected admission results: {result:?}"),
        };

        // Act
        let lifecycle = &manager.get(owner).unwrap().lifecycle;
        lifecycle.fail(accepted.generation).await;
        let during_cleanup = manager.launch(other, Instant::now()).await;
        lifecycle.cleaned(accepted.generation, Ok(None)).await;
        let after_cleanup = manager.launch(other, Instant::now()).await;

        // Assert
        assert!(matches!(during_cleanup, Err(Error::Conflict { .. })));
        assert!(after_cleanup.unwrap().start);
    }

    #[tokio::test]
    async fn waiting_for_cleanup_does_not_block_an_unrelated_app() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &[]);
        let first = manager
            .launch("activity-demo", Instant::now())
            .await
            .unwrap();
        let lifecycle = &manager.get("activity-demo").unwrap().lifecycle;
        lifecycle.fail(first.generation).await;
        let waiting_manager = manager.clone();
        let waiter = tokio::spawn(async move {
            waiting_manager
                .launch("activity-demo", Instant::now())
                .await
        });
        tokio::task::yield_now().await;

        // Act
        let unrelated = tokio::time::timeout(
            Duration::from_secs(1),
            manager.launch("storage-demo", Instant::now()),
        )
        .await;
        lifecycle.cleaned(first.generation, Ok(None)).await;
        let resumed = waiter.await.unwrap();

        // Assert
        assert!(unrelated.unwrap().unwrap().start);
        assert!(resumed.unwrap().start);
    }
}
