//! Registry-backed per-app coordination and conservative restart discovery.
use crate::{
    lifecycle::{AppLifecycle, Attempt, Launch, Status},
    registry::{Registration, Registry},
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use workload_docker::compose::{DockerAdapter, OwnedResource};

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
    Busy { owner: String },
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
        let resources = docker.resources().discover_runs("gal-").await?;
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
        let resources = docker.resources().discover_runs("gal-").await?;
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
            if let (Some(root), Some(artifacts)) = (data, resource.artifacts.as_deref())
                && !Path::new(artifacts).starts_with(root)
            {
                continue;
            }
            let owner = data.and_then(|root| {
                resource.artifacts.as_ref().and_then(|artifacts| {
                    registry.iter().find(|app| {
                        resource
                            .run
                            .strip_prefix("gal-")
                            .and_then(|id| id.get(..32))
                            .is_some_and(|id| {
                                Path::new(artifacts)
                                    .starts_with(root.join(format!("{}-{id}", app.id)))
                            })
                    })
                })
            });
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
            if let Status::Blocked { error } = app.lifecycle.status().await {
                return Err(Error::Blocked(error));
            }
            for other in self.iter().filter(|other| other.registration.id != id) {
                if !matches!(other.lifecycle.status().await, Status::Stopped { .. }) {
                    return Err(Error::Busy {
                        owner: other.registration.id.clone(),
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
            kind: "container".into(),
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
    async fn another_directory_does_not_block_launch() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        let resources = [OwnedResource {
            artifacts: Some("/other-repository/data/run".into()),
            run: "gal-0123".into(),
            project: Some("dc-gal-0123-activity-demo".into()),
            kind: "container".into(),
            id: "other-container".into(),
        }];
        let manager = Manager::from_resources_at(
            registry,
            Duration::from_secs(120),
            &resources,
            Some(Path::new("/this-repository/data")),
        );

        // Act
        let launched = manager.launch("activity-demo", Instant::now()).await;

        // Assert
        assert!(launched.is_ok(), "{launched:?}");
    }

    #[tokio::test]
    async fn ambiguous_owned_resource_blocks_all_registered_apps() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        let resources = vec![OwnedResource {
            artifacts: None,
            run: "gal-0123".into(),
            project: None,
            kind: "network".into(),
            id: "orphan".into(),
        }];
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &resources);

        // Act
        let result = manager.launch("activity-demo", Instant::now()).await;

        // Assert
        assert!(matches!(result, Err(Error::Blocked(message)) if message.contains("orphan")));
    }
    #[tokio::test]
    async fn scoped_nested_resources_block_only_their_owner() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let data =
            std::env::temp_dir().join(format!("gallery-discovery-{:032x}", rand::random::<u128>()));
        let token = "0123456789abcdef0123456789abcdef";
        let run = format!("gal-{token}-1");
        let generation = data.join(format!("activity-demo-{token}"));
        let mut resources = vec![OwnedResource {
            artifacts: Some(
                generation
                    .join("private/telemetry")
                    .join(&run)
                    .to_str()
                    .unwrap()
                    .to_owned(),
            ),
            run: run.clone(),
            project: Some(format!("dc-{run}-demo")),
            kind: "volume".into(),
            id: "nested-volume".into(),
        }];

        // Act / Assert: retained owned resources block admission of a different experiment.
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
        assert!(matches!(
            manager.launch("storage-demo", Instant::now()).await,
            Err(Error::Busy { .. })
        ));

        // Act / Assert: missing directory ownership labels make ownership ambiguous.
        resources[0].artifacts = None;
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
    }

    #[tokio::test]
    async fn conflicting_concurrent_launches_are_serialized_until_cleanup() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let manager = Manager::from_resources(registry, Duration::from_secs(120), &[]);

        // Act
        let (activity, storage) = tokio::join!(
            manager.launch("activity-demo", Instant::now()),
            manager.launch("storage-demo", Instant::now())
        );

        // Assert
        let (owner, other, accepted) = match (activity, storage) {
            (Ok(launch), Err(Error::Busy { owner })) => {
                assert_eq!(owner, "activity-demo");
                ("activity-demo", "storage-demo", launch)
            }
            (Err(Error::Busy { owner }), Ok(launch)) => {
                assert_eq!(owner, "storage-demo");
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
        assert!(matches!(during_cleanup, Err(Error::Busy { .. })));
        assert!(after_cleanup.unwrap().start);
    }

    #[tokio::test]
    async fn waiting_for_cleanup_keeps_other_experiments_busy() {
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
        assert!(matches!(unrelated.unwrap(), Err(Error::Busy { .. })));
        assert!(resumed.unwrap().start);
    }
}
