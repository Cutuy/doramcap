//! Compose startup, observation and teardown policy expressed through shared steps.
pub mod finalization;
mod observation;
pub mod preparation;
pub mod projects;
pub mod resolution;

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

/// Optional typed notifications owned by one run.
#[derive(Default)]
pub struct RunSignals {
    /// Send ready plans; a closed receiver triggers orderly teardown.
    pub ready: Option<oneshot::Sender<Vec<ResolvedComposeProject>>>,
    /// Signal before stopping writers; a closed receiver does not affect cleanup.
    pub quiescing: Option<oneshot::Sender<()>>,
}

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

    /// Run with typed readiness and quiescence notifications plus preflight validation.
    /// Quiescence is signaled before stopping writers, including automatic shutdown.
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
        signals: RunSignals,
        check: impl FnOnce(&preparation::PreparedRun) -> Result<()>,
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
        executor.validate_descriptors().map_err(step_error)?;
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
        let flows = dora::bind(&spec.system_apps.dora_flows, &plans)?;
        let events = EventCapture::start(
            self.adapter.docker.clone(),
            spec.id.clone(),
            artifacts.join("docker-events.jsonl"),
        );
        let mut phases = ComposePhases {
            executor: executor.clone(),
            spec: spec.clone(),
            artifacts: artifacts.clone(),
            prepared,
            overlays,
            groups,
            plans,
            flows,
            events: Some(events),
            attempted: Vec::new(),
            launches: Vec::new(),
            flow_start: None,
            shutdown: None,
            finalized: None,
            cleanup: None,
            signals,
            event_result: Ok(()),
        };
        let outcome = workload_lifecycle::recorded_run::run(&mut phases, stop).await;
        for error in &outcome.completion_errors {
            diagnostics
                .borrow_mut()
                .push(invalid(error.clone()).diagnostic("completion"));
        }
        if executor.observer_failures() != 0 {
            diagnostics
                .borrow_mut()
                .push(invalid("workflow observer lost events").diagnostic("observer"));
        }
        let finalized = phases
            .finalized
            .as_ref()
            .ok_or_else(|| invalid("finalization evidence absent; manager blocked"))?;
        let cleanup = phases
            .cleanup
            .as_ref()
            .ok_or_else(|| invalid("cleanup evidence absent; manager blocked"))?;
        let shutdown = phases
            .shutdown
            .as_ref()
            .ok_or_else(|| invalid("shutdown evidence absent; manager blocked"))?;
        let event_result = &phases.event_result;
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
                        execution_error: outcome.execution_error,
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

