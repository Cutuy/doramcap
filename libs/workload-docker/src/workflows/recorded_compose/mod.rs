//! Compose startup, observation and teardown policy expressed through shared steps.
mod execution;
pub mod finalization;
mod observation;
pub mod preparation;
pub mod projects;
pub mod reset;

use self::{
    finalization::{FinalizeInput, FinalizeRun, ReportInput, WriteRunReport},
    observation::EventCapture,
    preparation::PrepareRun,
    projects::{
        LaunchDoraFlows, LaunchFlowsInput, LaunchProjects, ObserveInput, ObserveProjects,
        QuiesceInput, QuiesceProjects, RemoveInput, RemoveProjects,
    },
};
use super::{dora, system_apps::SystemAppsConfig};
use crate::compose::{
    error::invalid,
    filesystem::FilesystemChanges,
    overlay::{OverlayPolicy, OverlayResults, PrepareInput, PrepareOverlays},
    *,
};
use orchestrator::{Input, Observer};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, future::Future, path::PathBuf, rc::Rc};
use tokio::sync::oneshot;

/// Compose-specific notifications to an in-process listener; delivery never controls the run.
#[derive(Default)]
pub struct RunSignals {
    /// Notify after all projects are ready; a closed receiver is harmless.
    pub ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
    /// Notify when teardown begins, before stopping writers; delivery does not confirm shutdown.
    pub quiescing: Option<oneshot::Sender<()>>,
}

impl RunSignals {
    fn notify_ready(&mut self, plans: Vec<ResolvedComposeProject>) {
        if let Some(ready) = self.ready.take() {
            let _ = ready.send(plans);
        }
    }
}

