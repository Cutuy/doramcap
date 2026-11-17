//! The native/system handle behind the shared run sequence.
use super::{
    RunSignals,
    launch::{self, Live},
    observation, report,
    views::PrepareViews,
};
use crate::{
    HoloscanPlan, HoloscanRunReport, NativeMount, Preparation, holoscan_run_report, native_mount,
};
use compose_apps::compose::PreparedOverlays;
use docker_engine::docker_proto::OwnedContainer;
use host_filesystem::{
    changes::{CollectInput, FilesystemSources, SourcesInput},
    filesystem_proto::{AppScope, ReportState},
    overlay::{self, OverlaySet},
};
use orchestrator::{Input, LocalExecutor, Output, Step};
use std::{
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
use tokio::{sync::Mutex, task::JoinHandle};
use workload_lifecycle::{
    capture::{self, PrepareFilesystemSources},
    launch::{CleanupRecord, RemovalPermit, RunPlan, StopRecord},
};

pub(super) struct ApplicationRunning {
    pub executor: Rc<LocalExecutor>,
    pub live: Rc<Mutex<Live>>,
    pub run: RunPlan,
    pub plan: HoloscanPlan,
    pub prepared: Output<(Preparation, Option<compose_apps::preparation::ResolvedRun>)>,
    pub signals: RunSignals,
    pub report: HoloscanRunReport,
    pub views: Option<Output<PreparedOverlays>>,
    pub preparing: Option<JoinHandle<Result<Output<PreparedOverlays>, String>>>,
    pub launched: Option<Output<String>>,
    pub stopped: Option<StopRecord>,
    pub finalized: Option<bool>,
    pub removal_owner: Result<Option<String>, String>,
    pub systems: Option<crate::system::runtime::Systems>,
}
impl ApplicationRunning {
    fn workspace(&self) -> PathBuf {
        compose_apps::compose::adapter::repository_scope(Path::new(&self.run.repository_root))
            .join("overlays")
    }
    async fn settle_views(&mut self) -> Result<(), String> {
        if let Some(task) = &mut self.preparing {
            let result = task.await;
            self.preparing.take();
            self.views = Some(result.map_err(error)??);
        }
        Ok(())
    }

    async fn capture(&mut self, live: &mut Live) -> Result<bool, String> {
        if let Some(durable) = self.finalized {
            return Ok(durable);
        }
        self.finalized = Some(false);
        let Some(overlays) = self
            .views
            .as_ref()
            .map(|output| output.value().views.clone())
        else {
            // An interrupted preparation ran no CLI; discard partial staging after its task settled.
            overlay::discard(&self.workspace()).await.map_err(error)?;
            self.finalized = Some(!live.attempted);
            return Ok(!live.attempted);
        };
        if live.container.is_some() {
            live.prepare_sources(&self.run, &self.plan, &overlays)
                .await?;
        } else if !live.attempted {
            let mappings = super::views::mappings(&overlays, &self.run, &self.plan)?;
            let mut mounts = super::views::controlled_mounts(&self.run, &self.plan)?;
            for (mount, mapping) in mounts.iter_mut().zip(mappings) {
                mount.source = mapping
                    .original
                    .to_str()
                    .ok_or("canonical source must be UTF-8")?
                    .into();
            }
            let mut step = PrepareFilesystemSources;
            live.sources = Some(
                step.run(live.merge_system_sources(SourcesInput {
                    overlays: true,
                    passthrough_folders: self.run.passthrough_folders.clone(),
                    excluded: report::excluded(&self.run),
                    bind_mounts: mounts,
                    metadata_sources: vec![],
                    artifacts: PathBuf::from(&self.run.artifacts),
                })?)
                .await
                .map_err(error)?,
            );
        }
        let mut sources = live
            .sources
            .take()
            .ok_or("native filesystem source evidence is unavailable")?;
        let mut containers: Vec<_> = live
            .container
            .iter()
            .map(|id| OwnedContainer {
                id: id.clone(),
                consumer: super::views::consumer(
                    &self
                        .plan
                        .selection
                        .as_ref()
                        .expect("validated selection")
                        .application,
                ),
                app_scope: AppScope::User as i32,
            })
            .collect();
        if let Some(systems) = &self.systems {
            let system_containers = systems.containers().await?;
            sources
                .excluded
                .extend(system_containers.iter().map(|container| {
                    compose_apps::compose::adapter::stopped_record(&systems.plan.key, &container.id)
                }));
            containers.extend(system_containers);
        }
        let mut input = Input::new((sources, Some(overlays), containers)).after(&self.prepared);
        if let Some(views) = &self.views {
            input = input.after(views);
        }
        if let Some(launched) = &self.launched {
            input = input.after(launched);
        }
        let observed = self
            .executor
            .run::<ObserveFilesystem>(input)
            .await
            .map_err(error)?;
        let finalized = capture::finalize(
            &self.executor,
            observed,
            self.views.take().map(|output| output.into_value().views),
        )
        .await;
        if let Some(changes) = &finalized.changes {
            self.report.file_changes = Some(changes.reference());
            self.report.file_changes_state = ReportState::Complete as i32;
        }
        self.report.diagnostics.extend(
            finalized
                .failures
                .iter()
                .map(|(phase, error)| format!("{phase}: {error}")),
        );
        let durable = finalized.changes.is_some()
            && finalized
                .results
                .as_ref()
                .is_some_and(|results| results.unmounted)
            && finalized.failures.is_empty();
        self.finalized = Some(durable);
        Ok(durable)
    }

    fn retain_facts(&mut self, live: &Live) {
        if let Some(systems) = &self.systems {
            let mut diagnostics = systems.diagnostics.borrow().clone();
            if let Some(stopped) = &systems.stopped {
                if !stopped.clean || !stopped.runtime_exit_clean {
                    diagnostics.push(
                        compose_apps::compose::error::invalid(
                            "system or Dora shutdown was not clean",
                        )
                        .diagnostic("system shutdown"),
                    );
                }
                self.report.system_shutdown = Some(compose_apps::compose::Shutdown {
                    project: systems.plan.name.clone(),
                    stopped: stopped.services_stopped && stopped.runtime_stopped,
                    exits: stopped.exits.clone(),
                    diagnostics,
                });
            } else {
                self.report.system_shutdown = Some(compose_apps::compose::Shutdown {
                    stopped: false,
                    diagnostics,
                    ..Default::default()
                });
            }
        }
        self.report.container_id = live.container.clone().unwrap_or_default();
        self.report.cli_error = live.cli_error.clone().unwrap_or_default();
        self.report.cli_termination = live.cli_status.and_then(|status| {
            status
                .code()
                .map(holoscan_run_report::CliTermination::CliExitCode)
                .or_else(|| {
                    status
                        .signal()
                        .map(holoscan_run_report::CliTermination::CliSignal)
                })
        });
        self.report.untracked_mounts = live.coverage.clone();
        self.report.diagnostics.extend(live.notes.clone());
        if live.attempted && live.container.is_none() {
            self.report.untracked_mounts.push(
                "Native container ownership, mounts and writable coverage remain unknown".into(),
            );
        }
        if let Some(info) = &live.inspection {
            self.report.image_id = info.image.clone().unwrap_or_default();
            self.report.observed_mounts = info
                .mounts
                .iter()
                .flatten()
                .map(|mount| NativeMount {
                    kind: mount
                        .typ
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    source: mount.source.clone().unwrap_or_default(),
                    destination: mount.destination.clone().unwrap_or_default(),
                    volume_name: mount.name.clone().unwrap_or_default(),
                    access: mount.rw.map(|rw| native_mount::Access::ReadOnly(!rw)),
                })
                .collect();
            if let Some(state) = &info.state
                && state.pid == Some(0)
                && state.running == Some(false)
                && state
                    .status
                    .as_ref()
                    .is_some_and(|status| status.to_string() == "exited")
            {
                self.report.container_termination = state
                    .exit_code
                    .map(holoscan_run_report::ContainerTermination::ContainerExitCode);
            }
        }
    }
}
impl workload_lifecycle::run::Running for ApplicationRunning {
    type Removed = HoloscanRunReport;
    async fn start(&mut self) -> Result<(), String> {
        let changes = self
            .live
            .lock()
            .await
            .events
            .as_ref()
            .ok_or("run event observation is unavailable")?
            .changes();
        let operation = async {
            let executor = self.executor.clone();
            let run = self.run.clone();
            let plan = self.plan.clone();
            let workspace = self.workspace();
            let snapshot = self
                .prepared
                .value()
                .0
                .snapshot
                .clone()
                .ok_or("prepared snapshot is unavailable")?;
            let system_plans = self
                .systems
                .as_ref()
                .map(|systems| vec![systems.prepared.value().plan.clone()])
                .unwrap_or_default();
            let mut input =
                Input::new((run, plan, workspace, snapshot, system_plans)).after(&self.prepared);
            if let Some(systems) = &self.systems {
                input = input.after(&systems.prepared);
            }
            self.preparing = Some(tokio::task::spawn_local(async move {
                executor.run::<PrepareViews>(input).await.map_err(error)
            }));
            self.settle_views().await?;
            let views = self.views.as_ref().expect("settled views");
            if let Some(systems) = &mut self.systems {
                self.live.lock().await.system_sources =
                    Some(systems.source_input(&views.value().views)?);
                systems.start(&self.executor, views).await?;
            }
            let image = self
                .prepared
                .value()
                .0
                .snapshot
                .as_ref()
                .expect("prepared snapshot")
                .image_id
                .clone();
            let launch = self.executor.run::<super::launch::LaunchApplication>(
                Input::new((
                    self.run.clone(),
                    self.plan.clone(),
                    views.value().views.clone(),
                    image,
                ))
                .after(&self.prepared)
                .after(views),
            );
            self.launched = Some(if let Some(systems) = &self.systems {
                tokio::select! {
                    result = launch => result.map_err(error)?,
                    error = systems.watch(&self.executor, changes.clone()) => return Err(error),
                }
            } else {
                launch.await.map_err(error)?
            });
            Ok::<_, String>(())
        };
        let result = tokio::select! {
            result = operation => result,
            failure = observation::event_failure(changes.clone()) => Err(failure),
        };
        if let Err(error) = &result {
            self.report.execution_error = error.clone();
        }
        result
    }
    fn ready(&mut self) -> Result<(), String> {
        if let Some(ready) = self.signals.ready.take() {
            let _ = ready.send(());
        }
        Ok(())
    }
    async fn ended(&mut self) -> Result<(), String> {
        let result = if let Some(systems) = &self.systems {
            let changes = self
                .live
                .lock()
                .await
                .events
                .as_ref()
                .ok_or("run event observation is unavailable")?
                .changes();
            let native = async { self.live.lock().await.observe_end(&self.run).await };
            tokio::select! {
                result = native => result,
                error = systems.watch(&self.executor, changes) => Err(error),
            }
        } else {
            self.live.lock().await.observe_end(&self.run).await
        };
        if let Err(error) = &result {
            self.report.execution_error = error.clone();
        }
        result
    }
    async fn stop(&mut self) -> StopRecord {
        if let Some(stopped) = self.stopped {
            return stopped;
        }
        if let Some(quiescing) = self.signals.quiescing.take() {
            let _ = quiescing.send(());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.run.stop_seconds);
        let live_handle = self.live.clone();
        let mut live = live_handle.lock().await;
        let runtime = live.stop_runtime(&self.run, deadline).await;
        if let Err(error) = &runtime {
            self.report
                .diagnostics
                .push(format!("stop native writers: {error}"));
        }
        let systems_stopped = if let Some(systems) = &mut self.systems {
            let result = systems.stop(&self.executor, deadline).await;
            if let Err(error) = &result {
                self.report
                    .diagnostics
                    .push(format!("stop system writers: {error}"));
            }
            result.is_ok()
        } else {
            true
        };
        let scope =
            compose_apps::compose::adapter::repository_scope(Path::new(&self.run.repository_root));
        let owner = compose_apps::shared_dora::owner(Path::new(&self.run.repository_root));
        let quiescent = match launch::remaining_seconds(deadline) {
            Ok(seconds) => tokio::time::timeout_at(
                deadline,
                live.engine
                    .resources(scope.to_str().expect("validated scope"), &owner)
                    .quiesce(seconds),
            )
            .await
            .map_err(|_| "repository writer verification deadline expired".to_owned())
            .and_then(|result| result.map_err(error)),
            Err(error) => Err(error),
        };
        if let Ok(containers) = &quiescent {
            self.removal_owner = compose_apps::system_apps::removal_owner(containers)
                .map(|last| last.map(str::to_owned))
                .map_err(error);
            if let Err(error) = &self.removal_owner {
                self.report.diagnostics.push(error.clone());
            }
        } else if let Err(error) = &quiescent {
            self.report
                .diagnostics
                .push(format!("verify repository writers: {error}"));
        }
        // A cancelled copy/mount and source preparation always settle before collection/removal.
        if let Err(error) = self.settle_views().await {
            self.report
                .diagnostics
                .push(format!("settle views: {error}"));
        }
        if let Err(error) = live.settle_sources().await {
            self.report
                .diagnostics
                .push(format!("settle filesystem sources: {error}"));
        }
        let writers_stopped = runtime.is_ok() && systems_stopped && quiescent.is_ok();
        if let Some(output) = live.output.take()
            && let Err(error) = output.finish(writers_stopped).await
        {
            live.cli_error = Some(format!("native CLI output: {error}"));
            self.report
                .diagnostics
                .push(format!("native CLI output: {error}"));
        }
        if writers_stopped && let Some(container) = &live.container {
            match observation::runtime(
                &live.engine,
                container,
                Path::new(&self.run.artifacts),
                deadline,
            )
            .await
            {
                Ok(()) => self.report.runtime_log = observation::RUNTIME.into(),
                Err(error) => self
                    .report
                    .diagnostics
                    .push(format!("available native runtime log: {error}")),
            }
        }
        if let Some(events) = live.events.take() {
            match events.finish().await {
                Ok(()) => self.report.event_stream_healthy = true,
                Err(error) => self
                    .report
                    .diagnostics
                    .push(format!("native event stream: {error}")),
            }
        }
        let evidence_durable = if writers_stopped {
            match self.capture(&mut live).await {
                Ok(durable) => durable,
                Err(error) => {
                    self.report.file_changes_state = ReportState::CheckpointUnknown as i32;
                    self.report
                        .diagnostics
                        .push(format!("filesystem evidence: {error}"));
                    false
                }
            }
        } else {
            false
        };
        self.retain_facts(&live);
        let mut stopped = StopRecord {
            writers_stopped,
            evidence_durable,
        };
        self.report.stopped = Some(stopped);
        if let Err(error) = report::save(Path::new(&self.run.artifacts), &self.report).await {
            stopped.evidence_durable = false;
            self.report.stopped = Some(stopped);
            self.report.report_error = error;
        }
        self.stopped = Some(stopped);
        stopped
    }
    async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Self::Removed) {
        self.report.permit = Some(*permit);
        let cleanup = if permit.remove {
            let live = self.live.lock().await;
            let directory = compose_apps::compose::adapter::repository_scope(Path::new(
                &self.run.repository_root,
            ));
            let owner = compose_apps::shared_dora::owner(Path::new(&self.run.repository_root));
            let removed = async {
                let last = self.removal_owner.as_ref().map_err(Clone::clone)?;
                live.engine
                    .resources(directory.to_str().expect("validated scope"), &owner)
                    .remove(last.as_deref())
                    .await
                    .map_err(error)?;
                overlay::discard(&directory.join("overlays"))
                    .await
                    .map_err(error)?;
                if self
                    .systems
                    .as_ref()
                    .is_some_and(|systems| systems.prepared.value().cgroup.is_some())
                {
                    self.executor
                        .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(
                            workload_lifecycle::launch::SystemAppsCgroupsInput {
                                action: workload_lifecycle::cgroup::Action::Remove.into(),
                                ..Default::default()
                            },
                        )
                        .await
                        .map_err(error)?;
                }
                if let Some(helper) = &live.helper {
                    helper.remove().await?;
                }
                Ok::<_, String>(())
            }
            .await;
            if let Err(error) = &removed {
                self.report
                    .diagnostics
                    .push(format!("remove owned resources: {error}"));
            }
            CleanupRecord {
                verified: removed.is_ok(),
            }
        } else {
            CleanupRecord { verified: false }
        };
        self.report.cleanup = Some(cleanup);
        (cleanup, self.report)
    }
}

