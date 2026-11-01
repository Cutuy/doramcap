//! Resolve app inputs natively, then freeze one experiment and its startup stages.
mod model;
mod snapshot;

use super::{ComposeRun, observation};
use crate::compose::{
    adapter::{digest, private_write},
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    overlay::OverlaySet,
    *,
};
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedRun {
    pub artifacts: PathBuf,
    pub plan: ResolvedComposeProject,
    pub system_services: Vec<String>,
    pub user_services: Vec<String>,
    pub baseline: FilesystemBaseline,
    pub dora: super::super::dora::DoraRuntime,
    pub snapshot: ConfigurationSnapshot,
}
impl PreparedRun {
    pub fn execution_plan(&self, overlays: Option<&OverlaySet>) -> ResolvedComposeProject {
        overlays.map_or_else(|| self.plan.clone(), |o| o.projects[0].clone())
    }
}
pub struct PrepareRun {
    pub executor: Weak<LocalExecutor>,
    pub adapter: Rc<DockerAdapter>,
}
impl Step for PrepareRun {
    const ID: Option<&'static str> = Some("compose.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare project");
    type Input = ComposeRun;
    type Output = PreparedRun;
    async fn run(&mut self, spec: ComposeRun) -> orchestrator::Result<PreparedRun> {
        self.prepare(spec)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
/// Expand native Dora modules relative to their source file for configuration assembly.
/// Completed graph validation remains part of experiment resolution.
pub async fn expand_graph(path: &Path) -> Result<Vec<u8>> {
    use dora_core::descriptor::{Descriptor, DescriptorExt};
    let bytes = crate::evidence::read_regular(path, 2 * 1024 * 1024).await?;
    let graph = Descriptor::parse(bytes)
        .map_err(|error| invalid(error.to_string()))?
        .expand(
            path.parent()
                .ok_or_else(|| invalid("graph parent missing"))?,
        )
        .map_err(|error| invalid(error.to_string()))?;
    Ok(serde_json::to_vec(&graph)?)
}

/// List profiles using the same repository-scoped Compose identity as execution.
pub async fn profiles(
    adapter: &DockerAdapter,
    run: &str,
    repository: &Path,
    project: &ComposeProject,
) -> Result<Vec<String>> {
    adapter
        .configuration()
        .profiles(
            run,
            project,
            &crate::compose::adapter::project_name("experiment", repository),
        )
        .await
}

/// Frozen experiment inputs; resolution creates private files without starting resources.
#[derive(Clone, Debug)]
pub struct ResolvedRun {
    pub artifacts: PathBuf,
    pub plan: ResolvedComposeProject,
    pub system_services: Vec<String>,
    pub user_services: Vec<String>,
    pub dora: super::super::dora::DoraRuntime,
    pub snapshot: ConfigurationSnapshot,
}

/// Resolve inputs using the execution path, without reset, reservation, mounts or containers.
/// `spec.artifacts` must name a new private directory beneath an existing parent.
/// Callers compare the snapshot with fresh preparation in the runner's preflight callback.
pub async fn resolve(adapter: &DockerAdapter, spec: ComposeRun) -> Result<ResolvedRun> {
    // Resolve runtime settings and validate the request.
    let mut runtime = spec.dora.clone();
    super::super::shared_dora::resolve(&mut runtime, &spec.artifacts)?;
    model::validate(&spec)?;
    crate::evidence::create_private_dir(&spec.artifacts).await?;
    let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
    // Freeze Dora inputs and merge apps while recording their startup stages.
    let dora = super::super::shared_dora::prepare(&runtime, &artifacts).await?;
    let mut model = json!({"services":{}});
    let mut system_services = Vec::new();
    let mut user_services = Vec::new();
    let mut diagnostics = Vec::new();
    for (apps, services) in [
        (&spec.system_apps, &mut system_services),
        (&spec.projects, &mut user_services),
    ] {
        for app in apps {
            let normalized = adapter
                .configuration()
                .normalize(
                    &spec.id,
                    &model::app_input(app, &dora)?,
                    &crate::compose::adapter::project_name("experiment", &spec.repository_root),
                )
                .await?;
            let warnings =
                model::merge_app(&mut model, normalized.model, app.remove_volumes, services)?;
            private_write(
                    &artifacts.join(format!("{}.inputs.json", app.key)),
                    &serde_json::to_vec(
                        &json!({"sources":normalized.input_hashes,"diagnostics":normalized.stderr,"warnings":warnings}),
                    )?,
                )
                .await?;
            diagnostics.extend(warnings);
        }
    }
    // Configure shared IPC, then validate graph bindings and startup dependencies.
    diagnostics.extend(model::configure_dora(
        &mut model,
        &dora,
        super::super::shared_dora::owner(&spec.repository_root),
    )?);
    if !dora.graph.is_empty() {
        let graph = super::super::shared_dora::graph(
            &dora,
            &crate::evidence::read_regular(Path::new(&dora.graph), 2 * 1024 * 1024).await?,
        )?;
        model::validate_graph(&model, &graph)?;
    }
    model::validate_dependencies(&model, &system_services, &user_services)?;
    // Snapshot file inputs and resolve one project with pinned images.
    freeze_files(&mut model, &artifacts).await?;
    let manifest = artifacts.join("experiment.input.json");
    private_write(&manifest, &serde_json::to_vec(&model)?).await?;
    let apps = spec.system_apps.iter().chain(&spec.projects);
    let mut plan = adapter
        .configuration()
        .resolve(ResolveInput {
            repository_root: Some(
                spec.repository_root
                    .to_str()
                    .ok_or_else(|| invalid("repository path is not UTF-8"))?
                    .into(),
            ),
            run: spec.id.clone(),
            artifacts: artifacts
                .to_str()
                .ok_or_else(|| invalid("artifact path is not UTF-8"))?
                .into(),
            project: Some(ComposeProject {
                key: "experiment".into(),
                directory: spec
                    .repository_root
                    .to_str()
                    .ok_or_else(|| invalid("repository path is not UTF-8"))?
                    .into(),
                files: vec![
                    manifest
                        .to_str()
                        .ok_or_else(|| invalid("manifest path is not UTF-8"))?
                        .into(),
                ],
                completed_services: apps
                    .clone()
                    .flat_map(|p| p.completed_services.clone())
                    .collect(),
                remove_volumes: apps.clone().all(|p| p.remove_volumes),
                ..Default::default()
            }),
        })
        .await?;
    for warning in &diagnostics {
        eprintln!("warning: {warning}");
    }
    plan.diagnostics.extend(diagnostics);
    let snapshot = snapshot::capture(
        &spec,
        &artifacts,
        &plan,
        &system_services,
        &user_services,
        &dora,
    )
    .await?;
    Ok(ResolvedRun {
        artifacts,
        plan,
        system_services,
        user_services,
        dora,
        snapshot,
    })
}

impl PrepareRun {
    async fn prepare(&self, spec: ComposeRun) -> Result<PreparedRun> {
        let resolved = resolve(&self.adapter, spec).await?;
        // Runtime endpoint availability is checked after execution resets the experiment.
        super::super::shared_dora::validate_endpoints(&resolved.dora)?;
        observation::snapshot(&resolved.artifacts, "before").await?;
        // Capture the filesystem baseline only for execution.
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| invalid("executor dropped"))?;
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(BaselineInput {
                projects: vec![resolved.plan.clone()],
                artifacts: resolved.artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?
            .into_value();
        Ok(PreparedRun {
            artifacts: resolved.artifacts,
            plan: resolved.plan,
            system_services: resolved.system_services,
            user_services: resolved.user_services,
            dora: resolved.dora,
            snapshot: resolved.snapshot,
            baseline,
        })
    }
}

async fn snapshot_file(source: &str, directory: &Path) -> Result<String> {
    let source = tokio::fs::canonicalize(source).await?;
    let bytes = crate::evidence::read_regular(&source, 256 * 1024 * 1024).await?;
    use std::os::unix::fs::PermissionsExt;
    let permissions = tokio::fs::metadata(&source).await?.permissions();
    let destination = directory.join(format!("{}-{:o}", digest(&bytes), permissions.mode()));
    if !destination.exists() {
        private_write(&destination, &bytes).await?;
        tokio::fs::set_permissions(&destination, permissions).await?;
    }
    Ok(destination.to_string_lossy().into_owned())
}
async fn freeze_files(model: &mut Value, artifacts: &Path) -> Result<()> {
    let directory = artifacts.join("inputs");
    crate::evidence::create_private_dir(&directory).await?;
    for service in model["services"].as_object_mut().unwrap().values_mut() {
        if let Some(mounts) = service.get_mut("volumes").and_then(Value::as_array_mut) {
            for mount in mounts {
                if mount["type"] == "bind"
                    && mount["read_only"] == true
                    && let Some(source) = mount["source"].as_str()
                    && tokio::fs::metadata(source).await?.is_file()
                {
                    mount["source"] = snapshot_file(source, &directory).await?.into();
                }
            }
        }
    }
    for kind in ["configs", "secrets"] {
        if let Some(items) = model.get_mut(kind).and_then(Value::as_object_mut) {
            for item in items.values_mut() {
                if let Some(source) = item["file"].as_str() {
                    item["file"] = snapshot_file(source, &directory).await?.into();
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
