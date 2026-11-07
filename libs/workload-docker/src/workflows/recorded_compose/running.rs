//! Compose apps of one run behind the shared run sequence's handle.
use super::{
    execution,
    finalization::{FinalizeInput, FinalizeRun, FinalizedRun},
    observation::EventCapture,
    preparation::PreparedRun,
    *,
};
use crate::compose::{ComposeProjectState, ServiceExit};
use orchestrator::{LocalExecutor, Output};
use workload_lifecycle::launch::{CleanupRecord, RemovalPermit, StopRecord};

/// Prepared Compose resources, from the first start attempt until removal.
pub(super) struct ComposeRunning {
    pub executor: Rc<LocalExecutor>,
    pub spec: ComposeRun,
    pub prepared: Output<PreparedRun>,
    pub plan: ResolvedComposeProject,
    pub overlays: Option<Output<overlay::OverlaySet>>,
    pub resources: super::super::experiment::SharedExperiment,
    pub events: EventCapture,
    pub signals: RunSignals,
    pub diagnostics: Diagnostics,
    pub last_launch: Option<Output<ComposeProjectState>>,
    pub applications_started: bool,
    pub stopped: Option<Stopped>,
    /// Why stopping could not produce shutdown or filesystem evidence.
    pub stop_error: Option<Error>,
}

/// What stopping every app established.
pub(super) struct Stopped {
    service_exits: Vec<ServiceExit>,
    /// Adopted apps stopped without killing an MCAP writer, and system apps stopped cleanly.
    apps_clean: bool,
    runtime_stopped: bool,
    runtime_exit_clean: bool,
    shutdown: Output<Shutdown>,
    finalized: Output<FinalizedRun>,
}

/// Compose evidence handed back by removal; `completion` fails when stop could not
/// produce shutdown or filesystem evidence.
pub(super) struct Removed {
    pub completion: Result<Completed>,
    pub event_stream: Result<()>,
}
pub(super) struct Completed {
    pub service_exits: Vec<ServiceExit>,
    pub applications_started: bool,
    pub apps_clean: bool,
    pub runtime_exit_clean: bool,
    pub writers_stopped: bool,
    pub finalized: FinalizedRun,
    pub cleanup: Cleanup,
    pub boundary_clean: bool,
}

impl workload_lifecycle::run::Running for ComposeRunning {
    type Removed = Removed;

    async fn start(&mut self) -> std::result::Result<(), String> {
        let failure = self.events.failure();
        let launch = execution::launch_apps(
            &self.executor,
            &self.spec,
            &self.prepared,
            &self.plan,
            self.overlays.as_ref(),
            &mut self.last_launch,
        );
        // A lost event stream ends startup as it ends observation.
        tokio::select! {
            result = launch => result.map_err(|error| error.to_string())?,
            error = failure => return Err(error),
        }
        self.applications_started = true;
        Ok(())
    }

    fn ready(&mut self) {
        self.signals.notify_ready(self.plan.clone());
    }

    async fn ended(&mut self) -> String {
        let failure = self.events.failure();
        let monitor = execution::monitor(
            &self.executor,
            &self.plan,
            &self.resources,
            self.events.changes(),
        );
        tokio::select! {
            result = monitor => match result {
                Ok(()) => "observation ended".into(),
                Err(error) => error.to_string(),
            },
            error = failure => error,
        }
    }

    async fn stop(&mut self) -> StopRecord {
        match self.stop_apps().await {
            Ok(stopped) => {
                let record = StopRecord {
                    writers_stopped: stopped.shutdown.value().stopped && stopped.runtime_stopped,
                    evidence_durable: stopped.finalized.value().can_remove,
                };
                self.stopped = Some(stopped);
                record
            }
            Err(error) => {
                execution::record_failure(&self.diagnostics, "completion", &error);
                self.stop_error = Some(error);
                StopRecord::default()
            }
        }
    }

    async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Removed) {
        let completion = match self.stopped.take() {
            Some(stopped) => Ok(self.remove_stopped(stopped, permit).await),
            None => Err(self
                .stop_error
                .take()
                .unwrap_or_else(|| invalid("shutdown or filesystem evidence incomplete"))),
        };
        // Finish event capture even when shutdown or finalization produced no evidence.
        let event_stream = self.events.finish().await;
        let record = CleanupRecord {
            verified: completion.as_ref().is_ok_and(|c| c.boundary_clean),
        };
        (
            record,
            Removed {
                completion,
                event_stream,
            },
        )
    }
}

