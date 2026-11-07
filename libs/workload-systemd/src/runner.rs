//! Admission, explicit cleanup and the report for one systemd run.
use crate::{Result, Scope, SystemdPlan, error, manager::Manager, running::SystemdRunning};
use orchestrator::{LocalExecutor, Observer};
use serde::{Deserialize, Serialize};
use std::{future::Future, os::unix::fs::DirBuilderExt, path::Path, rc::Rc};
use workload_lifecycle::recording::{FinalizeRecordings, Recording, WaitForRecordings};

/// How one unit ended, read while this run still referenced it.
/// Accounting fields are `None` when systemd did not report them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitExit {
    pub unit: String,
    /// Program ID or adopted unit stem: the unit name without its repository prefix.
    pub id: String,
    /// systemd's `Result`, such as `success`, `exit-code`, `signal` or `timeout`.
    pub result: Option<String>,
    /// `CLD_*` code of the main process; zero when it never ran.
    pub exec_main_code: Option<i32>,
    pub exec_main_status: Option<i32>,
    pub cpu_usage_nsec: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
}
impl UnitExit {
    fn complete(&self) -> bool {
        self.result.is_some() && self.exec_main_code.is_some() && self.exec_main_status.is_some()
    }
    pub(crate) async fn read(unit: &crate::manager::Unit) -> Self {
        let service = &unit.service;
        // systemd reports unavailable accounting as u64::MAX.
        let known = |value: zbus::Result<u64>| value.ok().filter(|value| *value != u64::MAX);
        // Names are `doramcap-<scope>-<id>.service`.
        let id = unit
            .name
            .strip_suffix(".service")
            .and_then(|name| name.splitn(3, '-').nth(2))
            .unwrap_or_default()
            .to_owned();
        Self {
            unit: unit.name.clone(),
            id,
            result: service.result().await.ok(),
            exec_main_code: service.exec_main_code().await.ok(),
            exec_main_status: service.exec_main_status().await.ok(),
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
            && self.units.iter().all(UnitExit::complete)
    }
}

/// Runs one experiment per repository at a time on the system service manager.
/// Prior owned resources block admission until explicit cleanup succeeds.
#[derive(Default)]
pub struct SystemdRunner {
    blocked: bool,
}
impl SystemdRunner {
    pub fn new() -> Self {
        Self::default()
    }
    /// Stop and remove a repository's abandoned resources under its admission lock.
    /// Leaves artifacts untouched; errors preserve resources whose shutdown is unconfirmed.
    pub async fn cleanup(&mut self, repository: &Path) -> Result<()> {
        let scope = Scope::of(repository)?;
        let _lock = lock_scope(&scope)?;
        self.blocked = true;
        let remaining = Manager::connect().await?.reset(&scope).await?;
        if !remaining.is_empty() {
            return Err(format!("cleanup left resources: {remaining:?}"));
        }
        self.blocked = false;
        Ok(())
    }