pub(super) struct ObserveFilesystem {
    pub engine: docker_engine::Engine,
}
impl Step for ObserveFilesystem {
    const ID: Option<&'static str> = Some("holoscan.observe-filesystem");
    const DISPLAY_NAME: Option<&'static str> = Some("Observe Holoscan file changes");
    type Input = (FilesystemSources, Option<OverlaySet>, Vec<OwnedContainer>);
    type Output = CollectInput;
    async fn run(
        &mut self,
        (sources, overlays, containers): Self::Input,
    ) -> orchestrator::Result<CollectInput> {
        let layers = docker_engine::filesystem::container_changes(
            &self.engine,
            &containers,
            &sources.directory,
        )
        .await
        .map_err(|error| orchestrator::Error::message(error.to_string()))?;
        Ok(CollectInput {
            sources,
            overlays,
            container_layers: Some(layers),
        })
    }
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::ContainerSummary;
    use docker_engine::{Engine, docker_proto::ResourceLabels};
    use std::{fs, os::unix::net::UnixListener};
    use workload_lifecycle::run::Running;

    struct Prepared;
    impl Step for Prepared {
        type Input = ();
        type Output = (Preparation, Option<compose_apps::preparation::ResolvedRun>);
        async fn run(&mut self, (): ()) -> orchestrator::Result<Self::Output> {
            Ok((Preparation::default(), None))
        }
    }