impl ComposeRunning {
    /// Stop adopted apps, then system apps, then the shared Dora runtime; then
    /// secure filesystem evidence once every writer stopped.
    async fn stop_apps(&mut self) -> Result<Stopped> {
        if let Some(quiescing) = self.signals.quiescing.take() {
            let _ = quiescing.send(());
        }
        let prepared = self.prepared.value();
        let adopted = execution::quiesce_services(
            &self.executor,
            &self.plan,
            &prepared.user_services,
            &self.diagnostics,
        )
        .await;
        // A killed workload loses unsent messages; a killed MCAP writer also loses its footer.
        let adopted_clean = adopted.stopped
            && !adopted
                .exits
                .iter()
                .any(|exit| exit.killed && self.spec.mcap_writers.services.contains(&exit.service));
        let systems_stopped = self
            .resources
            .lock()
            .await
            .shutdown(&prepared.system_services)
            .await;
        if let Err(error) = &systems_stopped {
            execution::record_failure(&self.diagnostics, "system shutdown", error);
        }
        let systems = execution::quiesce_services(
            &self.executor,
            &self.plan,
            &prepared.system_services,
            &self.diagnostics,
        )
        .await;
        let systems_clean = systems.stopped && !systems.exits.iter().any(|exit| exit.killed);
        let mut service_exits = adopted.exits;
        service_exits.extend(systems.exits);
        let runtime_shutdown = self
            .executor
            .run::<super::super::experiment::StopDoraRuntime>(())
            .await;
        let runtime_stopped = runtime_shutdown
            .as_ref()
            .is_ok_and(|output| output.value().stopped);
        let runtime_exit_clean = runtime_shutdown
            .as_ref()
            .is_ok_and(|output| output.value().diagnostics.is_empty());
        match runtime_shutdown {
            Ok(stopped) => {
                let stopped = stopped.into_value();
                service_exits.extend(stopped.exits);
                self.diagnostics.borrow_mut().extend(stopped.diagnostics);
            }
            Err(error) => execution::record_failure(&self.diagnostics, "Dora shutdown", error),
        }

        // Filesystem evidence requires every writer stopped.
        let mut input = Input::new(StopInput {
            allow_volume_deletion: false,
            services: vec![],
            plan: Some(self.plan.clone()),
        })
        .after(&self.prepared);
        if let Some(launched) = &self.last_launch {
            input = input.after(launched);
        }
        let shutdown = self
            .executor
            .run::<QuiesceCompose>(input)
            .await
            .map_err(step_error)?;
        self.diagnostics
            .borrow_mut()
            .extend(shutdown.value().diagnostics.clone());
        let mut writer_evidence = shutdown.value().clone();
        writer_evidence.stopped &= runtime_stopped;
        let finalized = self
            .executor
            .run::<FinalizeRun>(
                Input::new(FinalizeInput {
                    id: self.spec.id.clone(),
                    artifacts: prepared.artifacts.clone(),
                    baseline: prepared.baseline.clone(),
                    overlays: self.overlays.take().map(|output| output.into_value()),
                    shutdown: writer_evidence,
                    project: self.plan.clone(),
                })
                .after(&self.prepared)
                .after(&shutdown),
            )
            .await
            .map_err(step_error)?;
        Ok(Stopped {
            service_exits,
            apps_clean: adopted_clean && systems_stopped.is_ok() && systems_clean,
            runtime_stopped,
            runtime_exit_clean,
            shutdown,
            finalized,
        })
    }

    /// Remove project resources when permitted; otherwise report them preserved.
    async fn remove_stopped(&self, stopped: Stopped, permit: &RemovalPermit) -> Completed {
        let cleanup = if permit.remove {
            match self
                .executor
                .run::<RemoveCompose>(
                    Input::new(StopInput {
                        allow_volume_deletion: !stopped.finalized.value().preserve_volumes,
                        services: vec![],
                        plan: Some(self.plan.clone()),
                    })
                    .after(&stopped.shutdown)
                    .after(&stopped.finalized),
                )
                .await
            {
                Ok(cleanup) => Some(cleanup.into_value()),
                Err(error) => {
                    execution::record_failure(&self.diagnostics, "completion", error);
                    None
                }
            }
        } else {
            None
        }
        .unwrap_or_else(|| Cleanup {
            project: self.plan.name.clone(),
            remaining: vec!["preserved: shutdown or filesystem evidence incomplete".into()],
            ..Default::default()
        });
        let finalized = stopped.finalized.into_value();
        let boundary_clean = stopped.runtime_stopped
            && finalized.can_remove
            && cleanup.verified
            && cleanup.diagnostics.is_empty()
            && (!finalized.preserve_volumes || cleanup.retained_volumes.is_empty());
        Completed {
            service_exits: stopped.service_exits,
            applications_started: self.applications_started,
            apps_clean: stopped.apps_clean,
            runtime_exit_clean: stopped.runtime_exit_clean,
            writers_stopped: stopped.shutdown.value().stopped && stopped.runtime_stopped,
            finalized,
            cleanup,
            boundary_clean,
        }
    }
}
