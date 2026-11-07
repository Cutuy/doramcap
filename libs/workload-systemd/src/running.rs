//! One run's units behind the shared run sequence's handle.
use crate::{
    Result, Scope, SystemdPlan,
    manager::{Manager, Unit},
    runner::UnitExit,
};
use orchestrator::LocalExecutor;
use std::{rc::Rc, time::Duration};
use workload_lifecycle::{
    launch::{CleanupRecord, RemovalPermit, StopRecord},
    recording::{ReadinessInput, Recording, WaitForRecordings},
};

/// Time beyond a unit's own stop timeout allowed for systemd to report it ended.
const STOP_REPORT_MARGIN: Duration = Duration::from_secs(5);

pub(crate) struct SystemdRunning {
    pub manager: Rc<Manager>,
    pub executor: Rc<LocalExecutor>,
    pub scope: Scope,
    pub run: workload_lifecycle::launch::RunPlan,
    pub plan: SystemdPlan,
    pub recordings: Vec<Recording>,
    pub ready: Option<tokio::sync::oneshot::Sender<()>>,
    /// Units this run started, by start group.
    pub groups: Vec<Vec<Unit>>,
    pub diagnostics: Vec<String>,
}

/// Unit results and failures handed back by removal.
pub(crate) struct Removed {
    pub units: Vec<UnitExit>,
    pub diagnostics: Vec<String>,
}

impl SystemdRunning {
    fn run(&self) -> &workload_lifecycle::launch::RunPlan {
        &self.run
    }
}

impl workload_lifecycle::run::Running for SystemdRunning {
    type Removed = Removed;

    async fn start(&mut self) -> std::result::Result<(), String> {
        let run = self.run().clone();
        let readiness = Duration::from_secs(run.readiness_seconds);
        for group in self.plan.groups.clone() {
            // Units are recorded before they start, so stop and removal see every attempt.
            self.groups.push(Vec::new());
            let start = async {
                for program in &group.programs {
                    let unit = self
                        .manager
                        .start_program(&self.scope, &run.id, program, run.stop_seconds)
                        .await?;
                    self.groups.last_mut().unwrap().push(unit);
                }
                for name in self.manager.adopt(&self.scope, &group.unit_files).await? {
                    let unit = self.manager.start_adopted(&name).await?;
                    self.groups.last_mut().unwrap().push(unit);
                }
                Ok::<_, String>(())
            };
            tokio::time::timeout(readiness, start)
                .await
                .map_err(|_| format!("group not active within {readiness:?}"))??;
            if group.await_recordings {
                self.executor
                    .run::<WaitForRecordings>(ReadinessInput {
                        recordings: self.recordings.clone(),
                        timeout_seconds: run.readiness_seconds,
                    })
                    .await
                    .map_err(crate::error)?;
            }
        }
        Ok(())
    }

    fn ready(&mut self) {
        if let Some(ready) = self.ready.take() {
            let _ = ready.send(());
        }
    }

    /// A failed unit is recorded at stop; only a stop request ends a systemd run.
    async fn ended(&mut self) -> String {
        std::future::pending().await
    }

    async fn stop(&mut self) -> StopRecord {
        let deadline = Duration::from_secs(self.run().stop_seconds) + STOP_REPORT_MARGIN;
        let mut stopped = true;
        for group in self.groups.iter_mut().rev() {
            for unit in group.iter() {
                if let Err(error) = self.manager.stop(&unit.name).await {
                    self.diagnostics
                        .push(format!("stop {}: {error}", unit.name));
                }
            }
            for unit in group.iter_mut() {
                match tokio::time::timeout(deadline, unit.ended()).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        stopped = false;
                        self.diagnostics.push(error);
                    }
                    Err(_) => {
                        stopped = false;
                        self.diagnostics
                            .push(format!("{} still running after {deadline:?}", unit.name));
                    }
                }
            }
        }
        StopRecord {
            writers_stopped: stopped,
            evidence_durable: true,
        }
    }

    async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Removed) {
        let mut units = Vec::new();
        for unit in self.groups.iter().flatten() {
            units.push(UnitExit::read(unit).await);
            self.manager.release(&unit.name).await;
        }
        let verified = if permit.remove {
            match self.reset().await {
                Ok(()) => true,
                Err(error) => {
                    self.diagnostics.push(error);
                    false
                }
            }
        } else {
            self.diagnostics
                .push("preserved: writers did not stop".into());
            false
        };
        (
            CleanupRecord { verified },
            Removed {
                units,
                diagnostics: self.diagnostics,
            },
        )
    }
}

impl SystemdRunning {
    async fn reset(&self) -> Result<()> {
        let deadline = Duration::from_secs(self.run().stop_seconds) + STOP_REPORT_MARGIN;
        let remaining = self.manager.reset(&self.scope, deadline).await?;
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(format!("units remain after removal: {remaining:?}"))
        }
    }
}
