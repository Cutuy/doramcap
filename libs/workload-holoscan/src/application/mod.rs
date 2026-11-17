//! Native application and supporting system-app Run ownership through shared teardown.
mod arguments;
mod container;
mod launch;
mod observation;
mod report;
mod reset;
mod running;
mod views;

use crate::system::runtime::{PrepareSystems, Systems};
use crate::{
    BuildKey, BuildStoreRequest, ConfigurationSnapshot, HoloscanRunReport, PrepareLaunch,
    PrepareRequest, builds, cli_options,
};
use docker_engine::Engine;
use orchestrator::{Input, LocalExecutor, Observer};
use std::{future::Future, path::Path, rc::Rc};
use tokio::sync::{Mutex, oneshot, watch};
use workload_lifecycle::{admission::Admission, launch::RunPlan};

/// Backend-local acknowledgements; delivery never controls the run or confirms teardown.
#[derive(Default)]
pub struct RunSignals {
    /// Repository reset and authoritative Review comparison passed before new run resources.
    pub admitted: Option<oneshot::Sender<()>>,
    /// The actual native container passed image, ownership, placement and mount checks.
    pub ready: Option<oneshot::Sender<()>>,
    /// Teardown began; await the run result to confirm stopped writers and cleanup.
    pub quiescing: Option<oneshot::Sender<()>>,
}

