//! Compose startup, observation and teardown policy expressed through shared steps.
pub mod finalization;
mod observation;
pub mod preparation;
pub mod reset;

use self::{
    finalization::{FinalizeInput, FinalizeRun, ReportInput, WriteRunReport},
    observation::EventCapture,
    preparation::PrepareRun,
};
use super::dora::DoraRuntime;
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
pub use workload_lifecycle::recording::Recording;

/// Compose-specific notifications to an in-process listener; delivery never controls the run.
#[derive(Default)]
pub struct RunSignals {
    /// Notify after all projects are ready; a closed receiver is harmless.
    pub ready: Option<oneshot::Sender<ResolvedComposeProject>>,
    /// Notify when teardown begins, before stopping writers; delivery does not confirm shutdown.
    pub quiescing: Option<oneshot::Sender<()>>,
}

impl RunSignals {
    fn notify_ready(&mut self, plan: ResolvedComposeProject) {
        if let Some(ready) = self.ready.take() {
            let _ = ready.send(plan);
        }
    }
}

/// Run one machine-exclusive experiment in a stable repository Compose project.
/// Signal stop and await completion for cleanup; dropping the future leaves the
/// next run responsible for resetting owned Docker and upper-layer resources.
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
    /// An exclusive controller lease and reserved daemon name exclude overlapping
    /// experiments. Pre-run reset discards previous uppers without replaying writes.
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

    /// Notify after the experiment application stages become ready.
    /// A closed receiver is harmless. Await completion for cleanup; the resolved
    /// plan remains valid only during this run.
    pub async fn run_with_ready(
        &mut self,
        spec: ComposeRun,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<oneshot::Sender<ResolvedComposeProject>>,
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
        ready: Option<oneshot::Sender<ResolvedComposeProject>>,
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
        signals: RunSignals,
        check: impl FnOnce(&preparation::PreparedRun) -> Result<()>,
    ) -> Result<ComposeRunReport> {
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
        let backend = self.adapter.clone();
        let reset_controller = controller.clone();
        let stop_seconds = spec.stop_seconds;
        let mut executor =
            Rc::try_unwrap(executor).map_err(|_| invalid("executor already shared"))?;
        executor.register::<reset::ResetPreviousProject>(move || reset::ResetPreviousProject {
            adapter: backend.clone(),
            controller: reset_controller.clone(),
            stop_seconds,
        });
        executor.validate_steps().map_err(step_error)?;
        let executor = super::steps::attach(executor, self.adapter.clone(), diagnostics.clone());
        executor
            .run::<reset::ResetPreviousProject>(())
            .await
            .map_err(step_error)?;
        self.blocked = false;
        let prepared = executor
            .run::<PrepareRun>(spec.clone())
            .await
            .map_err(step_error)?;
        check(prepared.value())?;
        let artifacts = prepared.value().artifacts.clone();

        self.blocked = true;
        controller.save(&prepared.value().plan).await?;
        self.adapter
            .reserve(
                &prepared.value().plan,
                super::shared_dora::DAEMON,
                spec.readiness_seconds,
            )
            .await?;
        let overlays = if let Some(policy) = &spec.filesystem {
            Some(
                executor
                    .run::<PrepareOverlays>(
                        Input::new(PrepareInput {
                            artifacts: artifacts.clone(),
                            projects: vec![prepared.value().plan.clone()],
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
        let plan = prepared
            .value()
            .execution_plan(overlays.as_ref().map(|o| o.value()));
        let resources = Rc::new(tokio::sync::Mutex::new(
            super::experiment::ExperimentOwner::new(
                prepared.value().dora.clone(),
                self.adapter.clone(),
                plan.clone(),
                spec.readiness_seconds,
                spec.stop_seconds,
            ),
        ));
        let executor = super::steps::bind_experiment(
            executor,
            self.adapter.clone(),
            diagnostics.clone(),
            resources.clone(),
        )?;
        let events = EventCapture::start(
            self.adapter.docker.clone(),
            spec.id.clone(),
            plan.name.clone(),
            artifacts.join("docker-events.jsonl"),
        );
        let mut phases = ComposePhases {
            executor: executor.clone(),
            spec: spec.clone(),
            artifacts: artifacts.clone(),
            prepared,
            overlays,
            plan,
            resources,
            diagnostics: diagnostics.clone(),
            consumers_ready: false,
            dora_clean: true,
            dora_finalized: false,
            events: Some(events),
            last_launch: None,
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
            .map(|o| o.value().clone())
            .unwrap_or_else(|| Cleanup {
                project: phases.plan.name.clone(),
                remaining: vec!["preserved: shutdown or filesystem evidence incomplete".into()],
                ..Default::default()
            });
        let shutdown = phases
            .shutdown
            .as_ref()
            .ok_or_else(|| invalid("shutdown evidence absent; manager blocked"))?;
        let event_result = &phases.event_result;
        let finalized = finalized.value();
        let boundary_clean = phases.dora_clean
            && finalized.can_remove
            && cleanup.verified
            && cleanup.diagnostics.is_empty()
            && (!finalized.preserve_volumes || cleanup.retained_volumes.is_empty());
        let report = executor
            .run::<WriteRunReport>(
                Input::new(ReportInput {
                    artifacts,
                    report: ComposeRunReport {
                        id: spec.id,
                        execution_error: outcome.execution_error.clone(),
                        diagnostics: Vec::new(),
                        cleanup: cleanup.clone(),
                        boundary_clean,
                        writers_stopped: shutdown.value().stopped && phases.dora_clean,
                        recording_finalized: finalized.recording_finalized
                            && phases.dora_finalized
                            && outcome.execution_error.is_none(),
                        event_stream_healthy: event_result.is_ok(),
                        filesystem_changes: finalized.filesystem_changes.clone(),
                        filesystem_results: finalized.filesystem_results.clone(),
                    },
                })
                .after(&phases.prepared),
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
    plan: ResolvedComposeProject,
    resources: super::experiment::SharedExperiment,
    diagnostics: Diagnostics,
    consumers_ready: bool,
    dora_clean: bool,
    dora_finalized: bool,
    events: Option<EventCapture>,
    last_launch: Option<orchestrator::Output<ComposeProjectState>>,
    shutdown: Option<orchestrator::Output<Shutdown>>,
    finalized: Option<orchestrator::Output<finalization::FinalizedRun>>,
    cleanup: Option<orchestrator::Output<Cleanup>>,
    signals: RunSignals,

    event_result: Result<()>,
}
impl ComposePhases {
    async fn start_services(
        &mut self,
        services: Vec<String>,
        graph: &orchestrator::Output<()>,
    ) -> orchestrator::Result<()> {
        let mut input = Input::new(StartInput {
            plan: Some(self.plan.clone()),
            services,
            readiness_seconds: self.spec.readiness_seconds,
        })
        .after(graph);
        if let Some(overlays) = &self.overlays {
            input = input.after(overlays);
        }
        if let Some(previous) = &self.last_launch {
            input = input.after(previous);
        }
        self.last_launch = Some(self.executor.run::<StartCompose>(input).await?);
        Ok(())
    }

    async fn start_system_apps(
        &mut self,
        graph: &orchestrator::Output<()>,
    ) -> orchestrator::Result<()> {
        let system = self.prepared.value().system_services.clone();
        if system.is_empty() {
            return Ok(());
        }
        self.start_services(system.clone(), graph).await?;
        self.executor
            .run::<workload_lifecycle::recording::WaitForRecordings>(
                workload_lifecycle::recording::ReadinessInput {
                    recordings: self.spec.recordings.clone(),
                    timeout_seconds: self.spec.readiness_seconds,
                },
            )
            .await?;
        if !self.prepared.value().user_services.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let mut state = self
                .executor
                .run::<InspectCompose>(self.plan.clone())
                .await?
                .into_value();
            let Some(plan) = &mut state.plan else {
                return Err(orchestrator::Error::message("Compose state plan missing"));
            };
            plan.services.retain(|name, _| system.contains(name));
            if !DockerAdapter::ready(&state)
                .map_err(|e| orchestrator::Error::message(e.to_string()))?
            {
                return Err(orchestrator::Error::message("system apps lost readiness"));
            }
        }
        Ok(())
    }

    async fn launch_apps(&mut self) -> orchestrator::Result<()> {
        let daemon = self
            .executor
            .run::<super::experiment::StartDoraRuntime>(Input::new(()).after(&self.prepared))
            .await?;
        let graph = self
            .executor
            .run::<super::experiment::RegisterDoraGraph>(Input::new(()).after(&daemon))
            .await?;
        self.start_system_apps(&graph).await?;
        let adopted = self.prepared.value().user_services.clone();
        if !adopted.is_empty() {
            self.start_services(adopted, &graph).await?;
        }
        self.consumers_ready = true;
        Ok(())
    }

    async fn quiesce_services(&self, services: &[String]) -> bool {
        if services.is_empty() {
            return true;
        }
        let Ok(stopped) = self
            .executor
            .run::<QuiesceCompose>(StopInput {
                plan: Some(self.plan.clone()),
                services: services.to_vec(),
                allow_volume_deletion: false,
                stop_seconds: self.spec.stop_seconds,
            })
            .await
        else {
            return false;
        };
        let stopped = stopped.into_value();
        let clean = stopped.stopped && stopped.diagnostics.is_empty();
        self.diagnostics.borrow_mut().extend(stopped.diagnostics);
        clean
    }
}
impl workload_lifecycle::recorded_run::Phases for ComposePhases {
    async fn launch(&mut self) -> std::result::Result<(), String> {
        let failure = self.events.as_ref().unwrap().failure();
        tokio::select! {
            result = self.launch_apps() => result.map_err(|e| e.to_string()),
            error = failure => Err(error),
        }
    }
    fn ready(&mut self) -> std::result::Result<(), String> {
        self.signals.notify_ready(self.plan.clone());

        Ok(())
    }
    async fn inspect(&mut self) -> std::result::Result<(), String> {
        let inspect = async {
            let state = self
                .executor
                .run::<InspectCompose>(self.plan.clone())
                .await?;
            if !DockerAdapter::ready(state.value())
                .map_err(|e| orchestrator::Error::message(e.to_string()))?
            {
                return Err(orchestrator::Error::message("experiment lost readiness"));
            }
            self.resources
                .lock()
                .await
                .inspect()
                .await
                .map_err(|error| orchestrator::Error::message(error.to_string()))?;
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
        let resources = &self.resources;
        let executor = &self.executor;
        let diagnostics = &self.diagnostics;
        let dora_clean;
        let mut dora_finalized = self.consumers_ready;
        dora_finalized &= self
            .quiesce_services(&self.prepared.value().user_services)
            .await;
        {
            let drained = resources
                .lock()
                .await
                .shutdown(&self.prepared.value().system_services)
                .await;
            match drained {
                Ok(()) => {}
                Err(error) => {
                    dora_finalized = false;
                    diagnostics
                        .borrow_mut()
                        .push(error.diagnostic("workload drain"));
                }
            }
            dora_finalized &= self
                .quiesce_services(&self.prepared.value().system_services)
                .await;
            match executor.run::<super::experiment::StopDoraRuntime>(()).await {
                Ok(stopped) => {
                    dora_clean = stopped.value().stopped;
                    dora_finalized &= stopped.value().diagnostics.is_empty();
                    diagnostics
                        .borrow_mut()
                        .extend(stopped.value().diagnostics.clone());
                }
                Err(error) => {
                    dora_clean = false;
                    dora_finalized = false;
                    diagnostics
                        .borrow_mut()
                        .push(invalid(error.to_string()).diagnostic("Dora shutdown"));
                }
            }
        }
        self.dora_clean = dora_clean;
        self.dora_finalized = dora_finalized;
        // Stop all writer roles before collecting filesystems or removing infrastructure.
        let mut input = Input::new(StopInput {
            allow_volume_deletion: false,
            services: vec![],
            plan: Some(self.plan.clone()),
            stop_seconds: self.spec.stop_seconds,
        })
        .after(&self.prepared);
        if let Some(launched) = &self.last_launch {
            input = input.after(launched);
        }
        let shutdown = self
            .executor
            .run::<QuiesceCompose>(input)
            .await
            .map_err(|e| e.to_string())?;
        self.diagnostics
            .borrow_mut()
            .extend(shutdown.value().diagnostics.clone());
        let stopped = shutdown.value().stopped;
        self.shutdown = Some(shutdown);
        Ok(stopped)
    }
    async fn finalize(&mut self, _writers_stopped: bool) -> std::result::Result<(), String> {
        let mut shutdown = self
            .shutdown
            .as_ref()
            .ok_or("quiescence evidence absent")?
            .value()
            .clone();
        shutdown.stopped &= self.dora_clean;
        let finalized = self
            .executor
            .run::<FinalizeRun>(
                Input::new(FinalizeInput {
                    id: self.spec.id.clone(),
                    artifacts: self.artifacts.clone(),
                    baseline: self.prepared.value().baseline.clone(),
                    overlays: self.overlays.as_ref().map(|o| o.value().clone()),
                    shutdown,
                    project: self.plan.clone(),
                    recordings: self.spec.recordings.clone(),
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
        let finalized = self
            .finalized
            .as_ref()
            .ok_or("finalization evidence absent; resources retained")?;
        if !self.dora_clean || !finalized.value().can_remove {
            return Ok(false);
        }
        let cleanup = self
            .executor
            .run::<RemoveCompose>(
                Input::new(StopInput {
                    allow_volume_deletion: !finalized.value().preserve_volumes,
                    services: vec![],
                    plan: Some(self.plan.clone()),
                    stop_seconds: self.spec.stop_seconds,
                })
                .after(self.shutdown.as_ref().ok_or("shutdown evidence absent")?)
                .after(finalized),
            )
            .await
            .map_err(|e| e.to_string())?;
        let verified = cleanup.value().verified;
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
    /// Caller-selected run identifier used in Docker ownership labels and output names.
    pub id: String,
    /// Absolute, new artifact directory; its parent must already exist.
    pub artifacts: PathBuf,
    /// Explicit repository root used for the stable Compose project identity.
    pub repository_root: PathBuf,
    /// Adopted Compose app inputs whose services join the experiment project.
    pub projects: Vec<ComposeProject>,
    /// Ordinary system app inputs, started and stabilized before adopted apps.
    /// Their services stop after the adopted apps.
    #[serde(default)]
    pub system_apps: Vec<ComposeProject>,
    /// Communication setup; application containers remain ordinary Compose inputs.
    #[serde(default)]
    pub dora: DoraRuntime,
    /// Explicit ordinary app recordings, checked after writers stop.
    #[serde(default)]
    pub recordings: Vec<Recording>,
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
    /// Experiment resource-removal evidence.
    pub cleanup: Cleanup,
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
            && self.cleanup.diagnostics.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_plan() -> ResolvedComposeProject {
        ResolvedComposeProject {
            run: "run".into(),
            key: "experiment".into(),
            name: "experiment".into(),
            manifest: "/tmp/manifest".into(),
            sha256: String::new(),
            services: Default::default(),
            diagnostics: vec![],
            remove_volumes: false,
        }
    }

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
        signals.notify_ready(ready_plan());

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
        signals.notify_ready(ready_plan());

        // Assert
        assert_eq!(receiver.try_recv().unwrap().key, "experiment");
        assert!(signals.ready.is_none());

        // Act / Assert: repeated notification is harmless.
        signals.notify_ready(ready_plan());
    }
}
