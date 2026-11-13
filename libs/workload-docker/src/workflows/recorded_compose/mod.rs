//! Compose startup, observation and teardown policy expressed through shared steps.
use host_filesystem::{
    changes::FilesystemChanges,
    overlay::{OverlayPolicy, OverlayResults},
};
mod execution;
pub mod finalization;
pub mod observation;
pub mod reset;
mod running;
mod plan {
    include!(concat!(env!("OUT_DIR"), "/doramcap.compose_plan.v1.rs"));
}
pub use plan::ComposePlan;

use self::{
    finalization::{ReportInput, WriteRunReport},
    observation::EventCapture,
};
use compose_apps::compose::{ComposePrepareInput, PrepareOverlays, error::invalid, *};
use compose_apps::{
    cgroup,
    preparation::{self, McapWriters, PrepareInput, PrepareRun, validate_budgets},
};
use orchestrator::{Input, Observer};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, future::Future, path::PathBuf, rc::Rc, time::Duration};
use system_config::MachineState;
use tokio::sync::{oneshot, watch};
pub use workload_lifecycle::recording::Recording;

/// Compose-specific notifications to an in-process listener; delivery never controls the run.
#[derive(Default)]
pub struct RunSignals {
    /// Notify after all projects are ready; a closed receiver is harmless.
    pub ready: Option<oneshot::Sender<ResolvedComposeProject>>,
    /// Notify when teardown begins, before stopping writers; delivery does not confirm shutdown.
    pub quiescing: Option<oneshot::Sender<()>>,
    /// Publishes the services given a stop signal and not yet exited.
    pub stopping: watch::Sender<Vec<StoppingService>>,
    /// Publishes this run's machine states as each is captured.
    pub machine: watch::Sender<MachineCapture>,
}

