//! Compose startup, observation and teardown policy expressed through shared steps.
use super::{
    dora,
    observation::EventCapture,
    project_steps::{
        LaunchDoraFlows, LaunchFlowsInput, LaunchProjects, ObserveInput, ObserveProjects,
        QuiesceInput, QuiesceProjects, RemoveInput, RemoveProjects,
    },
    run_finalization::{FinalizeInput, FinalizeRun, ReportInput, WriteRunReport},
    run_preparation::PrepareRun,
    system_apps::SystemAppsConfig,
};
use crate::compose::{
    error::invalid,
    filesystem::FilesystemChanges,
    overlay::{OverlayPolicy, OverlayResults, PrepareInput, PrepareOverlays},
    *,
};
use orchestrator::{Input, Observer};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, future::Future, path::PathBuf, rc::Rc, time::Duration};
use tokio::sync::oneshot;

/// Single-session runner with concurrent independent Compose operations.
///
/// `run` accepts fully prepared settings and a cooperative stop future. It never
/// discovers launch inputs or chooses profiles. Ordinary failures await
/// independent teardown. Dropping/aborting `run` cannot await cleanup and leaves
/// this manager blocked; callers must signal stop and continue awaiting the result.
/// Unknown/failed cleanup blocks subsequent runs on this manager. Direct-mount
/// recovery is available through [`super::recovery::recover_project`]. Admission
/// across processes remains the caller’s responsibility.
pub struct ComposeRunner {
    adapter: Rc<DockerAdapter>,
    blocked: bool,
}
impl ComposeRunner {
    /// Create an unblocked manager sharing the caller-owned Docker adapter.
    pub fn new(adapter: Rc<DockerAdapter>) -> Self {
        Self {
            adapter,
            blocked: false,
        }
    }
    /// Return whether an unfinished run or unverified teardown prevents admission.
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// Execute one prepared request through startup, observation and teardown.
    ///
    /// # Parameters
    ///
    /// `spec` owns launch settings and a new artifact path. `stop` cooperatively
    /// ends observation or interrupts startup. `observer` is owned by this run's
    /// executor and receives synchronous step events, including JSON payloads
    /// which may contain sensitive configuration.
    ///
    /// # Return value and errors
    ///
    /// Returns a [`ComposeRunReport`] containing runtime failures and independent cleanup
    /// outcomes. Input/preflight errors return `Err` before launch. Filesystem or
    /// evidence-writing failures may also return `Err` later in the run. Callers
    /// must inspect report success and manager admission state separately.
    ///
    /// # Ownership and cancellation
    ///
    /// Exclusive access serializes calls on this manager. There is no admission
    /// lock across managers or processes. Ordinary runtime failures await teardown.
    /// Dropping the future cannot await cleanup and can leave resources alive and
    /// this manager blocked. Signal `stop` and await completion for orderly shutdown.
    pub async fn run(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
    ) -> Result<ComposeRunReport> {
        self.run_with_ready(spec, stop, observer, None).await
    }

