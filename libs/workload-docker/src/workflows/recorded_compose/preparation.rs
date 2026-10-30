//! Side-effect-free Docker preflight and private run metadata preparation.
use super::super::system_apps::{self, PrepareSystemApps};
use super::{
    ComposeRun, observation,
    projects::LaunchInput,
    resolution::{self, ReviewReceipt},
};
use crate::compose::{
    adapter::validate_name,
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    overlay::OverlaySet,
    *,
};
use crate::workflows::dora;
use orchestrator::{Input, LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::PathBuf,
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedRun {
    pub artifacts: PathBuf,
    pub groups: Vec<LaunchInput>,
    pub baseline: FilesystemBaseline,
    pub receipt: ReviewReceipt,
}
impl PreparedRun {
    pub fn projects(&self) -> Vec<ResolvedComposeProject> {
        self.groups
            .iter()
            .flat_map(|g| g.projects.clone())
            .collect()
    }
    /// Substitute the frozen OverlayFS execution manifests without changing order.
    pub fn launch_groups(&self, overlays: Option<&OverlaySet>) -> Vec<LaunchInput> {
        let mut groups = self.groups.clone();
        if let Some(overlays) = overlays {
            for plan in groups.iter_mut().flat_map(|g| &mut g.projects) {
                *plan = overlays
                    .projects
                    .iter()
                    .find(|p| p.key == plan.key)
                    .expect("overlay preparation preserves project keys")
                    .clone();
            }
        }
        groups
    }
}

/// Validate inputs, resolve all projects, capture a baseline and freeze recovery
/// identities before any mount or container is created. Failures leave private
/// metadata only. Supporting apps precede ordered first-party and workload groups.
pub struct PrepareRun {
    pub executor: Weak<LocalExecutor>,
    pub adapter: Rc<DockerAdapter>,
}
impl Step for PrepareRun {
    fn descriptor() -> Option<orchestrator::StepDescriptor> {
        Some(orchestrator::StepDescriptor::new(
            "compose.prepare",
            "Prepare project",
            orchestrator::ProgressRetention::Keep,
            orchestrator::ProgressKind::Operation,
        ))
    }
    type Input = ComposeRun;
    type Output = PreparedRun;
    async fn run(&mut self, spec: ComposeRun) -> orchestrator::Result<PreparedRun> {
        self.prepare(spec)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
impl PrepareRun {
    async fn prepare(&self, spec: ComposeRun) -> Result<PreparedRun> {
        validate(&spec)?;
        let mut builder = tokio::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(&spec.artifacts).await?;
        let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
        observation::snapshot(&artifacts, "before").await?;
        if spec.filesystem.is_some() {
            self.adapter.verify_overlay_namespace().await?;
        }
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| invalid("run executor dropped"))?;
        let system = executor
            .run::<PrepareSystemApps>(system_apps::PrepareInput {
                config: spec.system_apps.clone(),
                artifacts: artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?;
        let (plans, resolved) = resolution::resolve_with(
            &spec,
            &artifacts,
            system.value(),
            |input| async {
                executor
                    .run::<ResolveCompose>(Input::new(input).after(&system))
                    .await
                    .map_err(|e| invalid(e.to_string()))
            },
            |output| output.value(),
        )
        .await?;
        let projects: Vec<_> = plans.iter().map(|p| p.value().clone()).collect();
        let mut input = Input::new(BaselineInput {
            projects: projects.clone(),
            artifacts: artifacts.clone(),
        });
        for plan in &plans {
            input = input.after(plan);
        }
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(input)
            .await
            .map_err(|e| invalid(e.to_string()))?;
        if let Some(directory) = &spec.recovery_anchor_dir {
            super::super::recovery::write_anchor(
                directory,
                &artifacts,
                &spec.id,
                &projects,
                &baseline.value().recovery_files(),
            )
            .await?;
        }
        Ok(PreparedRun {
            artifacts,
            groups: resolved.groups,
            receipt: resolved.receipt,
            baseline: baseline.into_value(),
        })
    }
}

pub(super) fn validate(spec: &ComposeRun) -> Result<()> {
    validate_name(&spec.id)?;
    for flow in &spec.system_apps.dora_flows {
        dora::validate_runtime(flow)?;
    }
    super::validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
    if !spec.artifacts.is_absolute()
        || (spec.projects.is_empty()
            && spec.first_party.is_empty()
            && spec.system_apps.cpu_monitor.is_none())
    {
        return Err(invalid(
            "require absolute new artifact directory, a workload or supporting app, positive readiness and stop_seconds 1..300",
        ));
    }
    if spec
        .recovery_anchor_dir
        .as_ref()
        .is_some_and(|p| !p.is_absolute())
    {
        return Err(invalid("recovery anchor directory must be absolute"));
    }
    let mut keys = HashSet::new();
    for project in spec.first_party.iter().chain(&spec.projects) {
        validate_name(&project.key)?;
        if !project.directory.is_absolute() || !keys.insert(project.key.clone()) {
            return Err(invalid(
                "require absolute project directories and unique keys; system is reserved",
            ));
        }
    }
    Ok(())
}
