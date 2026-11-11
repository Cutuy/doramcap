//! Admission, explicit cleanup and the report for one systemd run.
use crate::{Result, Scope, SystemdPlan, error, manager::Manager, running::SystemdRunning};
use orchestrator::{LocalExecutor, Observer};
use serde::{Deserialize, Serialize};
use std::{future::Future, os::unix::fs::DirBuilderExt, path::Path, rc::Rc};
use workload_lifecycle::{
    admission::Admission,
    recording::{FinalizeRecordings, Recording, WaitForRecordings},
};

/// How one unit ended, read while this run still referenced it.
/// Accounting fields are `None` when systemd did not report them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitExit {
    pub unit: String,
    /// systemd's `Result`, such as `success`, `exit-code`, `signal` or `timeout`.
    pub result: Option<String>,
    /// `CLD_*` code of the main process; zero when it never ran.
    pub exec_main_code: Option<i32>,
    pub exec_main_status: Option<i32>,
    pub cpu_usage_nsec: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
}
impl UnitExit {
    /// Program ID or adopted unit stem, derived from the owned unit name.
    pub fn id(&self) -> &str {
        self.unit
            .strip_suffix(".service")
            .and_then(|name| name.strip_prefix("doramcap-"))
            .and_then(|name| name.split_once('-'))
            .map_or(self.unit.as_str(), |(_, id)| id)
    }

    fn complete(&self) -> bool {
        self.result.is_some() && self.exec_main_code.is_some() && self.exec_main_status.is_some()
    }
    pub(crate) async fn read(unit: &crate::manager::Unit) -> Self {
        let service = &unit.service;
        // systemd reports unavailable accounting as u64::MAX.
        let known = |value: zbus::Result<u64>| value.ok().filter(|value| *value != u64::MAX);
        Self {
            unit: unit.name.clone(),
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
        let admission = Admission::acquire(repository).map_err(error)?;
        let scope = Scope::of(admission.repository())?;
        remove_obsolete_lock(&scope)?;
        self.blocked = true;
        let manager = Rc::new(Manager::connect().await?);
        let system_apps = Rc::new(workload_lifecycle::cgroup::Group::new(Rc::new(
            crate::cgroup::SystemAppSlice::native(manager.clone(), &scope),
        )));
        let executor = Rc::new_cyclic(|weak| {
            let mut executor = LocalExecutor::default();
            workload_lifecycle::cgroup::register(&mut executor, system_apps);
            crate::runtime::register(&mut executor, manager, scope.clone(), weak.clone());
            executor
        });
        executor
            .run::<crate::runtime::RemoveRuntime>(scope)
            .await
            .map_err(error)?;
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
        let admission = Admission::acquire(Path::new(&run.repository_root)).map_err(error)?;
        let scope = Scope::of(admission.repository())?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&scope.directory)
            .map_err(error)?;
        remove_obsolete_lock(&scope)?;
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
        let system_apps = Rc::new(workload_lifecycle::cgroup::Group::new(Rc::new(
            crate::cgroup::SystemAppSlice::native(manager.clone(), &scope),
        )));
        let executor = Rc::new_cyclic(|weak| {
            workload_lifecycle::cgroup::register(&mut executor, system_apps.clone());
            crate::runtime::register(&mut executor, manager.clone(), scope.clone(), weak.clone());
            executor
        });
        let prepared = executor
            .run::<crate::PrepareRun>(crate::PreparationInput {
                run: Some(run),
                plan: Some(plan),
            })
            .await
            .map_err(error)?;
        let mut run = prepared.value().run.clone().expect("prepared run");
        let mut plan = prepared.value().plan.clone().expect("prepared plan");
        let preparation = || {
            if !run.dora_apps.is_empty() {
                std::fs::create_dir_all(Path::new(&run.artifacts).join("dora/runtime"))
                    .map_err(error)?;
            }
            crate::dora::prepare(&mut run, &mut plan)?;
            preflight(
                prepared
                    .value()
                    .snapshot
                    .as_ref()
                    .expect("prepared snapshot"),
            )
        };
        if let Err(e) = preparation() {
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
            system_apps,
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
            prepared,
            runtime: None,
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

/// Lock file written before shared admission; `/run` keeps it until reboot.
const OBSOLETE_LOCK: &str = "lock";

/// Remove the obsolete lock file so it cannot read as a leftover run; absence succeeds.
/// Call while holding admission for the scope's repository.
fn remove_obsolete_lock(scope: &Scope) -> Result<()> {
    match std::fs::remove_file(scope.directory.join(OBSOLETE_LOCK)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsolete_lock_is_removed_and_absence_is_accepted() {
        // Arrange
        let directory =
            std::env::temp_dir().join(format!("doramcap-obsolete-lock-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join(OBSOLETE_LOCK), b"").unwrap();
        let scope = Scope {
            prefix: String::new(),
            slice: String::new(),
            directory: directory.clone(),
        };

        // Act / Assert
        remove_obsolete_lock(&scope).unwrap();
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        remove_obsolete_lock(&scope).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }

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
