//! Compose manifest rewriting around shared filesystem view creation.
use crate::compose::{
    DockerAdapter, ResolvedComposeProject, Result,
    adapter::{digest, private_write},
    error::invalid,
    mounts,
};
use host_filesystem::overlay::{
    OverlayPlan, OverlayPolicy, OverlaySet, PrepareInput, prepare, resolve, unmount,
};
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
/// Pinned Compose models and native bind facts for one shared filesystem plan.
/// Observation owns no views; the workflow owns preparation, cancellation and finalization.
pub struct ComposeFilesystem {
    projects: Vec<ResolvedComposeProject>,
    models: Vec<serde_json::Value>,
    binds: Vec<docker_engine::docker_proto::BindMount>,
}
impl ComposeFilesystem {
    /// Read each pinned manifest once, retaining Docker bind options and rejecting changed input.
    /// Creates no filesystem views or Engine resources.
    pub async fn observe(projects: Vec<ResolvedComposeProject>) -> Result<Self> {
        let mut models = Vec::new();
        let mut binds = Vec::new();
        for project in &projects {
            let model = mounts::model(project, "overlay input manifest changed").await?;
            binds.extend(mounts::binds(&project.key, &model)?);
            models.push(model);
        }
        Ok(Self {
            projects,
            models,
            binds,
        })
    }

    /// Directory mounts to combine with other backend inputs before resolving one shared OverlayPlan.
    /// Consumer identities retain the existing project-key/service format.
    pub fn directory_mounts(&self) -> Vec<host_filesystem::filesystem_proto::DirectoryMount> {
        mounts::directories(&self.binds)
    }

    /// Validate each Compose alias in a combined plan before filesystem preparation.
    /// Missing/ambiguous consumer+target mappings and unsupported isolated bind options fail.
    pub fn validate_views(&self, views: &OverlayPlan) -> Result<()> {
        mounts::validate_views(&self.binds, views)
    }

    /// Write and natively validate execution manifests against the caller's already-prepared views.
    /// Validate the shared plan before preparing it; this function never mounts or finalizes views.
    /// Errors/cancellation retain caller ownership of views and any execution files already written.
    pub async fn rewrite(
        mut self,
        adapter: &DockerAdapter,
        views: &OverlaySet,
    ) -> Result<Vec<ResolvedComposeProject>> {
        for (project, model) in self.projects.iter_mut().zip(&mut self.models) {
            for (service, value) in model["services"].as_object_mut().unwrap() {
                for bind in value
                    .get_mut("volumes")
                    .and_then(serde_json::Value::as_array_mut)
                    .into_iter()
                    .flatten()
                    .filter(|v| v["type"] == "bind")
                {
                    let consumer = format!("{}/{}", project.key, service);
                    let mapping = mounts::view_mapping(
                        &views.mappings,
                        &consumer,
                        bind["target"].as_str().unwrap_or(""),
                    )?;
                    bind["source"] = serde_json::json!(mapping.effective);
                }
            }
            let bytes = serde_json::to_vec_pretty(model)?;
            project.manifest = views
                .directory
                .with_file_name(super::adapter::execution_record(&project.key))
                .to_str()
                .ok_or_else(|| invalid("manifest path is not UTF-8"))?
                .into();
            project.sha256 = digest(&bytes);
            private_write(Path::new(&project.manifest), &bytes).await?;
            adapter.project(project).validate_execution().await?;
        }
        Ok(self.projects)
    }
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
        let observed = ComposeFilesystem::observe(input.projects).await?;
        let views = resolve(PrepareInput {
            artifacts: input.artifacts,
            workspace: input.workspace,
            bind_mounts: observed.directory_mounts(),
            policy: input.policy,
        })
        .await?;
        observed.validate_views(&views)?;
        let views = prepare(views).await?;
        match observed.rewrite(&self.adapter, &views).await {
            Ok(projects) => Ok(PreparedOverlays { views, projects }),
            Err(error) => {
                let mut errors = vec![error.to_string()];
                for tree in views.trees.iter().rev() {
                    if let Err(error) = unmount(&tree.merged).await {
                        errors.push(format!("rollback {}: {error}", tree.merged.display()));
                    }
                }
                Err(invalid(errors.join("; ")))
            }
        }
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
        host_filesystem::overlay::finalize(prepared.views)
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

