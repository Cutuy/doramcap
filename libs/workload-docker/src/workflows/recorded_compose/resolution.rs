//! Resolve Compose projects, determine launch order and capture configuration snapshots.
//! Record the resolved settings so callers can detect changes before launch.
//! Generated recording paths compare relative to each run's artifact directory.
use super::{ComposeRun, preparation, projects::LaunchInput};
use crate::{
    compose::{adapter::digest, error::invalid, *},
    workflows::{
        dora,
        system_apps::{PrepareInput, PrepareSystemApps, PreparedSystemApps},
    },
};
use futures_util::future::join_all;
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    rc::Rc,
};

/// Snapshot of settings, Compose manifests, service policies, launch order and recording paths.
/// Compare snapshots from separate resolutions to detect changed inputs before launch.
/// Generated system-app bind paths and recording paths use artifact-relative locations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationSnapshot {
    settings: String,
    groups: Vec<SnapshotGroup>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SnapshotGroup {
    projects: Vec<SnapshotProject>,
    recordings: Vec<(PathBuf, PathBuf)>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SnapshotProject {
    key: String,
    manifest_sha256: String,
    services: BTreeMap<String, ComposeService>,
    remove_volumes: bool,
}
/// Projects in launch order, with a snapshot for comparison before launch.
#[derive(Clone, Debug)]
pub struct ResolvedInputs {
    pub groups: Vec<LaunchInput>,
    pub snapshot: ConfigurationSnapshot,
}

/// Resolve app and workload manifests in the existing `spec.artifacts` directory.
/// Returns launch groups and a configuration snapshot without creating mounts or containers.
/// Callers can compare snapshots to reject changed inputs before execution.
pub async fn resolve(adapter: Rc<DockerAdapter>, spec: &ComposeRun) -> Result<ResolvedInputs> {
    preparation::validate(spec)?;
    let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
    let system = PrepareSystemApps
        .run(PrepareInput {
            config: spec.system_apps.clone(),
            artifacts: artifacts.clone(),
        })
        .await
        .map_err(|e| invalid(e.to_string()))?;
    let (_, inputs) = resolve_with(
        spec,
        &artifacts,
        &system,
        |input| adapter.resolve(input),
        |plan| plan,
    )
    .await?;
    Ok(inputs)
}

// The callback allows workflow steps to track dependencies and direct callers to use the adapter.
pub(super) async fn resolve_with<P, F, Fut>(
    spec: &ComposeRun,
    artifacts: &Path,
    system: &PreparedSystemApps,
    resolve: F,
    value: fn(&P) -> &ResolvedComposeProject,
) -> Result<(Vec<P>, ResolvedInputs)>
where
    F: Fn(ResolveInput) -> Fut,
    Fut: Future<Output = Result<P>>,
{
    let system_count = system.projects.len();
    let projects: Vec<_> = system
        .projects
        .iter()
        .cloned()
        .chain(spec.first_party.clone())
        .chain(spec.projects.clone())
        .collect();
    let mut keys = HashSet::new();
    if projects.iter().any(|p| !keys.insert(p.key.clone())) {
        return Err(invalid("system and workload project keys must be unique"));
    }
    let results = join_all(projects.into_iter().map(|project| {
        resolve(ResolveInput {
            run: spec.id.clone(),
            artifacts: artifacts.to_owned(),
            project,
        })
    }))
    .await;
    // Every independent resolution completes before propagating the first error.
    // Report an empty service selection before image errors so the caller can request profiles.
    if results
        .iter()
        .any(|result| matches!(result, Err(Error::EmptySelection)))
    {
        return Err(Error::EmptySelection);
    }
    let plans: Vec<P> = results.into_iter().collect::<Result<_>>()?;
    let projects: Vec<_> = plans.iter().map(|p| value(p).clone()).collect();
    dora::bind(&spec.system_apps.dora_flows, &projects)?;
    let group = |projects, recordings| LaunchInput {
        projects,
        recordings,
        readiness_seconds: spec.readiness_seconds,
    };
    let mut groups = Vec::new();
    if system_count != 0 {
        groups.push(group(
            projects[..system_count].to_vec(),
            system.recordings.clone(),
        ));
    }
    for plan in &projects[system_count..system_count + spec.first_party.len()] {
        groups.push(group(vec![plan.clone()], vec![]));
    }
    groups.push(group(
        projects[system_count + spec.first_party.len()..].to_vec(),
        vec![],
    ));
    let system_keys: HashSet<_> = system.projects.iter().map(|p| p.key.as_str()).collect();
    let snapshot = snapshot(spec, artifacts, &groups, &system_keys).await?;
    Ok((plans, ResolvedInputs { groups, snapshot }))
}

async fn snapshot(
    spec: &ComposeRun,
    artifacts: &Path,
    groups: &[LaunchInput],
    system_keys: &HashSet<&str>,
) -> Result<ConfigurationSnapshot> {
    let settings = digest(&serde_json::to_vec(&serde_json::json!({
        "system_apps": spec.system_apps, "filesystem": spec.filesystem,
        "readiness_seconds": spec.readiness_seconds, "stop_seconds": spec.stop_seconds,
    }))?);
    let mut captured = Vec::new();
    for group in groups {
        let mut projects = Vec::new();
        for plan in &group.projects {
            let bytes = tokio::fs::read(&plan.manifest).await?;
            if digest(&bytes) != plan.sha256 {
                return Err(invalid(
                    "frozen Compose manifest changed while capturing configuration",
                ));
            }
            let mut model: serde_json::Value = serde_json::from_slice(&bytes)?;
            if system_keys.contains(plan.key.as_str()) {
                relocate_artifact_binds(&mut model, artifacts);
            }
            projects.push(SnapshotProject {
                key: plan.key.clone(),
                manifest_sha256: digest(&serde_json::to_vec(&model)?),
                services: plan.services.clone(),
                remove_volumes: plan.remove_volumes,
            });
        }
        let relative = |path: &Path| {
            path.strip_prefix(artifacts)
                .map(Path::to_owned)
                .map_err(|_| invalid("recording output is outside run artifacts"))
        };
        let recordings = group
            .recordings
            .iter()
            .map(|r| Ok((relative(&r.stream)?, relative(&r.recording)?)))
            .collect::<Result<_>>()?;
        captured.push(SnapshotGroup {
            projects,
            recordings,
        });
    }
    Ok(ConfigurationSnapshot {
        settings,
        groups: captured,
    })
}

// Compare generated system-app binds under the artifact directory by relative path.
// Preserve commands, environment values and binds outside that directory exactly.
fn relocate_artifact_binds(model: &mut serde_json::Value, artifacts: &Path) {
    if let Some(services) = model
        .get_mut("services")
        .and_then(serde_json::Value::as_object_mut)
    {
        for service in services.values_mut() {
            if let Some(volumes) = service
                .get_mut("volumes")
                .and_then(serde_json::Value::as_array_mut)
            {
                for volume in volumes {
                    if volume["type"] == "bind"
                        && let Some(source) = volume["source"].as_str()
                        && let Ok(relative) = Path::new(source).strip_prefix(artifacts)
                    {
                        volume["source"] = serde_json::json!({"run_artifact_relative": relative});
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::system_apps::{CpuMonitor, SystemAppsConfig};
    use std::{cell::Cell, collections::BTreeMap};

    fn project(root: &Path, key: &str) -> ComposeProject {
        ComposeProject {
            key: key.into(),
            directory: root.to_owned(),
            files: vec!["compose.yaml".into()],
            env_files: vec![],
            environment: BTreeMap::new(),
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: false,
        }
    }
    fn spec(root: &Path) -> ComposeRun {
        ComposeRun {
            filesystem: Some(Default::default()),
            id: "initial".into(),
            artifacts: root.to_owned(),
            recovery_anchor_dir: None,
            projects: vec![project(root, "workload")],
            first_party: vec![project(root, "helper")],
            system_apps: SystemAppsConfig {
                cpu_monitor: Some(CpuMonitor {
                    image: "monitor:local".into(),
                }),
                ..Default::default()
            },
            readiness_seconds: 60,
            stop_seconds: 15,
        }
    }
    fn system(root: &Path) -> PreparedSystemApps {
        PreparedSystemApps {
            projects: vec![project(root, "system-a"), project(root, "system-b")],
            recordings: vec![crate::workflows::system_apps::Recording {
                stream: root.join("cpu.pb"),
                recording: root.join("cpu.mcap"),
            }],
        }
    }
    async fn resolved(
        root: &Path,
        input: ResolveInput,
        command: &str,
        image: &str,
    ) -> Result<ResolvedComposeProject> {
        let model = serde_json::json!({"services":{"app":{
            "image": image, "command": command,
            "environment": {"MEANINGFUL":"value"},
            "volumes":[{"type":"bind","source":root.to_string_lossy(),"target":"/recordings"}],
        }}});
        let bytes = serde_json::to_vec_pretty(&model)?;
        let manifest = root.join(format!("{}.compose.json", input.project.key));
        tokio::fs::write(&manifest, &bytes).await?;
        Ok(ResolvedComposeProject {
            run: input.run,
            key: input.project.key,
            name: "owned".into(),
            manifest,
            sha256: digest(&bytes),
            services: BTreeMap::from([(
                "app".into(),
                ComposeService {
                    image_id: image.into(),
                    replicas: 1,
                    completed: false,
                },
            )]),
            diagnostics: vec![],
            remove_volumes: input.project.remove_volumes,
        })
    }
    async fn inputs(root: &Path, command: &str, image: &str) -> ResolvedInputs {
        let spec = spec(root);
        resolve_with(
            &spec,
            root,
            &system(root),
            |input| async move {
                let is_second = input.project.key == "system-b";
                // Only system app binds relocate. Workload/first-party fixtures use stable host binds.
                let plan = resolved(
                    root,
                    input,
                    if is_second { command } else { "unchanged" },
                    if is_second { image } else { "image:fixed" },
                )
                .await?;
                if plan.key == "workload" || plan.key == "helper" {
                    let mut model: serde_json::Value =
                        serde_json::from_slice(&tokio::fs::read(&plan.manifest).await?)?;
                    model["services"]["app"]["volumes"][0]["source"] =
                        serde_json::json!("/stable/workload");
                    let bytes = serde_json::to_vec_pretty(&model)?;
                    tokio::fs::write(&plan.manifest, &bytes).await?;
                    return Ok(ResolvedComposeProject {
                        sha256: digest(&bytes),
                        ..plan
                    });
                }
                Ok(plan)
            },
            |plan| plan,
        )
        .await
        .unwrap()
        .1
    }

    #[tokio::test]
    async fn snapshot_covers_all_projects_and_ignores_artifact_directory_changes() {
        // Arrange
        let initial = tempfile::tempdir().unwrap();
        let launch = tempfile::tempdir().unwrap();
        // Act
        let captured = inputs(initial.path(), "original", "image:original").await;
        let fresh = inputs(launch.path(), "original", "image:original").await;
        // Assert
        assert_eq!(captured.snapshot, fresh.snapshot);
        assert_eq!(
            fresh
                .groups
                .iter()
                .map(|g| g
                    .projects
                    .iter()
                    .map(|p| p.key.as_str())
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [
                vec!["system-a", "system-b"],
                vec!["helper"],
                vec!["workload"]
            ]
        );
        // Act / Assert: changing another app's command or image changes the snapshot.
        let changed = inputs(launch.path(), "changed", "image:original").await;
        assert_ne!(captured.snapshot, changed.snapshot);
        let retagged = inputs(launch.path(), "original", "image:new").await;
        assert_ne!(captured.snapshot, retagged.snapshot);
    }

    #[tokio::test]
    async fn snapshot_detects_changed_binds_commands_and_group_order() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let spec = spec(root.path());
        let prepared = inputs(root.path(), "original", "image:original").await;
        let keys = HashSet::from(["system-a", "system-b"]);
        let baseline = prepared.snapshot;
        let mut groups = prepared.groups;
        // Act: explicit completion policy is runtime input even with an unchanged manifest.
        let mut changed_policy = groups.clone();
        changed_policy[0].projects[0]
            .services
            .get_mut("app")
            .unwrap()
            .completed = true;
        // Assert
        assert_ne!(
            baseline,
            snapshot(&spec, root.path(), &changed_policy, &keys)
                .await
                .unwrap()
        );
        // Act: reordering launch groups changes execution semantics.
        groups.swap(0, 1);
        // Assert
        assert_ne!(
            baseline,
            snapshot(&spec, root.path(), &groups, &keys).await.unwrap()
        );
        groups.swap(0, 1);
        // Act: foreign binds remain exact even when they point inside run artifacts.
        let plan = &mut groups.last_mut().unwrap().projects[0];
        let mut model: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(&plan.manifest).await.unwrap()).unwrap();
        model["services"]["app"]["volumes"][0]["source"] = serde_json::json!(root.path());
        let bytes = serde_json::to_vec(&model).unwrap();
        tokio::fs::write(&plan.manifest, &bytes).await.unwrap();
        plan.sha256 = digest(&bytes);
        // Assert
        assert_ne!(
            baseline,
            snapshot(&spec, root.path(), &groups, &keys).await.unwrap()
        );
        // Act / Assert: tampering with a frozen manifest is an explicit failure.
        tokio::fs::write(&groups[0].projects[0].manifest, b"{}")
            .await
            .unwrap();
        assert!(snapshot(&spec, root.path(), &groups, &keys).await.is_err());
    }

    #[tokio::test]
    async fn independent_resolvers_finish_before_empty_selection_is_returned() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let spec = spec(root.path());
        let finished = Cell::new(0);
        // Act
        let result = resolve_with::<ResolvedComposeProject, _, _>(
            &spec,
            root.path(),
            &system(root.path()),
            |input| {
                let finished = &finished;
                async move {
                    tokio::task::yield_now().await;
                    finished.set(finished.get() + 1);
                    if input.project.key == "workload" {
                        Err(Error::EmptySelection)
                    } else {
                        Err(invalid("missing app image"))
                    }
                }
            },
            |plan| plan,
        )
        .await;
        // Assert
        assert!(matches!(result, Err(Error::EmptySelection)));
        assert_eq!(finished.get(), 4);
    }
}