    /// Run a session and notify the caller after every project becomes ready.
    /// A closed receiver at notification triggers teardown. Continue awaiting
    /// this future for cleanup; plans remain valid only during the run.
    pub async fn run_with_ready(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
    ) -> Result<ComposeRunReport> {
        if self.blocked {
            return Err(invalid(
                "manager blocked by an unfinished or unverified experiment",
            ));
        }
        let diagnostics = Rc::new(RefCell::new(Vec::new()));
        let executor = super::steps::executor(
            self.adapter.clone(),
            diagnostics.clone(),
            observer,
            &spec.id,
        );
        let prepared = executor
            .run::<PrepareRun>(spec.clone())
            .await
            .map_err(step_error)?;
        let artifacts = prepared.value().artifacts.clone();

        // Mounts cross the side-effect boundary; dropping this run now blocks admission.
        self.blocked = true;
        let overlays = if let Some(policy) = &spec.filesystem {
            Some(
                executor
                    .run::<PrepareOverlays>(
                        Input::new(PrepareInput {
                            artifacts: artifacts.clone(),
                            projects: prepared.value().projects(),
                            policy: policy.clone(),
                        })
                        .after(&prepared),
                    )
                    .await
                    .map_err(step_error)?,
            )
        } else {
            None
        };
        let groups = prepared
            .value()
            .launch_groups(overlays.as_ref().map(|o| o.value()));
        let plans: Vec<_> = groups.iter().flat_map(|g| g.projects.clone()).collect();
        let mut flows = dora::bind(&spec.system_apps.dora_flows, &plans)?;
        let mut events = EventCapture::start(
            self.adapter.docker.clone(),
            spec.id.clone(),
            artifacts.join("docker-events.jsonl"),
        );
        let mut attempted = Vec::new();
        let mut launches = Vec::new();
        let mut flow_start = None;
        tokio::pin!(stop);
        let execute = async {
            // Supporting groups precede workloads. Siblings within a group run concurrently.
            for group in &groups {
                attempted.push(group.projects.clone());
                let mut input = Input::new(group.clone()).after(&prepared);
                if let Some(overlays) = &overlays {
                    input = input.after(overlays);
                }
                if let Some(previous) = launches.last() {
                    input = input.after(previous);
                }
                launches.push(executor.run::<LaunchProjects>(input).await?);
            }
            let mut input = Input::new(LaunchFlowsInput {
                flows: flows.clone(),
                readiness_seconds: spec.readiness_seconds,
            })
            .after(&prepared);
            if let Some(launched) = launches.last() {
                input = input.after(launched);
            }
            let started = executor.run::<LaunchDoraFlows>(input).await?;
            flows = started.value().clone();
            flow_start = Some(started);
            if let Some(ready) = ready {
                ready
                    .send(plans.clone())
                    .map_err(|_| orchestrator::Error::message("readiness receiver dropped"))?;
            }
            loop {
                let mut input = Input::new(ObserveInput {
                    projects: plans.clone(),
                    flows: flows.clone(),
                })
                .after(&prepared);
                if let Some(launched) = launches.last() {
                    input = input.after(launched);
                }
                if let Some(started) = &flow_start {
                    input = input.after(started);
                }
                executor.run::<ObserveProjects>(input).await?;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), orchestrator::Error>(())
        };
        let outcome = tokio::select! {
            result = execute => result.map_err(|e| e.to_string()),
            _ = &mut stop => Ok(()),
            error = events.failure() => Err(error),
        };

        // Stop all writer roles before collecting filesystems or removing infrastructure.
        let mut input = Input::new(QuiesceInput {
            groups: attempted,
            flows,
            stop_seconds: spec.stop_seconds,
        })
        .after(&prepared);
        for launched in &launches {
            input = input.after(launched);
        }
        if let Some(started) = &flow_start {
            input = input.after(started);
        }
        let shutdown = executor
            .run::<QuiesceProjects>(input)
            .await
            .map_err(step_error)?;
        let finalized = executor
            .run::<FinalizeRun>(
                Input::new(FinalizeInput {
                    id: spec.id.clone(),
                    artifacts: artifacts.clone(),
                    baseline: prepared.value().baseline.clone(),
                    overlays: overlays.as_ref().map(|o| o.value().clone()),
                    shutdown: shutdown.value().clone(),
                    recordings: groups.iter().flat_map(|g| g.recordings.clone()).collect(),
                    require_volume_evidence: spec.recovery_anchor_dir.is_some(),
                })
                .after(&prepared)
                .after(&shutdown),
            )
            .await
            .map_err(step_error)?;
        let cleanup = executor
            .run::<RemoveProjects>(
                Input::new(RemoveInput {
                    projects: shutdown.value().projects.clone(),
                    stop_seconds: spec.stop_seconds,
                    can_remove: finalized.value().can_remove,
                    preserve_volumes: finalized.value().preserve_volumes,
                })
                .after(&shutdown)
                .after(&finalized),
            )
            .await
            .map_err(step_error)?;

        let event_result = events.finish().await;
        if let Err(error) = &event_result {
            diagnostics
                .borrow_mut()
                .push(error.diagnostic("Docker events"));
        }
        if executor.observer_failures() != 0 {
            diagnostics
                .borrow_mut()
                .push(invalid("workflow observer lost events").diagnostic("observer"));
        }
        let finalized = finalized.value();
        let boundary_clean = finalized.can_remove
            && cleanup.value().iter().all(|c| {
                c.verified
                    && c.diagnostics.is_empty()
                    && (!finalized.preserve_volumes || c.retained_volumes.is_empty())
            });
        let report = executor
            .run::<WriteRunReport>(
                Input::new(ReportInput {
                    artifacts,
                    report: ComposeRunReport {
                        id: spec.id,
                        execution_error: outcome.err(),
                        diagnostics: Vec::new(),
                        cleanup: cleanup.value().clone(),
                        boundary_clean,
                        writers_stopped: shutdown.value().writers_stopped,
                        recording_finalized: finalized.recording_finalized,
                        event_stream_healthy: event_result.is_ok(),
                        filesystem_changes: finalized.filesystem_changes.clone(),
                        filesystem_results: finalized.filesystem_results.clone(),
                    },
                })
                .after(&cleanup),
            )
            .await
            .map_err(step_error)?;
        self.blocked = !boundary_clean;
        Ok(report.into_value())
    }
}
fn step_error(error: impl std::fmt::Display) -> Error {
    invalid(error.to_string())
}

