//! Side-effect-free Docker preflight and private run metadata preparation.
use super::super::{
    dora,
    system_apps::{self, PrepareSystemApps},
};
use super::{ComposeRun, observation, projects::LaunchInput};
use crate::compose::{
    adapter::validate_name,
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    overlay::OverlaySet,
    *,
};
use futures_util::future::join_all;
use orchestrator::{Input, LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedRun {
    pub artifacts: PathBuf,
    pub groups: Vec<LaunchInput>,
    pub baseline: FilesystemBaseline,
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

/// Validate inputs, resolve all projects, capture a baseline before any mount or container is created. Failures leave private
/// metadata only. Supporting apps precede ordered first-party and workload groups.
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
impl PrepareRun {
    async fn prepare(&self, spec: ComposeRun) -> Result<PreparedRun> {
        validate(&spec)?;
        let mut builder = tokio::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(&spec.artifacts).await?;
        let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
        observation::snapshot(&artifacts, "before").await?;
        if spec.filesystem.is_some() {
            crate::compose::overlay::verify_namespace(self.adapter.socket_path()).await?;
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
        let system_count = system.value().projects.len();
        let projects: Vec<_> = system
            .value()
            .projects
            .iter()
            .cloned()
            .chain(spec.first_party.clone())
            .chain(spec.projects.clone())
            .collect();
        let mut keys = HashSet::new();
        if projects.iter().any(|p| !keys.insert(&p.key)) {
            return Err(invalid("system and workload project keys must be unique"));
        }
        let repository = spec
            .repository_root
            .to_str()
            .ok_or_else(|| invalid("repository path is not UTF-8"))?
            .to_owned();
        let resolved = join_all(projects.into_iter().map(|project| {
            executor.run::<ResolveCompose>(
                Input::new(ResolveInput {
                    repository_root: Some(repository.clone()),
                    run: spec.id.clone(),
                    artifacts: artifacts.to_string_lossy().into_owned(),
                    project: Some(project),
                })
                .after(&system),
            )
        }))
        .await;
        // Await every independent resolution before propagating an error.
        let plans: Vec<_> = resolved
            .into_iter()
            .collect::<orchestrator::Result<_>>()
            .map_err(|e| invalid(e.to_string()))?;
        let projects: Vec<_> = plans.iter().map(|p| p.value().clone()).collect();
        dora::bind(&spec.system_apps.dora_flows, &projects)?;
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
        let group = |projects, recordings| LaunchInput {
            projects,
            recordings,
            readiness_seconds: spec.readiness_seconds,
        };
        let mut groups = Vec::new();
        if system_count != 0 {
            groups.push(group(
                projects[..system_count].to_vec(),
                system.value().recordings.clone(),
            ));
        }
        for plan in &projects[system_count..system_count + spec.first_party.len()] {
            groups.push(group(vec![plan.clone()], vec![]));
        }
        groups.push(group(
            projects[system_count + spec.first_party.len()..].to_vec(),
            vec![],
        ));
        Ok(PreparedRun {
            artifacts,
            groups,
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
    let mut keys = HashSet::new();
    for project in spec.first_party.iter().chain(&spec.projects) {
        validate_name(&project.key)?;
        if !Path::new(&project.directory).is_absolute() || !keys.insert(project.key.clone()) {
            return Err(invalid(
                "require absolute project directories and unique keys; system is reserved",
            ));
        }
    }
    Ok(())
}
