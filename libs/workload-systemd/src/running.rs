//! One run's units behind the shared run sequence's handle.
use crate::{
    Scope, SystemdPlan,
    manager::{Manager, Unit},
    runner::UnitExit,
};
use orchestrator::{Input, LocalExecutor, Output};
use std::{rc::Rc, time::Duration};
use tokio::time::Instant;
use workload_lifecycle::{
    launch::{CleanupRecord, RemovalPermit, StopRecord},
    recording::{ReadinessInput, Recording, WaitForRecordings},
};

pub(crate) struct SystemdRunning {
    pub system_apps: Rc<workload_lifecycle::cgroup::Group>,
    pub manager: Rc<Manager>,
    pub executor: Rc<LocalExecutor>,
    pub scope: Scope,
    pub run: workload_lifecycle::launch::RunPlan,
    pub plan: SystemdPlan,
    pub recordings: Vec<Recording>,
    pub ready: Option<tokio::sync::oneshot::Sender<()>>,
    /// Attempted units in fixed startup phases: Dora, system apps, user apps.
    pub phases: Vec<Vec<Unit>>,
    pub dora: Option<crate::dora::Dora>,
    pub diagnostics: Vec<String>,
    pub prepared: Output<crate::PreparedRun>,
    pub runtime: Option<Output<Scope>>,
}

/// Unit results and failures handed back by removal.
pub(crate) struct Removed {
    pub units: Vec<UnitExit>,
    pub diagnostics: Vec<String>,
}

impl workload_lifecycle::run::Running for SystemdRunning {
    type Removed = Removed;

    async fn start(&mut self) -> std::result::Result<(), String> {
        let readiness = Duration::from_secs(self.run.readiness_seconds);
        let cgroups = self
            .executor
            .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(
                Input::new(workload_lifecycle::launch::SystemAppsCgroupsInput {
                    action: workload_lifecycle::cgroup::Action::Create.into(),
                    artifacts: self.run.artifacts.clone(),
                    readiness_seconds: self.run.readiness_seconds,
                    pids: Vec::new(),
                })
                .after(&self.prepared),
            )
            .await
            .map_err(crate::error)?;
        self.runtime = Some(
            self.executor
                .run::<crate::runtime::PrepareRuntime>(
                    Input::new(crate::PreparationInput {
                        run: Some(self.run.clone()),
                        plan: Some(self.plan.clone()),
                    })
                    .after(&self.prepared)
                    .after(&cgroups),
                )
                .await
                .map_err(crate::error)?,
        );
        use workload_lifecycle::plan::Phase;
        for role in [Phase::Dora, Phase::System, Phase::User] {
            let deadline = Instant::now() + readiness;
            if role == Phase::User {
                for unit in self.phases.iter().flatten() {
                    crate::cgroup::verify_unit(
                        &self.system_apps,
                        &crate::cgroup::native_name(&self.scope),
                        unit,
                    )
                    .await?;
                }
            }
            let mut placement = self.scope.clone();
            if matches!(role, Phase::Dora | Phase::System) {
                placement.slice = crate::cgroup::native_name(&self.scope);
            }
            let mut phase = crate::phase(&self.run, &self.plan, role);
            if role != Phase::Dora {
                crate::dora::attach(&self.run, &mut phase)?;
            }
            let runtime = if role == Phase::Dora {
                None
            } else {
                self.run.dora_runtime.as_ref()
            };
            self.phases.push(Vec::new());
            let start = async {
                let names = self
                    .manager
                    .adopt(
                        &placement,
                        &phase.unit_files,
                        &phase.app_units,
                        &self.plan.binaries,
                        self.run.stop_seconds,
                        runtime,
                    )
                    .await?;
                // Keep every attempted unit before submitting any cancellable start.
                let units = self.phases.last_mut().unwrap();
                for program in &phase.programs {
                    units.push(self.manager.watch(&self.scope.unit(&program.id)).await?);
                }
                for name in &names {
                    units.push(self.manager.watch(name).await?);
                }
                let mut jobs = Vec::new();
                for program in &phase.programs {
                    jobs.push(
                        self.manager
                            .start_program(&placement, &self.run, program)
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
                if matches!(role, Phase::Dora | Phase::System) {
                    for unit in self.phases.last().unwrap() {
                        crate::cgroup::verify_unit(
                            &self.system_apps,
                            &crate::cgroup::native_name(&self.scope),
                            unit,
                        )
                        .await?;
                    }
                }
                Ok::<_, String>(())
            };
            tokio::time::timeout_at(deadline, start)
                .await
                .map_err(|_| format!("apps not active within {readiness:?}"))??;
            if role == Phase::Dora && !self.run.dora_apps.is_empty() {
                self.dora = Some(
                    tokio::time::timeout_at(
                        deadline,
                        crate::dora::Dora::from_units(
                            self.phases.last().unwrap(),
                            &mut self.run,
                            &self.plan,
                            deadline,
                        ),
                    )
                    .await
                    .map_err(|_| "Dora access deadline exceeded".to_owned())??,
                );
                let dora = self.dora.as_mut().unwrap();
                tokio::time::timeout_at(deadline, async {
                    dora.client
                        .wait_for_daemon(deadline)
                        .await
                        .map_err(crate::error)?;
                    dora.register(&mut self.run, deadline).await
                })
                .await
                .map_err(|_| "Dora readiness deadline exceeded".to_string())??;
            }
            if role == Phase::System {
                self.executor
                    .run::<WaitForRecordings>(ReadinessInput {
                        recordings: self.recordings.clone(),
                        timeout_seconds: self.run.readiness_seconds,
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
        for (index, phase) in self.phases.iter_mut().enumerate().rev() {
            // Dataflows stop before the coordinator and daemon they run on.
            if index == 0
                && let Some(dora) = &mut self.dora
                && let Err(error) = dora
                    .down(Instant::now() + Duration::from_secs(self.run.stop_seconds))
                    .await
            {
                self.diagnostics.push(format!("Dora down: {error}"));
            }
            let results = futures_util::future::join_all(
                phase
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
        for unit in self.phases.iter().flatten() {
            units.push(UnitExit::read(unit).await);
            if permit.remove {
                self.manager.release(&unit.name).await;
            }
        }
        let verified = if permit.remove {
            let input = match &self.runtime {
                Some(runtime) => Input::new(runtime.value().clone()).after(runtime),
                None => Input::new(self.scope.clone()).after(&self.prepared),
            };
            match self
                .executor
                .run::<crate::runtime::RemoveRuntime>(input)
                .await
            {
                Ok(_) => true,
                Err(error) => {
                    self.diagnostics.push(crate::error(error));
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