/// Inputs to the Compose workflow: workloads and supporting apps sharing one
/// run lifetime. Images must exist locally.
/// Artifact directory must be new and have an existing parent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeRun {
    /// Some isolates writable directory binds through OverlayFS. None uses the
    /// supplied mounts directly. Omitted JSON retains the default write-back policy.
    #[serde(default = "default_filesystem")]
    pub filesystem: Option<OverlayPolicy>,
    /// Caller-selected run identifier used in Docker ownership labels and project names.
    pub id: String,
    /// Absolute, new artifact directory; its parent must already exist.
    pub artifacts: PathBuf,
    /// Existing private directory outside every workload mount, used to freeze
    /// recovery identities before launch. The caller protects it from workloads.
    #[serde(default)]
    pub recovery_anchor_dir: Option<PathBuf>,
    /// Foreign Compose projects owned by this run; at least one is required.
    pub projects: Vec<ComposeProject>,
    /// Additional prepared first-party projects, such as a configured DORA/ROS
    /// subscriber. Started before foreign projects and stopped after them.
    #[serde(default)]
    pub first_party: Vec<ComposeProject>,
    /// Supporting-app configuration, interpreted by the dedicated system-app steps.
    #[serde(default)]
    pub system_apps: SystemAppsConfig,
    /// Positive startup/readiness budget in seconds; default 60.
    #[serde(default = "readiness_default")]
    pub readiness_seconds: u64,
    /// Graceful stop budget per project in seconds, from 1 through 300; default 15.
    #[serde(default = "stop_default")]
    pub stop_seconds: u64,
}
fn default_filesystem() -> Option<OverlayPolicy> {
    Some(OverlayPolicy::default())
}
fn readiness_default() -> u64 {
    60
}
fn stop_default() -> u64 {
    15
}

/// Outcome and cleanup are separate. Inspect `successful()` and `boundary_clean`
/// before treating evidence as complete or admitting another experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposeRunReport {
    /// Successful overlay disposition, absent on preparation/harvest failure.
    #[serde(default)]
    pub filesystem_results: Option<OverlayResults>,
    /// Run identifier.
    pub id: String,
    /// Startup/observation failure, independently of cleanup results.
    pub execution_error: Option<String>,
    /// Structured workflow and step failure evidence.
    pub diagnostics: Vec<Diagnostic>,
    /// Independent teardown results for attempted projects.
    pub cleanup: Vec<Cleanup>,
    /// Whether owned runtime resources were verified cleaned up.
    pub boundary_clean: bool,
    /// All attempted project writers stopped, independently of evidence or removal.
    #[serde(default)]
    pub writers_stopped: bool,
    /// Whether configured supporting-app recordings passed finalization checks.
    pub recording_finalized: bool,
    /// Whether event capture completed without a detected error; not a losslessness guarantee.
    pub event_stream_healthy: bool,
    /// Category-specific metadata manifests. Expected changes/coverage warnings
    /// are informational; unavailable required collection prevents removal.
    #[serde(default)]
    pub filesystem_changes: Option<FilesystemChanges>,
}
impl ComposeRunReport {
    pub fn successful(&self) -> bool {
        self.execution_error.is_none()
            && self.boundary_clean
            && self.recording_finalized
            && self.event_stream_healthy
            && self.diagnostics.is_empty()
            && self.cleanup.iter().all(|x| x.diagnostics.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filesystem_policy_default_and_direct_mount_opt_out() {
        // Arrange
        let mut config = serde_json::json!({"id":"run", "artifacts":"/tmp/example", "projects":[{
            "key":"workload", "directory":"/tmp", "files":["compose.yaml"]
        }]});
        // Act / Assert
        let defaulted: ComposeRun = serde_json::from_value(config.clone()).unwrap();
        assert!(defaulted.filesystem.is_some());
        assert!(super::super::run_preparation::validate(&defaulted).is_ok());
        config["filesystem"] = serde_json::Value::Null;
        let direct: ComposeRun = serde_json::from_value(config).unwrap();
        assert!(direct.filesystem.is_none());
        assert!(super::super::run_preparation::validate(&direct).is_ok());
    }
}
