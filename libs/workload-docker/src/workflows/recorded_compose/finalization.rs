//! Output finalization and durable evidence gates for Compose resource removal.
use super::ComposeRunReport;
use compose_apps::compose::{Diagnostics, adapter::private_write, error::invalid};
use docker_engine::docker_proto::OwnedContainer;
use host_filesystem::{
    changes::{CollectInput, FilesystemChanges, FilesystemSources},
    overlay::{OverlayResults, OverlaySet},
};
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    rc::{Rc, Weak},
};
use workload_lifecycle::capture::{self, CaptureFinalization};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeInput {
    pub id: String,
    pub artifacts: PathBuf,
    pub sources: FilesystemSources,
    pub overlays: Option<OverlaySet>,
    pub shutdown: compose_apps::compose::Shutdown,
    pub project: compose_apps::compose::ResolvedComposeProject,
}
/// Frozen Docker identity, mount sources and uppers after writer quiescence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObserveDockerFilesystemInput {
    pub project: compose_apps::compose::ResolvedComposeProject,
    pub sources: FilesystemSources,
    pub overlays: Option<OverlaySet>,
}

/// Verify stopped owned containers, save their layer changes, without inspecting named-volume contents.
/// Errors retain resources and are recorded before shared collection starts.
pub struct ObserveDockerFilesystem {
    pub adapter: Rc<compose_apps::compose::DockerAdapter>,
}
impl Step for ObserveDockerFilesystem {
    const ID: Option<&'static str> = Some("compose.filesystem.observe");
    const DISPLAY_NAME: Option<&'static str> = Some("Observe Docker file changes");
    type Input = ObserveDockerFilesystemInput;
    type Output = CollectInput;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        let state = self
            .adapter
            .project(&input.project)
            .inspect()
            .await
            .map_err(|error| orchestrator::Error::message(error.to_string()))?;
        let containers: Vec<_> = state
            .containers
            .into_iter()
            .map(|container| OwnedContainer {
                consumer: format!("{}/{}", input.project.name, container.service),
                id: container.id,
            })
            .collect();
        let container_layers = docker_engine::filesystem::container_changes(
            &self.adapter.engine,
            &containers,
            &input.sources.directory,
        )
        .await
        .map_err(|error| orchestrator::Error::message(error.to_string()))?;
        let mut sources = input.sources;
        sources.excluded.extend(containers.iter().map(|container| {
            compose_apps::compose::adapter::stopped_record(&input.project.key, &container.id)
        }));
        Ok(CollectInput {
            sources,
            overlays: input.overlays,
            container_layers: Some(container_layers),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizedRun {
    pub filesystem_changes: Option<FilesystemChanges>,
    pub filesystem_results: Option<OverlayResults>,
    /// Writers stopped, filesystem evidence durable, and overlays finalized.
    pub can_remove: bool,
    /// Retain volumes when collection failed; untracked volume coverage does not block removal.
    pub preserve_volumes: bool,
}

/// Collect filesystem metadata after writer quiescence. Checkpoint direct-mount
/// evidence or finalize existing overlays before allowing resource removal.
/// Failure retains resources and diagnostics in the output.
pub struct FinalizeRun {
    pub executor: Weak<LocalExecutor>,
    pub diagnostics: Diagnostics,
}
impl Step for FinalizeRun {
    const ID: Option<&'static str> = Some("compose.finalize");
    const DISPLAY_NAME: Option<&'static str> = Some("Finalize recording");
    type Input = FinalizeInput;
    type Output = FinalizedRun;
    async fn run(&mut self, input: FinalizeInput) -> orchestrator::Result<FinalizedRun> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("run executor dropped"))?;
        let finalized = if input.shutdown.stopped {
            let result = async {
                let collect = executor
                    .run::<ObserveDockerFilesystem>(ObserveDockerFilesystemInput {
                        project: input.project.clone(),
                        sources: input.sources.clone(),
                        overlays: input.overlays.clone(),
                    })
                    .await?;
                Ok::<_, orchestrator::Error>(
                    capture::finalize(&executor, collect, input.overlays.clone()).await,
                )
            }
            .await;
            match result {
                Ok(finalized) => finalized,
                Err(error) => {
                    self.failure("filesystem changes", error);
                    CaptureFinalization {
                        changes: None,
                        results: None,
                        failures: vec![],
                    }
                }
            }
        } else {
            CaptureFinalization {
                changes: None,
                results: None,
                failures: vec![],
            }
        };
        for (operation, error) in finalized.failures {
            self.failure(operation, error);
        }
        let changes = finalized.changes;
        let filesystem_results = finalized.results;
        let mut evidence_durable = input.shutdown.stopped && changes.is_some();
        if input.overlays.is_none()
            && let Some(changes) = &changes
            && let Err(error) = super::super::compose_lifecycle::save_outer_evidence(
                &input.artifacts,
                &input.id,
                changes,
            )
        {
            self.failure("filesystem checkpoint", error);
            evidence_durable = false;
        }
        let filesystem_changes = changes;
        let preserve_volumes = filesystem_changes.is_none();
        Ok(FinalizedRun {
            can_remove: evidence_durable
                && (input.overlays.is_none() || filesystem_results.is_some()),
            filesystem_changes,
            filesystem_results,
            preserve_volumes,
        })
    }
}
impl FinalizeRun {
    fn failure(&self, operation: &str, error: impl std::fmt::Display) {
        self.diagnostics
            .borrow_mut()
            .push(invalid(error.to_string()).diagnostic(operation));
    }
}

/// Run-relative final workflow report owned by Docker finalization.
pub(super) const REPORT_RECORD: &str = "report.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportInput {
    pub artifacts: PathBuf,
    pub report: ComposeRunReport,
}
/// Write the private final report after event capture ends.
/// A report-writing error preserves the caller's blocked admission state.
pub struct WriteRunReport {
    pub diagnostics: Diagnostics,
}
impl Step for WriteRunReport {
    const ID: Option<&'static str> = Some("compose.report");
    const DISPLAY_NAME: Option<&'static str> = Some("Write run report");
    type Input = ReportInput;
    type Output = ComposeRunReport;
    async fn run(&mut self, mut input: ReportInput) -> orchestrator::Result<ComposeRunReport> {
        input.report.diagnostics = self.diagnostics.borrow().clone();
        let bytes = serde_json::to_vec_pretty(&input.report)
            .map_err(|e| orchestrator::Error::message(e.to_string()))?;
        private_write(&input.artifacts.join(REPORT_RECORD), &bytes)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))?;
        Ok(input.report)
    }
}

#[cfg(test)]
mod tests;
