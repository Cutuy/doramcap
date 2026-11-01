//! Compose startup, observation and teardown policy expressed through shared steps.
mod execution;
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

/// Run one exclusive experiment per repository in a stable Compose project.
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
        // Reset the fixed ownership scope before preparing this run's immutable inputs.
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
        executor.register::<reset::ResetExperiment>(move || reset::ResetExperiment {
            adapter: backend.clone(),
            controller: reset_controller.clone(),
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
        if spec.filesystem.is_some() {
            overlay::verify_namespace(self.adapter.socket_path()).await?;
        }
        let artifacts = prepared.value().artifacts.clone();

        // Reserve the daemon and prepare optional filesystem views before application launch.
        self.blocked = true;
        self.adapter
            .project(&prepared.value().plan)
            .reserve(super::shared_dora::DAEMON, spec.readiness_seconds)
            .await?;
        let overlays = if let Some(policy) = &spec.filesystem {
            Some(
                executor
                    .run::<PrepareOverlays>(
                        Input::new(PrepareInput {
                            artifacts: artifacts.clone(),
                            workspace: controller.workspace(),
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
        // Race startup and observation against stop; teardown runs outside this boundary.
        let mut signals = signals;
        let mut last_launch = None;
        let mut applications_started = false;
        let execution_error = execution::until_stop(stop, events.failure(), async {
            execution::launch_apps(
                &executor,
                &spec,
                &prepared,
                &plan,
                overlays.as_ref(),
                &mut last_launch,
            )
            .await?;
            applications_started = true;
            // Give a concurrent stop request priority over publishing readiness.
            tokio::task::yield_now().await;
            signals.notify_ready(plan.clone());
            execution::monitor(&executor, &plan, &resources).await
        })
        .await;

        // Stop adopted apps, then system apps, then the shared Dora runtime.
        if let Some(quiescing) = signals.quiescing.take() {
            let _ = quiescing.send(());
        }
        let adopted_clean = execution::quiesce_services(
            &executor,
            &plan,
            &prepared.value().user_services,
            spec.stop_seconds,
            &diagnostics,
        )
        .await;
        let systems_stopped = resources
            .lock()
            .await
            .shutdown(&prepared.value().system_services)
            .await;
        if let Err(error) = &systems_stopped {
            execution::record_failure(&diagnostics, "system shutdown", error);
        }
        let systems_clean = execution::quiesce_services(
            &executor,
            &plan,
            &prepared.value().system_services,
            spec.stop_seconds,
            &diagnostics,
        )
        .await;
        let runtime_shutdown = executor.run::<super::experiment::StopDoraRuntime>(()).await;
        let runtime_stopped = runtime_shutdown
            .as_ref()
            .is_ok_and(|output| output.value().stopped);
        let runtime_exit_clean = runtime_shutdown
            .as_ref()
            .is_ok_and(|output| output.value().diagnostics.is_empty());
        match runtime_shutdown {
            Ok(stopped) => diagnostics
                .borrow_mut()
                .extend(stopped.into_value().diagnostics),
            Err(error) => execution::record_failure(&diagnostics, "Dora shutdown", error),
        }

        // Finalization and removal require stopped writers and durable filesystem evidence.
        let completion = async {
            let mut input = Input::new(StopInput {
                allow_volume_deletion: false,
                services: vec![],
                plan: Some(plan.clone()),
                stop_seconds: spec.stop_seconds,
            })
            .after(&prepared);
            if let Some(launched) = &last_launch {
                input = input.after(launched);
            }
            let shutdown = executor
                .run::<QuiesceCompose>(input)
                .await
                .map_err(step_error)?;
            diagnostics
                .borrow_mut()
                .extend(shutdown.value().diagnostics.clone());
            let mut writer_evidence = shutdown.value().clone();
            writer_evidence.stopped &= runtime_stopped;
            let finalized = executor
                .run::<FinalizeRun>(
                    Input::new(FinalizeInput {
                        id: spec.id.clone(),
                        artifacts: artifacts.clone(),
                        baseline: prepared.value().baseline.clone(),
                        overlays: overlays.map(|output| output.into_value()),
                        shutdown: writer_evidence,
                        project: plan.clone(),
                        recordings: spec.recordings.clone(),
                    })
                    .after(&prepared)
                    .after(&shutdown),
                )
                .await
                .map_err(step_error)?;
            if !finalized.value().recording_finalized {
                execution::record_failure(
                    &diagnostics,
                    "completion",
                    "recording finalization failed",
                );
            }
            let cleanup = if runtime_stopped && finalized.value().can_remove {
                match executor
                    .run::<RemoveCompose>(
                        Input::new(StopInput {
                            allow_volume_deletion: !finalized.value().preserve_volumes,
                            services: vec![],
                            plan: Some(plan.clone()),
                            stop_seconds: spec.stop_seconds,
                        })
                        .after(&shutdown)
                        .after(&finalized),
                    )
                    .await
                {
                    Ok(cleanup) => Some(cleanup.into_value()),
                    Err(error) => {
                        execution::record_failure(&diagnostics, "completion", error);
                        None
                    }
                }
            } else {
                None
            }
            .unwrap_or_else(|| Cleanup {
                project: plan.name.clone(),
                remaining: vec!["preserved: shutdown or filesystem evidence incomplete".into()],
                ..Default::default()
            });
            let finalized = finalized.into_value();
            let boundary_clean = runtime_stopped
                && finalized.can_remove
                && cleanup.verified
                && cleanup.diagnostics.is_empty()
                && (!finalized.preserve_volumes || cleanup.retained_volumes.is_empty());
            Ok::<_, Error>(ComposeRunReport {
                id: spec.id,
                execution_error: execution_error.clone(),
                diagnostics: Vec::new(),
                cleanup,
                boundary_clean,
                writers_stopped: shutdown.value().stopped && runtime_stopped,
                recording_finalized: finalized.recording_finalized
                    && runtime_exit_clean
                    && applications_started
                    && adopted_clean
                    && systems_stopped.is_ok()
                    && systems_clean
                    && execution_error.is_none(),
                event_stream_healthy: false,
                filesystem_changes: finalized.filesystem_changes,
                filesystem_results: finalized.filesystem_results,
            })
        }
        .await;

        // Finish event capture even when shutdown or finalization could not produce evidence.
        let event_result = events.finish().await;
        if let Err(error) = &event_result {
            execution::record_failure(&diagnostics, "completion", error);
        }
        if let Err(error) = &completion {
            execution::record_failure(&diagnostics, "completion", error);
        }
        if executor.observer_failures() != 0 {
            execution::record_failure(&diagnostics, "observer", "workflow observer lost events");
        }
        let mut report = completion?;
        report.event_stream_healthy = event_result.is_ok();
        let boundary_clean = report.boundary_clean;
        // Publish the final report before reopening admission for another experiment.
        let report = executor
            .run::<WriteRunReport>(Input::new(ReportInput { artifacts, report }).after(&prepared))
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
    /// Caller-selected run identifier used in Docker ownership labels and output names.
    pub id: String,
    /// Absolute, new artifact directory; its parent must already exist.
    pub artifacts: PathBuf,
    /// Repository root defining exclusive admission, reset scope and stable Compose identity.
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

    pub(super) fn ready_plan() -> ResolvedComposeProject {
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
