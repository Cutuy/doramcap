//! Admission, pre-run reset and the report for one systemd run.
use crate::{Result, Scope, SystemdPlan, error, manager::Manager, running::SystemdRunning};
use orchestrator::{LocalExecutor, Observer};
use serde::{Deserialize, Serialize};
use std::{future::Future, os::unix::fs::DirBuilderExt, path::Path, rc::Rc, time::Duration};
use workload_lifecycle::recording::{FinalizeRecordings, Recording, WaitForRecordings};

/// How one unit ended, read while this run still referenced it.
/// Accounting fields are `None` when systemd did not report them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitExit {
    pub unit: String,
    /// systemd's `Result`, such as `success`, `exit-code`, `signal` or `timeout`.
    pub result: String,
    /// `CLD_*` code of the main process; zero when it never ran.
    pub exec_main_code: i32,
    pub exec_main_status: i32,
    pub cpu_usage_nsec: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
}
impl UnitExit {
    pub(crate) async fn read(unit: &crate::manager::Unit) -> Self {
        let service = &unit.service;
        // systemd reports unavailable accounting as u64::MAX.
        let known = |value: zbus::Result<u64>| value.ok().filter(|value| *value != u64::MAX);
        Self {
            unit: unit.name.clone(),
            result: service.result().await.unwrap_or_default(),
            exec_main_code: service.exec_main_code().await.unwrap_or_default(),
            exec_main_status: service.exec_main_status().await.unwrap_or_default(),
            cpu_usage_nsec: known(service.cpu_usage_n_sec().await),
            memory_peak_bytes: known(service.memory_peak().await),
        }
    }
}

/// Outcomes stay independent; inspect `successful()` before trusting evidence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemdRunReport {
    pub id: String,
    pub execution_error: Option<String>,
    pub writers_stopped: bool,
    pub recording_finalized: bool,
    pub cleanup_verified: bool,
    /// Every unit this run started, including ones that failed during the run.
    pub units: Vec<UnitExit>,
    pub diagnostics: Vec<String>,
}
impl SystemdRunReport {
    pub fn successful(&self) -> bool {
        self.execution_error.is_none()
            && self.writers_stopped
            && self.recording_finalized
            && self.cleanup_verified
            && self.diagnostics.is_empty()
    }
}

/// Runs one experiment per repository at a time on the system service manager.
/// An unverified removal blocks this runner until a later reset succeeds.
#[derive(Default)]
pub struct SystemdRunner {
    blocked: bool,
}
impl SystemdRunner {
    pub fn new() -> Self {
        Self::default()
    }
    /// Whether an unfinished run or unverified removal prevents admission.
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// Reset the repository's units, run `plan` with the shared `run` inputs until
    /// `stop`, then stop and remove.
    ///
    /// # Results and errors
    /// Invalid plans, a busy repository and a failed pre-run reset return `Err` before
    /// launch. Start, stop and removal failures enter `Ok(SystemdRunReport)`, which is
    /// also written to `report.json` in the new artifact directory.
    ///
    /// # Ownership and cancellation
    /// Owns `observer`. Signal `stop` and await completion; dropping the future leaves
    /// units running until the next run's reset and keeps this runner blocked.
    pub async fn run(
        &mut self,
        run: workload_lifecycle::launch::RunPlan,
        plan: SystemdPlan,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<SystemdRunReport> {
        crate::validate(&run, &plan)?;
        let scope = Scope::of(Path::new(&run.repository_root))?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&scope.directory)
            .map_err(error)?;
        let lock = std::fs::File::create(scope.directory.join("lock")).map_err(error)?;
        lock.try_lock()
            .map_err(|e| format!("another run owns this repository: {e}"))?;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&run.artifacts)
            .map_err(error)?;
        self.blocked = true;
        let manager = Rc::new(Manager::connect().await?);
        let deadline = Duration::from_secs(run.stop_seconds);
        let remaining = manager.reset(&scope, deadline).await?;
        if !remaining.is_empty() {
            return Err(format!("reset left units loaded: {remaining:?}"));
        }
        self.blocked = false;

        let mut executor = LocalExecutor::new(observer)
            .with_label(&run.id)
            .capture_payloads(true);
        executor.register::<WaitForRecordings>(|| WaitForRecordings);
        executor.register::<FinalizeRecordings>(|| FinalizeRecordings);
        let executor = Rc::new(executor);
        let recordings: Vec<Recording> = run
            .recordings
            .iter()
            .map(|r| Recording {
                stream: r.stream.clone().into(),
                recording: r.recording.clone().into(),
            })
            .collect();
        self.blocked = true;
        let running = SystemdRunning {
            manager,
            executor: executor.clone(),
            scope,
            run: run.clone(),
            plan,
            recordings: recordings.clone(),
            ready,
            groups: Vec::new(),
            diagnostics: Vec::new(),
        };
        let (outcome, removed) =
            workload_lifecycle::run::run(running, &executor, recordings, stop).await;
        let mut diagnostics = removed.diagnostics;
        diagnostics.extend(outcome.diagnostics);
        if executor.observer_failures() != 0 {
            diagnostics.push("workflow observer lost events".into());
        }
        let report = SystemdRunReport {
            id: run.id,
            execution_error: outcome.execution_error,
            writers_stopped: outcome.stopped.writers_stopped,
            recording_finalized: outcome.permit.recording_finalized,
            cleanup_verified: outcome.cleanup.verified,
            units: removed.units,
            diagnostics,
        };
        let bytes = serde_json::to_vec_pretty(&report).map_err(error)?;
        tokio::fs::write(Path::new(&run.artifacts).join("report.json"), bytes)
            .await
            .map_err(error)?;
        self.blocked = !report.cleanup_verified;
        Ok(report)
    }
}
