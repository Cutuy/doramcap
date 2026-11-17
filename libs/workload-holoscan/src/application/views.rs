//! Controlled project/data views; arbitrary native mounts remain outside this adapter.
use crate::HoloscanPlan;
use host_filesystem::{
    filesystem_proto::{AppScope, DirectoryMount},
    overlay::{self, Disposition, OverlayPlan, OverlayPolicy},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use workload_lifecycle::launch::RunPlan;

// Native CLI5 get_volume_args maps the supplied basename beneath this destination.
const NATIVE_VOLUME_ROOT: &str = "/workspace/volumes";

/// Resolve the project and requested data folders without creating files or mounts.
/// Project roots retain the Build basename; data roots retain the native --add-volume basename.
/// Native project destinations remain unknown until the created container is inspected.
pub(super) async fn resolve(
    run: &RunPlan,
    plan: &HoloscanPlan,
    workspace: PathBuf,
) -> Result<OverlayPlan, String> {
    let mounts = controlled_mounts(run, plan)?;
    let mut views = overlay::resolve(overlay::PrepareInput {
        bind_mounts: mounts,
        artifacts: PathBuf::from(&run.artifacts),
        workspace,
        policy: OverlayPolicy {
            disposition: Disposition::from_policy(run.write_policy).map_err(error)?,
            passthrough_folders: run.passthrough_folders.clone(),
        },
    })
    .await
    .map_err(error)?;
    let mut names = BTreeMap::new();
    let project = tokio::fs::canonicalize(&run.repository_root)
        .await
        .map_err(error)?;
    if views.roots().contains(&project) {
        let name = project
            .file_name()
            .ok_or("project root requires a basename")?
            .to_os_string();
        views.name_view(&project, &name).map_err(error)?;
        names.insert(project, name);
    }
    for folder in &plan.extra_data_folders {
        let source = tokio::fs::canonicalize(folder).await.map_err(error)?;
        let supplied_name = Path::new(folder)
            .file_name()
            .ok_or("extra data folder requires a basename")?;
        if views
            .roots()
            .iter()
            .any(|root| source.starts_with(root) && &source != root)
            && source.file_name() != Some(supplied_name)
        {
            return Err("nested extra data alias cannot preserve its native basename in a shared project view".into());
        }
        if views.roots().contains(&source) {
            let name = Path::new(folder)
                .file_name()
                .ok_or("extra data folder requires a basename")?
                .to_os_string();
            if names.get(&source).is_some_and(|previous| previous != &name) {
                return Err(
                    "project/data aliases of one overlay root require the same native basename"
                        .into(),
                );
            }
            views.name_view(&source, &name).map_err(error)?;
            names.insert(source, name);
        }
    }
    Ok(views)
}

/// Only sources explicitly controlled by Console; the empty project destination is unknown.
/// Source identities are resolved by the shared overlay implementation.
pub(super) fn controlled_mounts(
    run: &RunPlan,
    plan: &HoloscanPlan,
) -> Result<Vec<DirectoryMount>, String> {
    let application = &plan
        .selection
        .as_ref()
        .ok_or("Holoscan selection is required")?
        .application;
    let mut mounts = vec![DirectoryMount {
        consumer: consumer(application),
        source: run.repository_root.clone(),
        target: String::new(),
        read_only: false,
        app_scope: AppScope::User as i32,
    }];
    let mut targets = BTreeSet::new();
    for folder in &plan.extra_data_folders {
        let basename = Path::new(folder)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("extra data folder requires a UTF-8 basename")?;
        let target = format!("{NATIVE_VOLUME_ROOT}/{basename}");
        if !targets.insert(target.clone()) {
            return Err("extra data folders share one native volume destination".into());
        }
        mounts.push(DirectoryMount {
            consumer: consumer(application),
            source: folder.clone(),
            target,
            read_only: false,
            app_scope: AppScope::User as i32,
        });
    }
    Ok(mounts)
}

/// Select only this backend's mappings, in project/data order, from a possible combined set.
pub(super) fn mappings<'a>(
    set: &'a host_filesystem::overlay::OverlaySet,
    run: &RunPlan,
    plan: &HoloscanPlan,
) -> Result<Vec<&'a host_filesystem::overlay::MountMapping>, String> {
    controlled_mounts(run, plan)?
        .into_iter()
        .map(|requested| {
            let mut matches = set.mappings.iter().filter(|mapping| {
                mapping.consumer == requested.consumer && mapping.target == requested.target
            });
            let mapping = matches
                .next()
                .ok_or("controlled Holoscan view mapping is unavailable")?;
            if matches.next().is_some() {
                return Err("controlled Holoscan consumer/destination mapping is ambiguous".into());
            }
            Ok(mapping)
        })
        .collect()
}