struct ComposePhases {
    executor: Rc<orchestrator::LocalExecutor>,
    spec: ComposeRun,
    artifacts: PathBuf,
    prepared: orchestrator::Output<preparation::PreparedRun>,
    overlays: Option<orchestrator::Output<crate::compose::overlay::OverlaySet>>,
    groups: Vec<projects::LaunchInput>,
    plans: Vec<ResolvedComposeProject>,
    flows: Vec<dora::Flow>,
    events: Option<EventCapture>,
    attempted: Vec<Vec<ResolvedComposeProject>>,
    launches: Vec<orchestrator::Output<()>>,
    flow_start: Option<orchestrator::Output<Vec<dora::Flow>>>,
    shutdown: Option<orchestrator::Output<projects::QuiescedProjects>>,
    finalized: Option<orchestrator::Output<finalization::FinalizedRun>>,
    cleanup: Option<orchestrator::Output<Vec<Cleanup>>>,
    signals: RunSignals,
    event_result: Result<()>,
}
impl workload_lifecycle::recorded_run::Phases for ComposePhases {
    async fn launch(&mut self) -> std::result::Result<(), String> {
        let events = self.events.as_mut().unwrap();
        let launch = async {
            // Supporting groups precede workloads. Siblings within a group run concurrently.
            for group in &self.groups {
                self.attempted.push(group.projects.clone());
                let mut input = Input::new(group.clone()).after(&self.prepared);
                if let Some(overlays) = &self.overlays {
                    input = input.after(overlays);
                }
                if let Some(previous) = self.launches.last() {
                    input = input.after(previous);
                }
                self.launches
                    .push(self.executor.run::<LaunchProjects>(input).await?);
            }
            let mut input = Input::new(LaunchFlowsInput {
                flows: self.flows.clone(),
                readiness_seconds: self.spec.readiness_seconds,
            })
            .after(&self.prepared);
            if let Some(launched) = self.launches.last() {
                input = input.after(launched);
            }
            let started = self.executor.run::<LaunchDoraFlows>(input).await?;
            self.flows = started.value().clone();
            self.flow_start = Some(started);
            Ok::<(), orchestrator::Error>(())
        };
        tokio::select! {
            result = launch => result.map_err(|e| e.to_string()),
            error = events.failure() => Err(error),
        }
    }
    fn ready(&mut self) -> std::result::Result<(), String> {
        if let Some(ready) = self.signals.ready.take() {
            ready
                .send(self.plans.clone())
                .map_err(|_| "readiness receiver dropped".to_string())?;
        }
        Ok(())
    }
    async fn inspect(&mut self) -> std::result::Result<(), String> {
        let inspect = async {
            let mut input = Input::new(ObserveInput {
                projects: self.plans.clone(),
                flows: self.flows.clone(),
            })
            .after(&self.prepared);
            if let Some(launched) = self.launches.last() {
                input = input.after(launched);
            }
            if let Some(started) = &self.flow_start {
                input = input.after(started);
            }
            self.executor.run::<ObserveProjects>(input).await?;
            Ok::<(), orchestrator::Error>(())
        };
        tokio::select! {
            result = inspect => result.map_err(|e| e.to_string()),
            error = self.events.as_mut().unwrap().failure() => Err(error),
        }
    }
    async fn quiesce(&mut self) -> std::result::Result<bool, String> {
        if let Some(quiescing) = self.signals.quiescing.take() {
            let _ = quiescing.send(());
        }
        // Stop all writer roles before collecting filesystems or removing infrastructure.
        let mut input = Input::new(QuiesceInput {
            groups: self.attempted.clone(),
            flows: self.flows.clone(),
            stop_seconds: self.spec.stop_seconds,
        })
        .after(&self.prepared);
        for launched in &self.launches {
            input = input.after(launched);
        }
        if let Some(started) = &self.flow_start {
            input = input.after(started);
        }
        let shutdown = self
            .executor
            .run::<QuiesceProjects>(input)
            .await
            .map_err(|e| e.to_string())?;
        let stopped = shutdown.value().writers_stopped;
        self.shutdown = Some(shutdown);
        Ok(stopped)
    }
    async fn finalize(&mut self, _writers_stopped: bool) -> std::result::Result<(), String> {
        let finalized = self
            .executor
            .run::<FinalizeRun>(
                Input::new(FinalizeInput {
                    id: self.spec.id.clone(),
                    artifacts: self.artifacts.clone(),
                    baseline: self.prepared.value().baseline.clone(),
                    overlays: self.overlays.as_ref().map(|o| o.value().clone()),
                    shutdown: self
                        .shutdown
                        .as_ref()
                        .ok_or("quiescence evidence absent")?
                        .value()
                        .clone(),
                    recordings: self
                        .groups
                        .iter()
                        .flat_map(|g| g.recordings.clone())
                        .collect(),
                    require_volume_evidence: self.spec.recovery_anchor_dir.is_some(),
                })
                .after(&self.prepared)
                .after(self.shutdown.as_ref().ok_or("quiescence evidence absent")?),
            )
            .await
            .map_err(|e| e.to_string())?;
        let valid = finalized.value().recording_finalized;
        self.finalized = Some(finalized);
        if valid {
            Ok(())
        } else {
            Err("recording finalization failed".into())
        }
    }
    async fn remove(&mut self) -> std::result::Result<bool, String> {
        let cleanup = self
            .executor
            .run::<RemoveProjects>(
                Input::new(RemoveInput {
                    projects: self
                        .shutdown
                        .as_ref()
                        .ok_or("quiescence evidence absent; resources retained")?
                        .value()
                        .projects
                        .clone(),
                    stop_seconds: self.spec.stop_seconds,
                    can_remove: self
                        .finalized
                        .as_ref()
                        .ok_or("finalization evidence absent; resources retained")?
                        .value()
                        .can_remove,
                    preserve_volumes: self
                        .finalized
                        .as_ref()
                        .ok_or("finalization evidence absent; resources retained")?
                        .value()
                        .preserve_volumes,
                })
                .after(self.shutdown.as_ref().unwrap())
                .after(self.finalized.as_ref().unwrap()),
            )
            .await
            .map_err(|e| e.to_string())?;

        let verified = cleanup.value().iter().all(|c| c.verified);
        self.cleanup = Some(cleanup);
        Ok(verified)
    }
    async fn finish(&mut self) -> std::result::Result<(), String> {
        self.event_result = self.events.take().unwrap().finish().await;
        self.event_result
            .as_ref()
            .map(|_| ())
            .map_err(|e| e.to_string())
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
    fn supporting_apps_allow_monitor_only_and_empty_runs_reject() {
        // Arrange
        let mut spec: ComposeRun = serde_json::from_value(serde_json::json!({
            "id":"monitor", "artifacts":"/tmp/new-monitor-run", "projects":[], "filesystem":null
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
        let mut config = serde_json::json!({"id":"run", "artifacts":"/tmp/example", "projects":[{
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
            "id":"run", "artifacts":"/tmp/example", "projects":[{
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