/// Run prepared Compose projects with repository-scoped admission and reset.
/// Signal stop and await completion for cleanup. A later run resets resources
/// left by cancellation or incomplete teardown before launching new workloads.
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
    /// A canonical repository lock serializes managers and processes.
    /// Ordinary runtime failures await teardown; later runs first reset the scope.
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
    /// A closed receiver is harmless. Continue awaiting this future for cleanup;
    /// plans remain valid only during the run.
    pub async fn run_with_ready(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
    ) -> Result<ComposeRunReport> {
        self.run_with_preflight(spec, stop, observer, ready, |_| Ok(()))
            .await
    }

    /// Check the freshly prepared run before any mount or Docker launch.
    /// A rejected check leaves the runner unblocked and returns its error.
    pub async fn run_with_preflight(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
        check: impl FnOnce(&preparation::PreparedRun) -> Result<()>,
    ) -> Result<ComposeRunReport> {
        self.run_with_signals(
            spec,
            stop,
            observer,
            RunSignals {
                ready,
                ..Default::default()
            },
            check,
        )
        .await
    }

    /// Run with Compose notifications and preflight validation.
    /// Teardown is signaled before stopping writers, including automatic shutdown.
    /// Notifications are best-effort; use `stop` to request orderly shutdown.
    pub async fn run_with_signals(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        signals: RunSignals,
        check: impl FnOnce(&preparation::PreparedRun) -> Result<()>,
    ) -> Result<ComposeRunReport> {
        // Keep large backend preparation futures off callers' async state machines.
        Box::pin(self.run_inner(spec, stop, observer, signals, check)).await
    }

    async fn run_inner(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        mut signals: RunSignals,
        check: impl FnOnce(&preparation::PreparedRun) -> Result<()>,
    ) -> Result<ComposeRunReport> {
        // Serialize this repository and reset its fixed resource scope before preparation.
        validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
        let controller = Rc::new(reset::Controller::acquire(&spec.repository_root)?);
        self.blocked = true;
        let mut spec = spec;
        spec.repository_root = controller.repository.clone();
        let diagnostics = Rc::new(RefCell::new(Vec::new()));
        let executor = super::steps::executor(
            self.adapter.clone(),
            diagnostics.clone(),
            observer,
            &spec.id,
        );
        let mut executor =
            Rc::try_unwrap(executor).map_err(|_| invalid("executor already shared"))?;
        let backend = self.adapter.clone();
        let scope = controller.clone();
        let stop_seconds = spec.stop_seconds;
        executor.register::<reset::ResetExperiment>(move || reset::ResetExperiment {
            adapter: backend.clone(),
            controller: scope.clone(),
            stop_seconds,
        });
        executor.validate_steps().map_err(step_error)?;
        let executor = super::steps::attach(executor, self.adapter.clone(), diagnostics.clone());
        executor
            .run::<reset::ResetExperiment>(())
            .await
            .map_err(step_error)?;
        self.blocked = false;
        let prepared = executor
            .run::<PrepareRun>(spec.clone())
            .await
            .map_err(step_error)?;
        check(prepared.value())?;
        let artifacts = prepared.value().artifacts.clone();

        // Mounts cross the side-effect boundary; dropping this run now blocks admission.
        self.blocked = true;
        let overlays = if let Some(policy) = &spec.filesystem {
            Some(
                executor
                    .run::<PrepareOverlays>(
                        Input::new(PrepareInput {
                            artifacts: artifacts.clone(),
                            workspace: controller.workspace(),
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
        let execution = async {
            // Retain each attempted group before awaiting startup so cancellation can stop it.
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

            // Yield before notifying readiness so a stop made ready during startup wins.
            tokio::task::yield_now().await;
            signals.notify_ready(plans.clone());
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
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), orchestrator::Error>(())
        };
        let execution_error = execution::until_stop(stop, events.failure(), execution).await;

        // Completion is independent of cancellation and keeps every evidence gate intact.
        if let Some(quiescing) = signals.quiescing.take() {
            let _ = quiescing.send(());
        }
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
        let shutdown = executor.run::<QuiesceProjects>(input).await;
        let finalized = async {
            let shutdown = shutdown.as_ref().map_err(step_error)?;
            executor
                .run::<FinalizeRun>(
                    Input::new(FinalizeInput {
                        id: spec.id.clone(),
                        artifacts: artifacts.clone(),
                        baseline: prepared.value().baseline.clone(),
                        overlays: overlays.as_ref().map(|o| o.value().clone()),
                        shutdown: shutdown.value().clone(),
                        recordings: groups.iter().flat_map(|g| g.recordings.clone()).collect(),
                    })
                    .after(&prepared)
                    .after(shutdown),
                )
                .await
                .map_err(step_error)
        }
        .await;
        let cleanup = async {
            let shutdown = shutdown.as_ref().map_err(step_error)?;
            let finalized = finalized.as_ref().map_err(step_error)?;
            executor
                .run::<RemoveProjects>(
                    Input::new(RemoveInput {
                        projects: shutdown.value().projects.clone(),
                        stop_seconds: spec.stop_seconds,
                        can_remove: finalized.value().can_remove,
                        preserve_volumes: finalized.value().preserve_volumes,
                    })
                    .after(shutdown)
                    .after(finalized),
                )
                .await
                .map_err(step_error)
        }
        .await;
        // Stop event capture even when a completion step cannot produce its evidence.
        let event_result = events.finish().await;
        for error in [
            shutdown.as_ref().err().map(ToString::to_string),
            finalized.as_ref().err().map(ToString::to_string),
            finalized
                .as_ref()
                .ok()
                .filter(|output| !output.value().recording_finalized)
                .map(|_| "recording finalization failed".to_owned()),
            cleanup.as_ref().err().map(ToString::to_string),
            event_result.as_ref().err().map(ToString::to_string),
        ]
        .into_iter()
        .flatten()
        {
            diagnostics
                .borrow_mut()
                .push(invalid(error).diagnostic("completion"));
        }
        if executor.observer_failures() != 0 {
            diagnostics
                .borrow_mut()
                .push(invalid("workflow observer lost events").diagnostic("observer"));
        }
        let finalized = finalized.as_ref().map_err(step_error)?;
        let cleanup = cleanup.as_ref().map_err(step_error)?;
        let shutdown = shutdown.as_ref().map_err(step_error)?;
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
                        execution_error,
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
                .after(cleanup),
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
    /// Canonical repository defining exclusive admission, reset and project identity.
    pub repository_root: PathBuf,
    /// Foreign Compose projects owned by this run; may be empty with supporting apps.
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
/// Validate the shared startup and per-project shutdown budgets before side effects.
pub fn validate_budgets(readiness_seconds: u64, stop_seconds: u64) -> Result<()> {
    if readiness_seconds == 0 {
        return Err(invalid("readiness_seconds must be positive"));
    }
    if !(1..=300).contains(&stop_seconds) {
        return Err(invalid("stop_seconds must be in 1..=300"));
    }
    Ok(())
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
    fn dropped_readiness_listener_does_not_interrupt_workflow() {
        // Arrange
        let (sender, receiver) = oneshot::channel();
        let mut signals = RunSignals {
            ready: Some(sender),
            ..Default::default()
        };
        drop(receiver);

        // Act
        signals.notify_ready(Vec::new());

        // Assert
        assert!(signals.ready.is_none());
    }

    #[test]
    fn readiness_is_delivered_once() {
        // Arrange
        let (sender, mut receiver) = oneshot::channel();
        let mut signals = RunSignals {
            ready: Some(sender),
            ..Default::default()
        };

        // Act
        signals.notify_ready(Vec::new());

        // Assert
        assert!(receiver.try_recv().unwrap().is_empty());
        assert!(signals.ready.is_none());

        // Act / Assert: repeated notification is harmless.
        signals.notify_ready(Vec::new());
    }

    #[test]
    fn supporting_apps_allow_monitor_only_and_empty_runs_reject() {
        // Arrange
        let mut spec: ComposeRun = serde_json::from_value(serde_json::json!({
            "id":"monitor", "repository_root":"/tmp", "artifacts":"/tmp/new-monitor-run", "projects":[], "filesystem":null
        }))
        .unwrap();
        // Act / Assert
        assert!(preparation::validate(&spec).is_err());
        // Arrange
        spec.system_apps.cpu_monitor = Some(super::super::system_apps::CpuMonitor {
            image: "local-monitor".into(),
        });
        // Act / Assert
        assert!(preparation::validate(&spec).is_ok());
    }

    #[test]
    fn filesystem_policy_default_and_direct_mount_opt_out() {
        // Arrange
        let mut config = serde_json::json!({"id":"run", "repository_root":"/tmp", "artifacts":"/tmp/example", "projects":[{
            "key":"workload", "directory":"/tmp", "files":["compose.yaml"]
        }]});
        // Act / Assert
        let defaulted: ComposeRun = serde_json::from_value(config.clone()).unwrap();
        assert!(defaulted.filesystem.is_some());
        assert!(super::preparation::validate(&defaulted).is_ok());
        config["filesystem"] = serde_json::Value::Null;
        let direct: ComposeRun = serde_json::from_value(config).unwrap();
        assert!(direct.filesystem.is_none());
        assert!(super::preparation::validate(&direct).is_ok());
    }

    #[test]
    fn preflight_requires_complete_dora_runtime_paths() {
        // Arrange: registration loading permits a path supplied by the run owner.
        let mut spec: ComposeRun = serde_json::from_value(serde_json::json!({
            "id":"run", "repository_root":"/tmp", "artifacts":"/tmp/example", "projects":[{
                "key":"workload", "directory":"/tmp", "files":["compose.yaml"]
            }], "system_apps":{"dora_flows":[{
                "project":"workload", "service":"daemon", "name":"flow",
                "descriptor":"/opt/flow.yml", "shutdown":[{"nodes":["source"]}]
            }]}
        }))
        .unwrap();
        assert!(dora::validate(&spec.system_apps.dora_flows[0]).is_ok());

        // Act / Assert: runtime preflight rejects missing and relative paths.
        for path in ["", "relative/results"] {
            spec.system_apps.dora_flows[0].results_directory = path.into();
            assert!(preparation::validate(&spec).is_err());
        }

        // Act: the caller supplies the absolute results directory before preparation.
        spec.system_apps.dora_flows[0].results_directory = "/tmp/results".into();

        // Assert
        assert!(preparation::validate(&spec).is_ok());
    }
}