    #[tokio::test]
    async fn ambiguous_removal_owners_preserve_stopped_facts_and_mutate_no_cleanup_resources() {
        // Arrange: stopped writers and durable evidence do not establish a valid removal order.
        let directory = std::env::temp_dir().join(format!(
            "holoscan-removal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let run = RunPlan {
            repository_root: directory.to_str().unwrap().into(),
            ..Default::default()
        };
        let scope = compose_apps::compose::adapter::repository_scope(&directory);
        let workspace = scope.join("overlays");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("retained"), b"evidence").unwrap();
        let socket = directory.join("engine.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let engine = Engine::new(socket.to_str().unwrap(), 1).unwrap();
        let labels = ResourceLabels {
            experiment_owner: true,
            ..Default::default()
        }
        .identity_labels();
        let containers: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|id| ContainerSummary {
                id: Some(id.into()),
                labels: Some(labels.clone()),
                ..Default::default()
            })
            .collect();
        let removal_owner = compose_apps::system_apps::removal_owner(&containers)
            .map(|last| last.map(str::to_owned))
            .map_err(error);
        let stopped = StopRecord {
            writers_stopped: true,
            evidence_durable: true,
        };
        let mut executor = LocalExecutor::new(|_: &orchestrator::Event| {});
        executor.register::<Prepared>(|| Prepared);
        let prepared = executor.run::<Prepared>(()).await.unwrap();
        let running = ApplicationRunning {
            executor: Rc::new(executor),
            live: Rc::new(Mutex::new(Live::new(engine))),
            run,
            plan: HoloscanPlan::default(),
            prepared,
            signals: RunSignals::default(),
            report: HoloscanRunReport {
                stopped: Some(stopped),
                ..Default::default()
            },
            views: None,
            preparing: None,
            launched: None,
            stopped: Some(stopped),
            finalized: Some(true),
            removal_owner,
            systems: None,
        };
        // Act
        let (cleanup, report) = running
            .remove(&RemovalPermit {
                remove: true,
                recording_finalized: true,
            })
            .await;
        // Assert: no Engine connection or filesystem deletion happens on the policy error.
        assert!(!cleanup.verified);
        assert_eq!(report.stopped, Some(stopped));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|error| error.contains("multiple experiment IPC owners"))
        );
        assert_eq!(fs::read(workspace.join("retained")).unwrap(), b"evidence");
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        fs::remove_dir_all(&scope).unwrap();
        fs::remove_dir_all(&directory).unwrap();
    }
}
