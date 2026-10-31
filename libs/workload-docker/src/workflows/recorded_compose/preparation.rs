//! Resolve app inputs natively, then freeze one experiment and its startup stages.
mod validation;

use super::{ComposeRun, observation};
use crate::compose::{
    adapter::private_write,
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    overlay::OverlaySet,
    planner::{AnalyzeInput, ComposeDocument, DoraEnvironment},
    *,
};
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
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
impl PrepareRun {
    async fn prepare(&self, mut spec: ComposeRun) -> Result<PreparedRun> {
        // Resolve runtime settings and validate the request.
        super::super::shared_dora::resolve(&mut spec.dora, &spec.artifacts)?;
        super::super::shared_dora::validate_endpoints(&spec.dora)?;
        validation::validate(&spec)?;
        crate::evidence::create_private_dir(&spec.artifacts).await?;
        let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
        observation::snapshot(&artifacts, "before").await?;
        if spec.filesystem.is_some() {
            overlay::verify_namespace(self.adapter.socket_path()).await?;
        }
        // Freeze Dora inputs and merge apps while recording their startup stages.
        let dora = super::super::shared_dora::prepare(&spec.dora, &artifacts).await?;
        let mut input = AnalyzeInput {
            run: spec.id.clone(),
            project_name: crate::compose::adapter::project_name(
                "experiment",
                &spec.repository_root,
            ),
            artifacts: artifacts
                .to_str()
                .ok_or_else(|| invalid("artifact path is not UTF-8"))?
                .into(),
            controller_scope: super::reset::directory().to_string_lossy().into_owned(),
            owner_container_name: super::super::shared_dora::MACHINE_OWNER.into(),
            dora: Some(dora.clone()),
            environment: Some(DoraEnvironment {
                node_id_variable: dora_app::environment::NODE_ID.into(),
                forced_values: dora_app::environment::settings(
                    "",
                    dora.daemon_port,
                    &dora.zenoh_overlay,
                )
                .into_iter()
                .filter(|(k, _)| *k != dora_app::environment::NODE_ID)
                .map(|(k, v)| (k.into(), v))
                .collect(),
                runtime_values: BTreeMap::from([(
                    dora_app::environment::DAEMON_PORT.into(),
                    dora.daemon_port.to_string(),
                )]),
            }),
            ..Default::default()
        };
        if !dora.graph.is_empty() {
            let graph = super::super::shared_dora::graph(
                &dora,
                &crate::evidence::read_regular(Path::new(&dora.graph), 2 * 1024 * 1024).await?,
            )?;
            input.dora_node_ids = graph.nodes.iter().map(|n| n.id.to_string()).collect();
        }
        // Native Compose resolves each app; the planner owns all document transformations.
        for (apps, documents) in [
            (&spec.system_apps, &mut input.system_apps),
            (&spec.projects, &mut input.adopted_apps),
        ] {
            for app in apps {
                let normalized = self
                    .adapter
                    .configuration()
                    .normalize(
                        &spec.id,
                        &validation::app_input(app, &dora)?,
                        &input.project_name,
                    )
                    .await?;
                private_write(
                    &artifacts.join(format!("{}.inputs.json", app.key)),
                    &serde_json::to_vec(&json!({
                        "sources": normalized.input_hashes,
                        "diagnostics": normalized.stderr,
                        "compose_version": normalized.compose_version,
                    }))?,
                )
                .await?;
                documents.push(ComposeDocument {
                    settings: Some(app.clone()),
                    compose_json: normalized.model,
                });
            }
        }
        let planned = self
            .adapter
            .configuration()
            .plan(input, "experiment")
            .await?;
        let plan = planned.project;
        let system_services = planned.configuration.system_services;
        let user_services = planned.configuration.adopted_services;
        for warning in &plan.diagnostics {
            eprintln!("warning: {warning}");
        }
        // Capture the filesystem baseline for normal completion.
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| invalid("executor dropped"))?;
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(BaselineInput {
                projects: vec![plan.clone()],
                artifacts: artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?
            .into_value();
        Ok(PreparedRun {
            artifacts,
            plan,
            system_services,
            user_services,
            baseline,
            dora,
        })
    }
}

#[cfg(test)]
mod tests;