pub(super) fn consumer(application: &str) -> String {
    format!("holoscan:{application}")
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Resolve and prepare controlled views on a retained task; callers settle it after cancellation.
pub(super) struct PrepareViews {
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}
impl orchestrator::Step for PrepareViews {
    const ID: Option<&'static str> = Some("holoscan.prepare-views");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare Holoscan filesystem views");
    type Input = (RunPlan, HoloscanPlan, PathBuf, crate::ConfigurationSnapshot);
    type Output = host_filesystem::overlay::OverlaySet;
    async fn run(
        &mut self,
        (run, plan, workspace, snapshot): Self::Input,
    ) -> orchestrator::Result<Self::Output> {
        let resolved = resolve(&run, &plan, workspace)
            .await
            .map_err(orchestrator::Error::message)?;
        let mappings = resolved.mappings();
        if mappings
            .first()
            .is_none_or(|mapping| mapping.original != Path::new(&snapshot.repository_root))
            || snapshot.resolved_extra_data_folders.len() != plan.extra_data_folders.len()
            || mappings[1..]
                .iter()
                .zip(&snapshot.resolved_extra_data_folders)
                .any(|(mapping, expected)| mapping.original != Path::new(expected))
        {
            return Err(orchestrator::Error::message("ConfigurationChanged"));
        }
        overlay::prepare_with_cancellation(resolved, || {
            *self.shutdown.borrow() || self.shutdown.has_changed().is_err()
        })
        .await
        .map_err(|error| orchestrator::Error::message(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selection;

    #[tokio::test]
    async fn resolving_views_retains_canonical_roots_without_creating_workspace() {
        use std::sync::atomic::{AtomicU64, Ordering};
        // Arrange
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "holoscan-views-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let project = directory.join("project");
        let data = directory.join("named-data");
        let artifacts = directory.join("artifacts");
        let workspace = directory.join("overlays");
        for path in [&project, &data, &artifacts] {
            std::fs::create_dir_all(path).unwrap();
        }
        let run = RunPlan {
            repository_root: project.to_str().unwrap().into(),
            artifacts: artifacts.to_str().unwrap().into(),
            ..Default::default()
        };
        let plan = HoloscanPlan {
            selection: Some(Selection {
                application: "sample".into(),
                ..Default::default()
            }),
            extra_data_folders: vec![data.to_str().unwrap().into()],
            ..Default::default()
        };
        // Act
        let views = resolve(&run, &plan, workspace.clone()).await.unwrap();
        // Assert
        assert_eq!(views.roots().len(), 2);
        assert!(views.roots().contains(&project));
        assert!(views.roots().contains(&data));
        assert!(!workspace.exists());
        assert!(std::fs::read_dir(&artifacts).unwrap().next().is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn controlled_mounts_preserve_supplied_extra_volume_names_and_user_scope() {
        // Arrange
        let run = RunPlan {
            repository_root: "/project".into(),
            ..Default::default()
        };
        let plan = HoloscanPlan {
            selection: Some(Selection {
                application: "sample".into(),
                ..Default::default()
            }),
            extra_data_folders: vec!["/data/source alias".into()],
            ..Default::default()
        };
        // Act
        let mounts = controlled_mounts(&run, &plan).unwrap();
        // Assert
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].source, "/project");
        assert!(mounts[0].target.is_empty());
        assert_eq!(mounts[1].source, "/data/source alias");
        assert_eq!(mounts[1].target, "/workspace/volumes/source alias");
        assert!(
            mounts
                .iter()
                .all(|mount| mount.consumer == "holoscan:sample"
                    && !mount.read_only
                    && mount.app_scope == AppScope::User as i32)
        );
    }
}