    #[tokio::test]
    async fn combined_plan_checks_compose_alias_options_by_identity_before_preparation() {
        use host_filesystem::filesystem_proto::DirectoryMount;
        // Arrange: an unrelated external mount precedes a Compose read-only alias.
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("artifacts");
        let source = root.path().join("source");
        let child = source.join("child");
        let unrelated = root.path().join("unrelated");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir(&unrelated).unwrap();
        for bind in [json!({"propagation":"rshared"}), json!({"selinux":"z"})] {
            let model = json!({"services":{"reader":{"image":"unused","volumes":[
                {"type":"bind","source":source,"target":"/read","read_only":true,"bind":bind}
            ]}}});
            let observed = ComposeFilesystem::observe(vec![project(&artifacts, &model)])
                .await
                .unwrap();
            let mut mounts = vec![DirectoryMount {
                consumer: "holoscan:app".into(),
                source: unrelated.to_str().unwrap().into(),
                target: "/unrelated".into(),
                read_only: true,
                ..Default::default()
            }];
            mounts.extend(observed.directory_mounts());
            mounts.push(DirectoryMount {
                consumer: "holoscan:app".into(),
                source: child.to_str().unwrap().into(),
                target: "".into(),
                ..Default::default()
            });
            let workspace = root.path().join("views");
            let plan = resolve(PrepareInput {
                artifacts: artifacts.clone(),
                workspace: workspace.clone(),
                bind_mounts: mounts,
                policy: OverlayPolicy::default(),
            })
            .await
            .unwrap();
            // Act / Assert: the read-only alias shares the external writer's isolated source tree.
            let error = observed.validate_views(&plan).unwrap_err();
            assert!(error.to_string().contains("project/reader"), "{error}");
            assert!(
                error
                    .to_string()
                    .contains("unsupported OverlayFS bind options"),
                "{error}"
            );
            assert!(!workspace.exists());
            assert!(!artifacts.join("overlays").exists());
        }
    }

    #[tokio::test]
    async fn missing_and_ambiguous_combined_bind_identities_fail_before_preparation() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("artifacts");
        let source = root.path().join("source");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir(&source).unwrap();
        let observed = ComposeFilesystem::observe(vec![project(
            &artifacts,
            &json!({"services":{"app":{"image":"unused","volumes":[
                {"type":"bind","source":source,"target":"/work","read_only":true}
            ]}}}),
        )])
        .await
        .unwrap();
        for (mounts, expected) in [
            (vec![], "missing"),
            (
                [observed.directory_mounts(), observed.directory_mounts()].concat(),
                "ambiguous",
            ),
        ] {
            let plan = resolve(PrepareInput {
                artifacts: artifacts.clone(),
                workspace: root.path().join("views"),
                bind_mounts: mounts,
                policy: OverlayPolicy::default(),
            })
            .await
            .unwrap();
            // Act / Assert
            let error = observed.validate_views(&plan).unwrap_err();
            assert!(error.to_string().contains("project/app:/work"), "{error}");
            assert!(error.to_string().contains(expected), "{error}");
            assert!(!artifacts.join("overlays").exists());
        }
    }

    #[tokio::test]
    #[ignore = "requires CAP_SYS_ADMIN, rsync and Docker Compose"]
    async fn shared_prepared_views_are_rewritten_once_and_remain_caller_owned_on_error() {
        use host_filesystem::{filesystem_proto::DirectoryMount, overlay::Disposition};
        // Arrange: native project and Compose child share one writable tree.
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("artifacts");
        let source = root.path().join("source");
        let child = source.join("child");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(child.join("keep"), b"original").unwrap();
        let model = json!({"services":{"app":{"image":"unused","volumes":[
            {"type":"bind","source":child,"target":"/work"}
        ]}}});
        let original = project(&artifacts, &model);
        let original_bytes = std::fs::read(&original.manifest).unwrap();
        let observed = ComposeFilesystem::observe(vec![original.clone()])
            .await
            .unwrap();
        let mut mounts = vec![DirectoryMount {
            consumer: "holoscan:app".into(),
            source: source.to_str().unwrap().into(),
            target: "".into(),
            ..Default::default()
        }];
        mounts.extend(observed.directory_mounts());
        let plan = resolve(PrepareInput {
            artifacts: artifacts.clone(),
            workspace: root.path().join("views"),
            bind_mounts: mounts,
            policy: OverlayPolicy {
                disposition: Disposition::Discard,
                ..Default::default()
            },
        })
        .await
        .unwrap();
        observed.validate_views(&plan).unwrap();
        let views = prepare(plan).await.unwrap();
        assert_eq!(views.trees.len(), 1);
        let effective = views.trees[0].merged.join("child");
        let (adapter, server) = test_engine(root.path(), vec![]);
        // Act
        let rewritten = observed.rewrite(&adapter, &views).await.unwrap();
        // Assert
        let output: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&rewritten[0].manifest).unwrap()).unwrap();
        assert_eq!(
            output["services"]["app"]["volumes"][0]["source"],
            json!(effective)
        );
        assert_eq!(std::fs::read(&original.manifest).unwrap(), original_bytes);
        assert_eq!(views.mappings[0].consumer, "holoscan:app");
        std::fs::write(effective.join("keep"), b"view only").unwrap();
        assert_eq!(std::fs::read(child.join("keep")).unwrap(), b"original");
        // Act: repeated execution-file creation fails while views remain owned by the caller.
        let observed = ComposeFilesystem::observe(vec![original]).await.unwrap();
        assert!(observed.rewrite(&adapter, &views).await.is_err());
        // Assert
        let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
        assert!(mounts.contains(&format!(" {} ", views.trees[0].merged.display())));
        host_filesystem::overlay::finalize(views).await.unwrap();
        assert_eq!(std::fs::read(child.join("keep")).unwrap(), b"original");
        server.await.unwrap();
    }
}
