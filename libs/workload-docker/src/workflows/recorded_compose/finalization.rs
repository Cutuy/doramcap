//! Output finalization and durable evidence gates for Compose resource removal.
use super::super::system_apps::{FinalizeRecordings, Recording};
use super::{ComposeRunReport, observation, projects::QuiescedProjects};
use crate::compose::{
    Diagnostics,
    adapter::private_write,
    error::invalid,
    filesystem::{CollectFilesystemChanges, CollectInput, FilesystemBaseline, FilesystemChanges},
    overlay::{FinalizeOverlays, OverlayResults, OverlaySet},
};
use orchestrator::{Input, LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, rc::Weak};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeInput {
    pub id: String,
    pub artifacts: PathBuf,
    pub baseline: FilesystemBaseline,
    pub overlays: Option<OverlaySet>,
    pub shutdown: QuiescedProjects,
    pub recordings: Vec<Recording>,
    pub require_volume_evidence: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizedRun {
    pub filesystem_changes: Option<FilesystemChanges>,
    pub filesystem_results: Option<OverlayResults>,
    pub recording_finalized: bool,
    /// Writers stopped, filesystem evidence durable, and overlays finalized.
    pub can_remove: bool,
    /// Recovery-backed runs retain volumes whose metadata could not be collected.
    pub preserve_volumes: bool,
}

/// Verify recordings and collect metadata concurrently after writer quiescence.
/// Checkpoint direct-mount evidence or finalize existing overlays before allowing
/// resource removal. Failure retains resources and diagnostics in the output;
/// recording validation failures remain independent of Docker cleanup eligibility.
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
        let collect = CollectInput {
            baseline: input.baseline,
            host_views: input
                .overlays
                .as_ref()
                .into_iter()
                .flat_map(|o| &o.trees)
                .map(|t| (t.source.clone(), t.merged.clone()))
                .collect(),
            projects: input
                .shutdown
                .projects
                .iter()
                .map(|p| p.plan.clone())
                .collect(),
        };
        let (recording, changes) = tokio::join!(
            async {
                if !input.shutdown.writers_stopped {
                    return Err(orchestrator::Error::message(
                        "recording writers not quiescent",
                    ));
                }
                executor.run::<FinalizeRecordings>(input.recordings).await
            },
            async {
                if !input.shutdown.writers_stopped {
                    return None;
                }
                match executor.run::<CollectFilesystemChanges>(collect).await {
                    Ok(changes) => Some(changes),
                    Err(error) => {
                        self.failure("filesystem changes", error);
                        None
                    }
                }
            },
        );
        if let Err(error) = &recording {
            self.failure("finalize recording", error);
        }
        let mut evidence_durable = input.shutdown.writers_stopped && changes.is_some();
        if input.overlays.is_none()
            && let Some(changes) = &changes
            && let Err(error) = super::super::compose_lifecycle::save_outer_evidence(
                &input.artifacts,
                &input.id,
                changes.value(),
            )
        {
            self.failure("filesystem checkpoint", error);
            evidence_durable = false;
        }
        let filesystem_results =
            if let (Some(changes), Some(overlays)) = (&changes, &input.overlays) {
                match executor
                    .run::<FinalizeOverlays>(Input::new(overlays.clone()).after(changes))
                    .await
                {
                    Ok(finalized) => Some(finalized.into_value()),
                    Err(error) => {
                        self.failure("finalize overlays", error);
                        None
                    }
                }
            } else {
                None
            };
        let filesystem_changes = changes.map(|c| c.into_value());
        let preserve_volumes = input.require_volume_evidence
            && filesystem_changes
                .as_ref()
                .is_some_and(|c| !c.named_volumes.coverage_complete);
        if preserve_volumes {
            self.failure(
                "filesystem changes",
                "named-volume evidence incomplete; volumes retained",
            );
        }
        Ok(FinalizedRun {
            can_remove: evidence_durable
                && (input.overlays.is_none() || filesystem_results.is_some()),
            filesystem_changes,
            filesystem_results,
            preserve_volumes,
            recording_finalized: recording.is_ok() && input.shutdown.dora_finalized,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportInput {
    pub artifacts: PathBuf,
    pub report: ComposeRunReport,
}
/// Capture post-state and write the private final report after event capture ends.
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
        if let Err(error) = observation::snapshot(&input.artifacts, "after").await {
            self.diagnostics
                .borrow_mut()
                .push(error.diagnostic("post-state"));
        }
        input.report.diagnostics = self.diagnostics.borrow().clone();
        let bytes = serde_json::to_vec_pretty(&input.report)
            .map_err(|e| orchestrator::Error::message(e.to_string()))?;
        private_write(&input.artifacts.join("report.json"), &bytes)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))?;
        Ok(input.report)
    }
}