/// Native application/system workflow on the caller's long-lived local runtime.
/// Signal Stop and await completion; dropping preserves recovery ownership and cannot await cleanup.
pub struct ApplicationRunner {
    engine: Engine,
}
impl ApplicationRunner {
    /// Share the selected local Engine; construction starts no work.
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    /// Recheck authoritative Build/Review under ordinary admission, then run through shared teardown.
    /// Input/preflight errors precede new run resources; the returned report separates runtime/evidence/cleanup.
    /// Call on a long-lived `tokio::task::LocalSet`; own `stop` and await completion before closing it.
    pub async fn run(
        &self,
        mut run: RunPlan,
        mut request: PrepareRequest,
        expected: ConfigurationSnapshot,
        stop: impl Future<Output = ()> + 'static,
        observer: impl Observer + 'static,
        mut signals: RunSignals,
    ) -> Result<HoloscanRunReport, String> {
        let plan = request.plan.clone().ok_or("Holoscan plan is required")?;
        let selection = plan
            .selection
            .clone()
            .ok_or("Holoscan selection is required")?;
        crate::preparation::validate(&run, &plan.cli, &selection, &plan.system_projects)?;
        compose_apps::preparation::validate_budgets(run.readiness_seconds, run.stop_seconds)
            .map_err(error)?;
        host_filesystem::changes::validate_exclusions(report::excluded(&run)).map_err(error)?;
        if !Path::new(&request.build_store_root).is_absolute() {
            return Err("Holoscan Build store root must be absolute".into());
        }
        validate_artifacts(Path::new(&run.artifacts)).await?;
        let cidfile = Path::new(&run.artifacts).join(launch::CIDFILE);
        cli_options::validate(&run, &cidfile)?;
        let stopper = StopWatch::new(stop);
        let admission = Admission::acquire(Path::new(&run.repository_root)).map_err(error)?;
        run.repository_root = admission
            .repository()
            .to_str()
            .ok_or("canonical repository must be UTF-8")?
            .into();
        cli_options::validate(&run, &cidfile)?;
        check_stop(&stopper.changed)?;
        crate::routing::verify_native_daemon(admission.repository(), &self.engine)
            .await
            .map_err(error)?;
        let live = Rc::new(Mutex::new(launch::Live::new(self.engine.clone())));
        let adapter = Rc::new(compose_apps::compose::DockerAdapter::from_engine(
            self.engine.clone(),
        ));
        let mut executor = LocalExecutor::new(observer).with_label(&run.id);
        let reset_engine = self.engine.clone();
        executor.register::<reset::ResetProject>(move || reset::ResetProject {
            engine: reset_engine.clone(),
        });
        let prepare_engine = self.engine.clone();
        let prepare_stop = stopper.changed.clone();
        executor.register::<PrepareLaunch>(move || {
            PrepareLaunch::with_shutdown(prepare_engine.clone(), prepare_stop.clone())
        });
        executor.register::<PrepareSystems>(|| PrepareSystems);
        let launch_live = live.clone();
        executor.register::<launch::LaunchApplication>(move || launch::LaunchApplication {
            live: launch_live.clone(),
        });
        let filesystem_engine = self.engine.clone();
        executor.register::<running::ObserveFilesystem>(move || running::ObserveFilesystem {
            engine: filesystem_engine.clone(),
        });
        executor.register::<workload_lifecycle::capture::CollectFilesystemChanges>(|| {
            workload_lifecycle::capture::CollectFilesystemChanges
        });
        executor.register::<workload_lifecycle::recording::FinalizeRecordings>(|| {
            workload_lifecycle::recording::FinalizeRecordings
        });
        executor.validate_steps().map_err(error)?;
        // Reset is intentionally settled before observing source/Build facts.
        let reset = executor
            .run::<reset::ResetProject>(run.clone())
            .await
            .map_err(error)?;
        check_stop(&stopper.changed)?;
        let lookup = builds::lookup_admitted(
            BuildStoreRequest {
                root: request.build_store_root.clone(),
                key: Some(BuildKey {
                    repository_root: run.repository_root.clone(),
                    selection: Some(selection),
                }),
            },
            &admission,
        )
        .await
        .map_err(error)?;
        let lookup_diagnostics = lookup.diagnostics;
        request.build_record = lookup.record;
        let prepared = executor
            .run::<PrepareLaunch>(Input::new((run.clone(), request)).after(&reset))
            .await
            .map_err(error)?;
        check_stop(&stopper.changed)?;
        if !prepared.value().0.rejections.is_empty() {
            return Err(lookup_diagnostics
                .into_iter()
                .chain(prepared.value().0.rejections.iter().cloned())
                .collect::<Vec<_>>()
                .join("; "));
        }
        let snapshot = prepared
            .value()
            .0
            .snapshot
            .as_ref()
            .ok_or("Holoscan Review snapshot is unavailable")?;
        if expected.sha256.is_empty() || *snapshot != expected {
            return Err("ConfigurationChanged".into());
        }
        if let Some(admitted) = signals.admitted.take() {
            let _ = admitted.send(());
        }
        let systems_prepared = if let Some(resolved) = prepared.value().1.clone() {
            let systems = executor
                .run::<PrepareSystems>(Input::new(resolved).after(&prepared))
                .await
                .map_err(error)?;
            run = systems.value().run.clone();
            Some(systems)
        } else {
            host_filesystem::files::create_private_dir(Path::new(&run.artifacts))
                .await
                .map_err(error)?;
            None
        };
        check_stop(&stopper.changed)?;
        // Own helper recovery before the first writable view or mutating Compose command.
        let helper = match crate::process::prepare(
            &self.engine,
            &run,
            compose_apps::cgroup::Role::User,
            launch::COMPONENT,
        )
        .await
        {
            Ok(helper) => helper,
            Err(error) => {
                crate::reset::reset_project(&self.engine, &run)
                    .await
                    .map_err(|cleanup| format!("{error}; helper preparation cleanup: {cleanup}"))?;
                return Err(error);
            }
        };
        let scope: host_filesystem::files::SharedCommandScope = std::sync::Arc::new(helper.clone());
        let adapter = Rc::new(adapter.with_command_scope(scope.clone()));
        live.lock().await.helper = Some(helper);
        let systems = systems_prepared.map(|systems| {
            Systems::bind(
                &mut executor,
                systems,
                adapter.clone(),
                Rc::new(std::cell::RefCell::new(Vec::new())),
            )
        });
        let view_stop = stopper.changed.clone();
        let view_adapter = adapter.clone();
        executor.register::<views::PrepareViews>(move || views::PrepareViews {
            shutdown: view_stop.clone(),
            adapter: view_adapter.clone(),
        });
        executor.register::<workload_lifecycle::capture::FinalizeOverlays>(move || {
            workload_lifecycle::capture::FinalizeOverlays {
                scope: scope.clone(),
            }
        });
        let setup = async {
            let parent = Path::new(&run.artifacts)
                .parent()
                .expect("validated artifact parent");
            tokio::fs::File::open(parent)
                .await
                .map_err(error)?
                .sync_all()
                .await
                .map_err(error)?;
            live.lock().await.begin_observation(&run)?;
            Ok::<_, String>(())
        }
        .await;
        if let Err(error) = setup {
            return Err(
                match crate::reset::reset_project(&self.engine, &run).await {
                    Ok(()) => error,
                    Err(cleanup) => format!("{error}; setup cleanup: {cleanup}"),
                },
            );
        }
        let executor = Rc::new(executor);
        let selection = plan.selection.clone();
        let running = running::ApplicationRunning {
            executor: executor.clone(),
            live,
            run: run.clone(),
            plan,
            prepared,
            signals,
            report: HoloscanRunReport {
                id: run.id.clone(),
                selection,
                stdout: observation::STDOUT.into(),
                stderr: observation::STDERR.into(),
                ..Default::default()
            },
            views: None,
            preparing: None,
            launched: None,
            stopped: None,
            finalized: None,
            removal_owner: Err("owned resource removal order is not established".into()),
            systems,
        };
        let (outcome, mut result) = workload_lifecycle::run::run(
            running,
            &executor,
            run.recordings
                .iter()
                .map(|recording| workload_lifecycle::recording::Recording {
                    stream: recording.stream.clone().into(),
                    recording: recording.recording.clone().into(),
                })
                .collect(),
            wait_stop(stopper.changed.clone()),
        )
        .await;
        result.execution_error = outcome.execution_error.unwrap_or_default();
        result.stopped = Some(outcome.stopped);
        result.permit = Some(outcome.permit);
        result.cleanup = Some(outcome.cleanup);
        result.diagnostics.extend(outcome.diagnostics);
        if let Err(error) = report::save(Path::new(&run.artifacts), &result).await {
            result.report_error = error;
        }
        drop(admission);
        Ok(result)
    }
}

async fn validate_artifacts(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("run artifacts require an existing parent")?;
    if !path.is_absolute() || tokio::fs::canonicalize(parent).await.map_err(error)? != parent {
        return Err("run artifacts require a canonical absolute parent".into());
    }
    match tokio::fs::symlink_metadata(path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(error(source)),
        Ok(_) => Err("run artifacts must be fresh".into()),
    }
}
fn check_stop(changed: &watch::Receiver<bool>) -> Result<(), String> {
    if *changed.borrow() || changed.has_changed().is_err() {
        Err("Stopped".into())
    } else {
        Ok(())
    }
}
async fn wait_stop(mut changed: watch::Receiver<bool>) {
    loop {
        if *changed.borrow_and_update() {
            return;
        }
        if changed.changed().await.is_err() {
            return;
        }
    }
}
struct StopWatch {
    changed: watch::Receiver<bool>,
    task: tokio::task::JoinHandle<()>,
}
impl StopWatch {
    fn new(stop: impl Future<Output = ()> + 'static) -> Self {
        let (send, changed) = watch::channel(false);
        let task = tokio::task::spawn_local(async move {
            stop.await;
            send.send_replace(true);
        });
        Self { changed, task }
    }
}
impl Drop for StopWatch {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