    /// Whether an unfinished run or unverified removal prevents admission.
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// Refuse prior owned resources, then run `plan` with the shared `run` inputs until
    /// `stop`, then stop and remove.
    ///
    /// # Results and errors
    /// Invalid plans, a busy repository and prior owned resources return `Err` before
    /// launch. Start, stop and removal failures enter `Ok(SystemdRunReport)`, which is
    /// also written to `report.json` in the new artifact directory.
    ///
    /// # Ownership and cancellation
    /// Owns `observer`. Signal `stop` and await completion; dropping the future leaves
    /// units and artifacts for explicit cleanup and keeps this runner blocked.
    pub async fn run(
        &mut self,
        run: workload_lifecycle::launch::RunPlan,
        plan: SystemdPlan,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<SystemdRunReport> {
        self.run_with_preflight(run, plan, stop, observer, ready, |_| Ok(()))
            .await
    }

    /// Capture and freeze the same inputs as `inspect`, then call `preflight` after admission.
    /// A rejected callback removes its new private artifacts before any units start.
    /// Native directories, executable files and references inside external units remain live.
    pub async fn run_with_preflight(
        &mut self,
        run: workload_lifecycle::launch::RunPlan,
        plan: SystemdPlan,
        stop: impl Future<Output = ()>,
        observer: impl Observer + 'static,
        ready: Option<tokio::sync::oneshot::Sender<()>>,
        preflight: impl FnOnce(&crate::ConfigurationSnapshot) -> Result<()>,
    ) -> Result<SystemdRunReport> {
        crate::validate(&run, &plan)?;
        let scope = Scope::of(Path::new(&run.repository_root))?;
        let _lock = lock_scope(&scope)?;
        let manager = Rc::new(Manager::connect().await?);
        let remaining = manager.remaining(&scope).await?;
        if !remaining.is_empty() {
            self.blocked = true;
            return Err(format!(
                "prior resources remain: {remaining:?}; run systemd_run --cleanup {}",
                run.repository_root
            ));
        }
        let mut executor = LocalExecutor::new(observer)
            .with_label(&run.id)
            .capture_payloads(true);
        executor.register::<crate::PrepareRun>(|| crate::PrepareRun);
        executor.register::<crate::DiscardRun>(|| crate::DiscardRun);
        executor.register::<WaitForRecordings>(|| WaitForRecordings);
        executor.register::<FinalizeRecordings>(|| FinalizeRecordings);
        let executor = Rc::new(executor);
        let prepared = executor
            .run::<crate::PrepareRun>(crate::PreparationInput {
                run: Some(run),
                plan: Some(plan),
            })
            .await
            .map_err(error)?;
        let mut run = prepared.value().run.clone().expect("prepared run");
        let mut plan = prepared.value().plan.clone().expect("prepared plan");
        let preparation = async {
            if !run.dora_apps.is_empty() {
                std::fs::create_dir_all(Path::new(&run.artifacts).join("dora/runtime"))
                    .map_err(error)?;
            }
            crate::dora::prepare(&mut run, &mut plan).await?;
            preflight(
                prepared
                    .value()
                    .snapshot
                    .as_ref()
                    .expect("prepared snapshot"),
            )
        }
        .await;
        if let Err(e) = preparation {
            return Err(match executor.run::<crate::DiscardRun>(prepared).await {
                Ok(_) => e,
                Err(cleanup) => format!("{e}; {cleanup}"),
            });
        }
        std::fs::write(scope.directory.join(crate::ACTIVE_RUN), &run.id).map_err(error)?;
        self.blocked = true;

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
            dora: None,
            phases: Vec::new(),
            diagnostics: Vec::new(),
        };
        let (outcome, removed) =
            workload_lifecycle::run::run(running, &executor, recordings, stop).await;
        let mut diagnostics = removed.diagnostics;
        diagnostics.extend(outcome.diagnostics);
        for unit in &removed.units {
            if !unit.complete() {
                diagnostics.push(format!("{}: required exit evidence unavailable", unit.unit));
            }
        }
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

/// The same admission lock serializes runs and operator cleanup.
fn lock_scope(scope: &Scope) -> Result<std::fs::File> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&scope.directory)
        .map_err(error)?;
    let lock = std::fs::File::create(scope.directory.join("lock")).map_err(error)?;
    lock.try_lock()
        .map_err(|e| format!("another run owns this repository: {e}"))?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_exit_evidence_prevents_success_but_accounting_is_optional() {
        // Arrange: wire-format report with complete exit evidence and no accounting.
        let json = serde_json::json!({
            "id": "run", "execution_error": null, "writers_stopped": true,
            "recording_finalized": true, "cleanup_verified": true, "diagnostics": [],
            "units": [{"unit": "example.service", "id": "example", "result": "success",
                "exec_main_code": 1, "exec_main_status": 0}]
        });
        let report: SystemdRunReport = serde_json::from_value(json.clone()).unwrap();
        // Act / Assert
        assert!(report.successful());
        for field in ["result", "exec_main_code", "exec_main_status"] {
            // Arrange
            let mut incomplete = json.clone();
            incomplete["units"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            // Act
            let report: SystemdRunReport = serde_json::from_value(incomplete).unwrap();
            // Assert
            assert!(!report.successful(), "missing {field}");
        }
    }
}