/// Before startup and after shutdown: None is pending, Some carries the observation result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MachineCapture {
    pub before: Option<std::result::Result<MachineState, String>>,
    pub after: Option<std::result::Result<MachineState, String>>,
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
        // Validate before reset can affect a previous run.
        workload_lifecycle::plan::validate_host_shm_access(spec.host_shm_access)
            .map_err(invalid)?;
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
            .run::<PrepareRun>(PrepareInput::try_from(spec.clone())?)
            .await
            .map_err(step_error)?;
        let before = observation::machine_state(&self.adapter, &prepared.value().artifacts).await;
        signals
            .machine
            .send_modify(|capture| capture.before = Some(before));
        check(prepared.value())?;
        let system_apps = prepared.value().system_cgroup.clone().map(|placement| {
            cgroup::DockerCgroup::group(placement, Duration::from_secs(stop_seconds))
        });
        let mut executor = Rc::try_unwrap(executor)
            .map_err(|_| invalid("executor still running during cgroup binding"))?;
        if let Some(group) = &system_apps {
            workload_lifecycle::cgroup::register(&mut executor, group.clone());
        }
        let executor = super::steps::attach(executor, self.adapter.clone(), diagnostics.clone());
        self.blocked = false;
        if spec.filesystem.is_some() {
            docker_engine::namespace::verify_namespace(self.adapter.socket_path()).await?;
        }
        let artifacts = prepared.value().artifacts.clone();

        // Reserve the daemon and prepare optional filesystem views before application launch.
        self.blocked = true;
        if prepared.value().dora.enabled() {
            self.adapter
                .project(&prepared.value().plan)
                .reserve(prepared.value().dora.daemon()?.0, spec.readiness_seconds)
                .await?;
        }
        let overlays = if let Some(policy) = &spec.filesystem {
            Some(
                executor
                    .run::<PrepareOverlays>(
                        Input::new(ComposePrepareInput {
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
            compose_apps::experiment::ExperimentOwner::new(
                prepared.value().dora.clone(),
                prepared.value().run.clone(),
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
            self.adapter.engine.client().clone(),
            plan.name.clone(),
            artifacts.join(EVENTS_RECORD),
            signals.stopping.clone(),
        );
        let machine = signals.machine.clone();
        let running = running::ComposeRunning {
            system_apps,
            adapter: self.adapter.clone(),
            executor: executor.clone(),
            spec: spec.clone(),
            prepared: prepared.clone(),
            plan: plan.clone(),
            overlays,
            resources,
            events,
            signals,
            diagnostics: diagnostics.clone(),
            last_launch: None,
            applications_started: false,
            stopped: None,
            stop_error: None,
        };
        // Start, observe until stop, stop in reverse, check recordings, then remove.
        let (outcome, removed) =
            workload_lifecycle::run::run(running, &executor, spec.recordings.clone(), stop).await;
        for failure in &outcome.diagnostics {
            execution::record_failure(&diagnostics, "completion", failure);
        }
        let execution_error = outcome.execution_error;
        let completion = removed.completion.map(|completed| {
            if !outcome.permit.recording_finalized {
                execution::record_failure(
                    &diagnostics,
                    "completion",
                    "recording finalization failed",
                );
            }
            ComposeRunReport {
                id: spec.id,
                execution_error: execution_error.clone(),
                diagnostics: Vec::new(),
                cleanup: completed.cleanup,
                boundary_clean: completed.boundary_clean,
                writers_stopped: completed.writers_stopped,
                recording_finalized: outcome.permit.recording_finalized
                    && completed.runtime_exit_clean
                    && completed.applications_started
                    && completed.apps_clean
                    && execution_error.is_none(),
                service_exits: completed.service_exits,
                event_stream_healthy: false,
                filesystem_changes: completed.finalized.filesystem_changes,
                filesystem_results: completed.finalized.filesystem_results,
            }
        });
        if let Err(error) = &removed.event_stream {
            execution::record_failure(&diagnostics, "completion", error);
        }
        if executor.observer_failures() != 0 {
            execution::record_failure(&diagnostics, "observer", "workflow observer lost events");
        }
        let after = observation::machine_state(&self.adapter, &artifacts).await;
        machine.send_modify(|capture| capture.after = Some(after));
        let mut report = completion?;
        report.event_stream_healthy = removed.event_stream.is_ok();
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

/// Run-relative Docker event capture record.
const EVENTS_RECORD: &str = "docker-events.jsonl";

fn step_error(error: impl std::fmt::Display) -> Error {
    invalid(error.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeRun {
    /// Caller-owned run-relative records excluded from application output listings.
    #[serde(default)]
    pub private_records: Vec<String>,
    /// Some isolates writable directory binds through OverlayFS. None uses the
    /// supplied mounts directly. Omitted JSON retains the default write-back policy.
    #[serde(default = "default_filesystem")]
    pub filesystem: Option<OverlayPolicy>,
    /// Required host shm access policy; zero and unknown values fail preparation.
    #[serde(default)]
    pub host_shm_access: i32,
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
    /// Ordinary app inputs launched before system and user apps.
    #[serde(default)]
    pub dora_apps: Vec<ComposeProject>,
    /// Absolute merged dataflow file; empty registers none.
    #[serde(default)]
    pub dataflow_file: String,
    /// Explicit ordinary app recordings, checked after writers stop.
    #[serde(default)]
    pub recordings: Vec<Recording>,
    /// Services using the io_uring MCAP writer; each gets `seccomp=unconfined` and memlock.
    #[serde(default)]
    pub mcap_writers: McapWriters,
    /// Positive startup/readiness budget in seconds; default 60.
    #[serde(default = "readiness_default")]
    pub readiness_seconds: u64,
    /// Stop grace period in seconds for services declaring no `stop_grace_period`,
    /// from 1 through 300; default 15. Declared periods apply as written.
    #[serde(default = "stop_default")]
    pub stop_seconds: u64,
}
impl ComposeRun {
    fn run_plan(&self) -> Result<workload_lifecycle::launch::RunPlan> {
        Ok(workload_lifecycle::launch::RunPlan {
            private_records: self.private_records.clone(),
            id: self.id.clone(),
            repository_root: self
                .repository_root
                .to_str()
                .ok_or_else(|| invalid("repository path is not UTF-8"))?
                .into(),
            artifacts: self
                .artifacts
                .to_str()
                .ok_or_else(|| invalid("artifact path is not UTF-8"))?
                .into(),
            recordings: self
                .recordings
                .iter()
                .map(|recording| workload_lifecycle::launch::Recording {
                    stream: recording.stream.to_string_lossy().into_owned(),
                    recording: recording.recording.to_string_lossy().into_owned(),
                })
                .collect(),
            readiness_seconds: self.readiness_seconds,
            stop_seconds: self.stop_seconds,
            system_apps: self.system_apps.iter().map(|app| app.key.clone()).collect(),
            mcap_apps: self.mcap_writers.apps.clone(),
            dora_apps: self.dora_apps.iter().map(|app| app.key.clone()).collect(),
            dataflow_file: self.dataflow_file.clone(),
            dora_runtime: None,
            host_shm_access: self.host_shm_access,
        })
    }

    /// Build launch settings from the shared run inputs and the Compose plan, with
    /// default OverlayFS isolation. MCAP writers write beneath the artifact directory,
    /// on its parent's filesystem.
    pub fn from_plan(run: workload_lifecycle::launch::RunPlan, plan: ComposePlan) -> Result<Self> {
        workload_lifecycle::plan::validate_apps(
            &run,
            plan.projects.iter().map(|app| app.key.as_str()),
        )
        .map_err(invalid)?;
        let (dora_apps, remaining): (Vec<_>, Vec<_>) = plan
            .projects
            .into_iter()
            .partition(|app| run.dora_apps.contains(&app.key));
        let (system_apps, projects) = remaining
            .into_iter()
            .partition(|app| run.system_apps.contains(&app.key));
        let artifacts = PathBuf::from(run.artifacts);
        Ok(Self {
            private_records: run.private_records,
            id: run.id,
            repository_root: run.repository_root.into(),
            mcap_writers: McapWriters {
                services: Vec::new(),
                apps: run.mcap_apps,
                directories: artifacts.parent().map(PathBuf::from).into_iter().collect(),
            },
            artifacts,
            filesystem: Some(Default::default()),
            host_shm_access: run.host_shm_access,
            system_apps,
            projects,
            dora_apps,
            dataflow_file: run.dataflow_file,
            recordings: run
                .recordings
                .into_iter()
                .map(|r| Recording {
                    stream: r.stream.into(),
                    recording: r.recording.into(),
                })
                .collect(),
            readiness_seconds: run.readiness_seconds,
            stop_seconds: run.stop_seconds,
        })
    }
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

impl TryFrom<ComposeRun> for PrepareInput {
    type Error = Error;

    /// Build shared preparation inputs, rejecting paths that cannot cross the launch contract.
    fn try_from(spec: ComposeRun) -> Result<Self> {
        Ok(Self {
            run_plan: {
                let mut run = spec.run_plan()?;
                run.private_records
                    .extend([EVENTS_RECORD.into(), finalization::REPORT_RECORD.into()]);
                run
            },
            dora_apps: spec.dora_apps,
            system_apps: spec.system_apps,
            extra_apps: spec.projects,
            filesystem: spec.filesystem,
            mcap_writers: spec.mcap_writers,
        })
    }
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
    /// How each workload, system app and Dora service ended at stop.
    #[serde(default)]
    pub service_exits: Vec<compose_apps::compose::ServiceExit>,
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
    fn compose_plan_maps_shared_roles_and_dataflow() {
        // Arrange
        use workload_lifecycle::launch::RunPlan;
        let run = RunPlan {
            host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
            id: "r1".into(),
            repository_root: "/repo".into(),
            artifacts: "/data/r1".into(),
            system_apps: vec!["monitor".into()],
            mcap_apps: vec!["monitor".into()],
            readiness_seconds: 60,
            stop_seconds: 15,
            dora_apps: vec!["communication".into()],
            dataflow_file: "/graph.yml".into(),
            ..Default::default()
        };
        let plan = ComposePlan {
            projects: ["user", "monitor", "communication"]
                .into_iter()
                .map(|key| ComposeProject {
                    key: key.into(),
                    ..Default::default()
                })
                .collect(),
        };
        // Act
        let mapped = ComposeRun::from_plan(run.clone(), plan.clone()).unwrap();
        // Assert
        assert_eq!(mapped.system_apps[0].key, "monitor");
        assert_eq!(mapped.projects[0].key, "user");
        assert_eq!(mapped.mcap_writers.apps, ["monitor"]);
        assert_eq!(mapped.mcap_writers.directories, [PathBuf::from("/data")]);
        assert_eq!(mapped.dataflow_file, "/graph.yml");
        assert_eq!(mapped.host_shm_access, run.host_shm_access);
        assert_eq!(mapped.dora_apps[0].key, "communication");
        assert_eq!((mapped.readiness_seconds, mapped.stop_seconds), (60, 15));
        // Act / Assert: roles must refer to an actual app.
        let mut unknown = run;
        unknown.system_apps.push("absent".into());
        assert!(ComposeRun::from_plan(unknown, plan).is_err());
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
