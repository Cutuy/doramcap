//! Compose manifest rewriting around shared filesystem view creation.
use crate::compose::{
    DockerAdapter, ResolvedComposeProject, Result,
    adapter::{digest, private_write},
    error::invalid,
    mounts,
};
use docker_engine::overlay::{OverlayPolicy, OverlaySet, PrepareInput, prepare, unmount};
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

/// Pinned Compose projects whose bind sources will use per-run views.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposePrepareInput {
    /// Existing fresh run directory outside all lower trees. Relative paths are accepted.
    pub artifacts: PathBuf,
    /// Fresh temporary view root with an existing canonical parent.
    pub workspace: PathBuf,
    /// Resolved and pinned manifests to rewrite without editing source files.
    pub projects: Vec<ResolvedComposeProject>,
    /// Filesystem disposition selected for this run.
    pub policy: OverlayPolicy,
}
/// Filesystem views and the Compose manifests that consume them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedOverlays {
    /// Shared per-run views, finalized only after writers stop.
    pub views: OverlaySet,
    /// Immutable execution manifests consuming these views.
    pub projects: Vec<ResolvedComposeProject>,
}
/// Prepare filesystem views and validate immutable execution manifests.
/// Errors unwind mounted views and retain any rollback failure in the diagnostic.
/// Cancellation may leave views in the workspace for later cleanup.
pub struct PrepareOverlays {
    pub adapter: Rc<DockerAdapter>,
}
impl Step for PrepareOverlays {
    const ID: Option<&'static str> = Some("compose.filesystem.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare filesystem");
    type Input = ComposePrepareInput;
    type Output = PreparedOverlays;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        self.prepare(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
impl PrepareOverlays {
    async fn prepare(&self, input: ComposePrepareInput) -> Result<PreparedOverlays> {
        let mut models = Vec::new();
        let mut bind_mounts = Vec::new();
        for project in &input.projects {
            let model = mounts::model(project, "overlay input manifest changed").await?;
            bind_mounts.extend(mounts::binds(&project.key, &model)?);
            models.push(model);
        }
        let views = prepare(PrepareInput {
            artifacts: input.artifacts,
            workspace: input.workspace,
            bind_mounts,
            policy: input.policy,
        })
        .await?;
        let mut projects = input.projects;
        let result = async {
            for (project, model) in projects.iter_mut().zip(&mut models) {
                for (service, value) in model["services"].as_object_mut().unwrap() {
                    for bind in value
                        .get_mut("volumes")
                        .and_then(serde_json::Value::as_array_mut)
                        .into_iter()
                        .flatten()
                        .filter(|v| v["type"] == "bind")
                    {
                        let consumer = format!("{}/{}", project.key, service);
                        let mapping = views
                            .mappings
                            .iter()
                            .find(|m| {
                                m.consumer == consumer
                                    && m.target == bind["target"].as_str().unwrap_or("")
                            })
                            .ok_or_else(|| invalid("prepared bind mapping missing"))?;
                        bind["source"] = serde_json::json!(mapping.effective);
                    }
                }
                let bytes = serde_json::to_vec_pretty(model)?;
                project.manifest = views
                    .directory
                    .with_file_name(format!("{}.execution.json", project.key))
                    .to_str()
                    .ok_or_else(|| invalid("manifest path is not UTF-8"))?
                    .into();
                project.sha256 = digest(&bytes);
                private_write(Path::new(&project.manifest), &bytes).await?;
                self.adapter.project(project).validate_execution().await?;
            }
            Ok::<(), crate::compose::Error>(())
        }
        .await;
        if let Err(error) = result {
            let mut errors = vec![error.to_string()];
            for tree in views.trees.iter().rev() {
                if let Err(error) = unmount(&tree.merged).await {
                    errors.push(format!("rollback {}: {error}", tree.merged.display()));
                }
            }
            return Err(invalid(errors.join("; ")));
        }
        Ok(PreparedOverlays { views, projects })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::adapter::tests::test_engine;
    use serde_json::json;

    fn project(artifacts: &Path, model: &serde_json::Value) -> ResolvedComposeProject {
        let bytes = serde_json::to_vec(model).unwrap();
        let manifest = artifacts.join("input.json");
        std::fs::write(&manifest, &bytes).unwrap();
        ResolvedComposeProject {
            key: "project".into(),
            name: "dc-project".into(),
            manifest: manifest.to_string_lossy().into(),
            sha256: digest(&bytes),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn execution_manifest_with_relative_artifacts_preserves_live_shm_and_source_manifest() {
        // Arrange
        let cwd = std::env::current_dir().unwrap();
        let root = tempfile::tempdir_in(&cwd).unwrap();
        let artifacts = root.path().join("artifacts");
        std::fs::create_dir(&artifacts).unwrap();
        let shared = tempfile::tempdir_in("/dev/shm").unwrap();
        let model = json!({"services":{"app":{"image":"unused", "volumes":[
            {"type":"bind","source":shared.path(),"target":"/input/shm"}
        ]}}});
        let original = project(&artifacts, &model);
        let original_bytes = std::fs::read(&original.manifest).unwrap();
        let (adapter, server) = test_engine(root.path(), vec![]);
        let mut step = PrepareOverlays {
            adapter: Rc::new(adapter),
        };

        // Act
        let prepared = step
            .run(ComposePrepareInput {
                artifacts: artifacts.strip_prefix(&cwd).unwrap().into(),
                workspace: root.path().join("views"),
                projects: vec![original.clone()],
                policy: OverlayPolicy::default(),
            })
            .await
            .unwrap();

        // Assert
        assert!(prepared.views.trees.is_empty());
        assert_eq!(prepared.views.mappings[0].consumer, "project/app");
        assert_eq!(std::fs::read(&original.manifest).unwrap(), original_bytes);
        let manifest = Path::new(&prepared.projects[0].manifest);
        assert_eq!(
            manifest.parent().unwrap(),
            artifacts.canonicalize().unwrap()
        );
        let execution: serde_json::Value =
            serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
        assert_eq!(
            execution["services"]["app"]["volumes"],
            model["services"]["app"]["volumes"]
        );
        assert_eq!(
            prepared.projects[0].sha256,
            digest(&std::fs::read(&prepared.projects[0].manifest).unwrap())
        );
        docker_engine::overlay::finalize(prepared.views)
            .await
            .unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires CAP_SYS_ADMIN, rsync and Docker Compose"]
    async fn failed_compose_validation_unmounts_prepared_views() {
        // Arrange: filesystem preparation succeeds before Compose rejects the service.
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("artifacts");
        let source = root.path().join("source");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("keep"), b"original").unwrap();
        let model = json!({"services":{"app":{"image":"unused", "unknown_option":true,
            "volumes":[{"type":"bind","source":source,"target":"/work"}]}}});
        let original = project(&artifacts, &model);
        let original_bytes = std::fs::read(&original.manifest).unwrap();
        let (adapter, server) = test_engine(root.path(), vec![]);
        let mut step = PrepareOverlays {
            adapter: Rc::new(adapter),
        };

        // Act
        let error = step
            .run(ComposePrepareInput {
                artifacts: artifacts.clone(),
                workspace: root.path().join("views"),
                projects: vec![original.clone()],
                policy: OverlayPolicy::default(),
            })
            .await
            .unwrap_err();

        // Assert: a mounted tree was prepared and then strictly unmounted.
        assert!(error.to_string().contains("unknown_option"), "{error}");
        let views: OverlaySet = serde_json::from_slice(
            &std::fs::read(artifacts.join("overlays/prepared.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(views.trees.len(), 1);
        let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
        assert!(!mounts.contains(&format!(" {} ", views.trees[0].merged.display())));
        assert_eq!(std::fs::read(source.join("keep")).unwrap(), b"original");
        assert_eq!(std::fs::read(original.manifest).unwrap(), original_bytes);
        server.await.unwrap();
    }
}
