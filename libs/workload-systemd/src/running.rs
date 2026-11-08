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
            self.groups.push(Vec::new());
            let start = async {
                let names = self
                    .manager
                    .adopt(&self.scope, &group.unit_files, run.stop_seconds)
                    .await?;
                // Keep every attempted unit before submitting any cancellable start.
                let units = self.groups.last_mut().unwrap();
                for program in &group.programs {
                    units.push(self.manager.watch(&self.scope.unit(&program.id)).await?);
                }
                for name in &names {
                    units.push(self.manager.watch(name).await?);
                }
                let mut jobs = Vec::new();
                for program in &group.programs {
                    jobs.push(
                        self.manager
                            .start_program(&self.scope, &run.id, program, run.stop_seconds)
                            .await?,
                    );
                }
                for name in &names {
                    jobs.push(self.manager.start_adopted(name).await?);
                }
                futures_util::future::try_join_all(
                    jobs.into_iter().map(|job| self.manager.wait_started(job)),
                )
                .await?;
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
        let mut stopped = true;
        for group in self.groups.iter_mut().rev() {
            let results = futures_util::future::join_all(
                group
                    .iter_mut()
                    .map(|unit| self.manager.stop_and_wait(unit)),
            )
            .await;
            for result in results {
                if let Err(error) = result {
                    stopped = false;
                    self.diagnostics.push(error);
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
            if permit.remove {
                self.manager.release(&unit.name).await;
            }
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
        let remaining = self.manager.reset(&self.scope).await?;
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(format!("units remain after removal: {remaining:?}"))
        }
    }
}
