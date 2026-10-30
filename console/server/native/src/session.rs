//! Owner-local session state and command transitions.
use crate::{
    policy,
    prepare::PreparedReview,
    progress::Progress,
    protocol::{ConsoleConfig, Marker, Phase},
    state::StateStore,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::oneshot;
use workload_docker::{compose::DockerAdapter, workflows::recorded_compose::ComposeRunner};

/// Stable public error categories; private diagnostics are stored separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandError {
    InvalidInput,
    Conflict,
    StaleReview,
    Stopped,
    Busy,
    Internal,
}
impl CommandError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "InvalidInput",
            Self::Conflict => "Conflict",
            Self::StaleReview => "StaleReview",
            Self::Stopped => "Stopped",
            Self::Busy => "Busy",
            Self::Internal => "Internal",
        }
    }
}

pub(crate) struct Session {
    pub phase: Phase,
    pub prepared: Option<PreparedReview>,
    pub marker: Option<Marker>,
    pub closing: bool,
    pub shutdown_waiter: Option<oneshot::Sender<()>>,
    pub start_key: Option<String>,
    pub start_result: Option<Result<String, CommandError>>,
    pub start_waiters: Vec<oneshot::Sender<Result<String, CommandError>>>,
    pub stop: Option<oneshot::Sender<()>>,
    pub stop_latched: Rc<Cell<bool>>,
    pub progress: Progress,
    pub metrics: Vec<crate::protocol::RunMetric>,
    pub metrics_incomplete: bool,
    pub cpu_snapshots: Vec<telemetry_protocol::pb::Snapshot>,
    pub cpu_group_names: Vec<crate::protocol::CpuGroupName>,
    pub observer: Option<orchestrator::telemetry::TelemetryObserver>,
    pub lost_events: u64,
    pub monitor_ready: bool,
    pub snapshot: Arc<Mutex<Arc<crate::protocol::Status>>>,
    pub loss: Arc<AtomicU64>,
}

impl Session {
    pub fn new(phase: Phase, marker: Option<Marker>) -> Self {
        let accepted = marker.as_ref().filter(|marker| marker.start_accepted);
        let start_key = accepted.map(|marker| marker.request_key.clone());
        let start_result = accepted.map(|marker| Ok(marker.run_id.clone()));
        Self {
            phase,
            prepared: None,
            marker,
            closing: false,
            shutdown_waiter: None,
            start_key,
            start_result,
            start_waiters: Vec::new(),
            stop: None,
            stop_latched: Rc::new(Cell::new(false)),
            progress: Progress::default(),
            metrics: Vec::new(),
            metrics_incomplete: false,
            cpu_snapshots: Vec::new(),
            cpu_group_names: Vec::new(),
            observer: None,
            lost_events: 0,
            monitor_ready: false,
            snapshot: Arc::new(Mutex::new(Arc::new(crate::protocol::Status::default()))),
            loss: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn reset_activity(&mut self) {
        self.progress = Progress::default();
        self.metrics.clear();
        self.cpu_snapshots.clear();
        self.cpu_group_names.clear();
        self.metrics_incomplete = false;
        self.lost_events = 0;
        self.loss.store(0, Ordering::Relaxed);
        self.monitor_ready = false;
    }
    pub fn publish(&self) {
        let marker = self.marker.as_ref().filter(|_| {
            matches!(
                self.phase,
                Phase::Starting
                    | Phase::Running
                    | Phase::Stopping
                    | Phase::Finished
                    | Phase::Blocked
            )
        });
        let status = crate::protocol::Status {
            phase: self.phase as i32,
            run_id: self
                .prepared
                .as_ref()
                .map(|p| p.review.run_id.clone())
                .or_else(|| marker.map(|m| m.run_id.clone()))
                .unwrap_or_default(),
            project_directory: self
                .prepared
                .as_ref()
                .map(|p| p.project().directory.to_string_lossy().into_owned())
                .or_else(|| marker.map(|m| m.project_directory.clone()))
                .unwrap_or_default(),
            profiles: self
                .prepared
                .as_ref()
                .map(|p| p.project().profiles.clone())
                .or_else(|| marker.map(|m| m.profiles.clone()))
                .unwrap_or_default(),
            system_apps: self
                .prepared
                .as_ref()
                .map(|p| crate::system_apps::selection(&p.run.system_apps))
                .or_else(|| marker.and_then(|m| m.system_apps.clone())),
            outcome: marker.and_then(|m| m.outcome.clone()),
            steps: self.progress.rows(),
            sequence: self.progress.sequence,
            lost_events: self.loss.load(Ordering::Relaxed),
            truncated_steps: self.progress.truncated,
            overflow_steps: self.progress.overflow,
            progress_incomplete: self.progress.incomplete()
                || self.loss.load(Ordering::Relaxed) != 0,
            metrics: self.metrics.clone(),
            cpu_snapshots: self.cpu_snapshots.clone(),
            cpu_group_names: self.cpu_group_names.clone(),
            metrics_incomplete: self.metrics_incomplete,
            activity_available: !self.progress.is_empty(),
            monitor_ready: self.monitor_ready,
            cpu_available: false,
            report_available: false,
        };
        let publication = Arc::new(status);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            let previous = std::mem::replace(&mut *snapshot, publication);
            drop(snapshot);
            drop(previous);
        }
    }
    fn add_start_waiter(&mut self, sender: oneshot::Sender<Result<String, CommandError>>) {
        self.start_waiters.retain(|waiter| !waiter.is_closed());
        if self.start_waiters.len() >= policy::MAX_START_WAITERS {
            let _ = sender.send(Err(CommandError::Busy));
        } else {
            self.start_waiters.push(sender);
        }
    }
    pub fn settle_start(&mut self, result: Result<String, CommandError>) {
        self.start_result = Some(result.clone());
        for waiter in self.start_waiters.drain(..) {
            let _ = waiter.send(result.clone());
        }
    }
    pub fn begin_stopping(&mut self) {
        if matches!(self.phase, Phase::Starting | Phase::Running) {
            self.phase = Phase::Stopping;
            self.publish();
        }
    }
    pub fn stop_run(&mut self) {
        self.stop_latched.set(true);
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.begin_stopping();
    }
    pub fn finish_shutdown(&mut self) {
        if let Some(waiter) = self.shutdown_waiter.take() {
            let _ = waiter.send(());
        }
    }
}

pub(crate) struct NativeState {
    pub runner: Rc<tokio::sync::Mutex<ComposeRunner>>,
    pub store: Rc<StateStore>,
    pub adapter: Rc<DockerAdapter>,
    pub config: ConsoleConfig,
    pub session: Rc<RefCell<Session>>,
}

/// Admit or join a Start request and poll execution on the local owner runtime.
pub(crate) fn start(
    state: &mut NativeState,
    request: crate::protocol::StartRequest,
) -> oneshot::Receiver<Result<String, CommandError>> {
    use crate::{
        project,
        protocol::{Marker, Outcome},
    };
    use orchestrator::{Observer, telemetry::TelemetryObserver};
    use std::{io, os::unix::fs::DirBuilderExt, path::Path};
    use workload_docker::compose::Error;

    let (sender, receiver) = oneshot::channel();
    let mut session = state.session.borrow_mut();
    if request.request_key.is_empty() || request.request_key.len() > policy::MAX_REQUEST_KEY_BYTES {
        let _ = sender.send(Err(CommandError::InvalidInput));
        return receiver;
    }
    if session.start_key.as_deref() == Some(&request.request_key) {
        if session
            .marker
            .as_ref()
            .is_none_or(|marker| marker.run_id != request.prepared_id)
        {
            let _ = sender.send(Err(CommandError::Conflict));
            return receiver;
        }
        if let Some(result) = &session.start_result {
            let _ = sender.send(result.clone());
        } else {
            session.add_start_waiter(sender);
        }
        return receiver;
    }
    if session.phase != Phase::Reviewed || session.closing {
        let _ = sender.send(Err(CommandError::Conflict));
        return receiver;
    }
    let Some(prepared) = session.prepared.take() else {
        let _ = sender.send(Err(CommandError::Conflict));
        return receiver;
    };
    if prepared.review.prepared_id != request.prepared_id {
        session.prepared = Some(prepared);
        let _ = sender.send(Err(CommandError::Conflict));
        return receiver;
    }
    let rediscovered = project::discover(
        Path::new(&state.config.project_root),
        &prepared.project().directory,
        prepared.project().profiles.clone(),
    );
    if !rediscovered.as_ref().is_ok_and(|fresh| {
        fresh.directory == prepared.project().directory
            && fresh.files == prepared.project().files
            && fresh.env_files == prepared.project().env_files
    }) {
        let _ = std::fs::remove_dir_all(&prepared.run.artifacts);
        session.phase = Phase::Idle;
        session.publish();
        let _ = sender.send(Err(CommandError::StaleReview));
        return receiver;
    }
    let run_id = prepared.review.run_id.clone();
    let runs = Path::new(&state.config.data_root).join("runs");
    if std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&runs)
        .is_err()
    {
        session.prepared = Some(prepared);
        let _ = sender.send(Err(CommandError::Internal));
        return receiver;
    }
    let marker = Marker {
        version: policy::MARKER_VERSION,
        run_id: run_id.clone(),
        request_key: request.request_key.clone(),
        phase: Phase::Starting as i32,
        artifact_reference: run_id.clone(),
        outcome: None,
        start_accepted: false,
        project_directory: prepared.project().directory.to_string_lossy().into_owned(),
        profiles: prepared.project().profiles.clone(),
        system_apps: Some(crate::system_apps::selection(&prepared.run.system_apps)),
    };
    if state.store.write(&marker).is_err() {
        session.phase = Phase::Blocked;
        session.publish();
        let _ = sender.send(Err(CommandError::Internal));
        return receiver;
    }
    let (stop_tx, stop_rx) = oneshot::channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let (quiescing_tx, quiescing_rx) = oneshot::channel();
    session.phase = Phase::Starting;
    session.marker = Some(marker);
    session.start_key = Some(request.request_key);
    session.start_result = None;
    session.add_start_waiter(sender);
    session.stop = Some(stop_tx);
    session.stop_latched = Rc::new(Cell::new(false));
    session.reset_activity();
    let latch = session.stop_latched.clone();
    let (events_tx, mut events_rx) =
        tokio::sync::mpsc::channel::<Vec<u8>>(policy::EVENT_QUEUE_CAPACITY);
    let observer = TelemetryObserver::new(policy::MAX_EVENT_BYTES, move |bytes| {
        events_tx
            .try_send(bytes.to_vec())
            .map_err(|e| io::Error::other(e.to_string()))
    });
    session.observer = Some(observer.clone());
    let loss = session.loss.clone();
    session.publish();
    drop(session);
    let progress_session = state.session.clone();
    let progress_done = tokio::task::spawn_local(async move {
        while let Some(bytes) = events_rx.recv().await {
            let mut session = progress_session.borrow_mut();
            match workflow_protocol::decode(&bytes) {
                Ok(event) => session.progress.apply(event),
                Err(_) => session.progress.unknown_version = true,
            }
        }
    });
    let cpu_enabled = prepared.run.system_apps.cpu_monitor.is_some();
    let ready_session = state.session.clone();
    tokio::task::spawn_local(async move {
        if ready_rx.await.is_ok() {
            let mut session = ready_session.borrow_mut();
            session.monitor_ready = cpu_enabled;
            if session.phase == Phase::Starting {
                session.phase = Phase::Running;
            }
            session.publish();
        }
    });
    let runtime = crate::config::runtime(&state.config);
    let mut spec = prepared.run.clone();
    spec.artifacts = runs.join(&run_id);
    let (metrics_stop, metrics_done) = crate::metrics::watch(
        state.session.clone(),
        spec.artifacts.clone(),
        spec.system_apps.cpu_monitor.is_some(),
        std::time::Duration::from_millis(u64::from(runtime.sample_interval_ms.unwrap())),
    );
    let session_ref = state.session.clone();
    let check_session = session_ref.clone();
    let shutdown_session = session_ref.clone();
    tokio::task::spawn_local(async move {
        if quiescing_rx.await.is_ok() {
            shutdown_session.borrow_mut().begin_stopping();
        }
    });
    let runner = state.runner.clone();
    let store = state.store.clone();
    let check_store = store.clone();
    let check_run = run_id.clone();
    let expected_receipt = prepared.receipt.clone();
    tokio::task::spawn_local(async move {
        let observer_for_runner = observer.clone();
        let (result, blocked) = {
            let mut runner = runner.lock().await;
            let result = runner
                .run_with_signals(
                    spec.clone(),
                    async {
                        let _ = stop_rx.await;
                    },
                    move |event: &orchestrator::Event| {
                        let mut safe = event.clone();
                        safe.payload = None;
                        safe.error = safe.error.map(|_| "step failed".into());
                        observer_for_runner.observe(&safe);
                        loss.store(observer_for_runner.dropped_events(), Ordering::Relaxed);
                    },
                    workload_docker::workflows::recorded_compose::RunSignals {
                        ready: Some(ready_tx),
                        quiescing: Some(quiescing_tx),
                    },
                    move |prepared_run| {
                        let rejection = if latch.get() {
                            Some(CommandError::Stopped)
                        } else if prepared_run.receipt != expected_receipt {
                            Some(CommandError::StaleReview)
                        } else {
                            None
                        };
                        if let Some(reason) = rejection {
                            check_session.borrow_mut().settle_start(Err(reason));
                            return Err(Error::Invalid(reason.code().into()));
                        }
                        let mut session = check_session.borrow_mut();
                        let mut marker = session.marker.clone().expect("starting marker");
                        marker.start_accepted = true;
                        if check_store.write(&marker).is_err() {
                            session.phase = Phase::Blocked;
                            session.publish();
                            session.settle_start(Err(CommandError::Internal));
                            return Err(Error::Invalid("marker persistence failed".into()));
                        }
                        session.marker = Some(marker);
                        session.settle_start(Ok(check_run));
                        Ok(())
                    },
                )
                .await;
            let blocked = runner.blocked();
            (result, blocked)
        };
        let lost = observer.dropped_events();
        session_ref.borrow_mut().observer = None;
        drop(observer);
        let _ = progress_done.await;
        let _ = metrics_stop.send(());
        let _ = metrics_done.await;
        let (final_phase, snapshot) = {
            let mut session = session_ref.borrow_mut();
            session.lost_events = lost;
            if session.start_result.is_none() {
                session.settle_start(Err(CommandError::Internal));
            }
            let (phase, outcome) = match result {
                Ok(report) => {
                    let outcome = Outcome {
                        execution: if report.execution_error.is_some() {
                            "Failed"
                        } else {
                            "Completed"
                        }
                        .into(),
                        writers_stopped: Some(report.writers_stopped),
                        recording_finalized: spec
                            .system_apps
                            .cpu_monitor
                            .as_ref()
                            .map(|_| report.recording_finalized),
                        boundary_clean: Some(report.boundary_clean),
                        event_stream_healthy: Some(report.event_stream_healthy),
                    };
                    let public = serde_json::json!({"execution":outcome.execution,"writers_stopped":outcome.writers_stopped,"recording_finalized":outcome.recording_finalized,"boundary_clean":outcome.boundary_clean,"event_stream_healthy":outcome.event_stream_healthy,"system_apps":crate::system_apps::selection(&spec.system_apps).ids});
                    let path = spec.artifacts.join("console-report.json");
                    let _ = workload_docker::evidence::write_private_once(&path, &public);
                    (
                        if report.boundary_clean && !blocked {
                            Phase::Finished
                        } else {
                            Phase::Blocked
                        },
                        Some(outcome),
                    )
                }
                Err(_) if !blocked && session.phase != Phase::Blocked => (
                    if session.stop_latched.get() {
                        Phase::Finished
                    } else {
                        Phase::Idle
                    },
                    Some(Outcome {
                        execution: if session.stop_latched.get() {
                            "Cancelled"
                        } else {
                            "Failed"
                        }
                        .into(),
                        boundary_clean: Some(true),
                        ..Default::default()
                    }),
                ),
                Err(_) => (Phase::Blocked, None),
            };
            let mut marker = session.marker.clone().expect("starting marker");
            marker.phase = phase as i32;
            marker.outcome = outcome;
            let final_phase = if store.write(&marker).is_ok() {
                phase
            } else {
                Phase::Blocked
            };
            session.marker = Some(marker);
            session.stop = None;
            session.publish();
            let snapshot = session.snapshot.lock().ok().map(|status| {
                let mut status = status.as_ref().clone();
                status.phase = final_phase as i32;
                status
            });
            (final_phase, snapshot)
        };
        if let Some(snapshot) = snapshot {
            let _ = crate::metrics::checkpoint(spec.artifacts.clone(), snapshot).await;
        }
        {
            let mut session = session_ref.borrow_mut();
            session.phase = final_phase;
            session.publish();
            session.finish_shutdown();
        }
        let _ = std::fs::remove_dir_all(prepared.run.artifacts);
    });
    receiver
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        prepare,
        protocol::{ConsoleConfig, PrepareRequest, StartRequest},
        state::StateStore,
    };
    use std::time::Duration;
    use workload_docker::workflows::system_apps::{CpuMonitor, SystemAppsConfig};

    #[test]
    fn pending_start_subscriptions_are_bounded_and_closed_receivers_are_reclaimed() {
        // Arrange
        let mut session = Session::new(Phase::Starting, None);
        let mut receivers = Vec::new();
        for _ in 0..policy::MAX_START_WAITERS {
            let (sender, receiver) = oneshot::channel();
            session.add_start_waiter(sender);
            receivers.push(receiver);
        }
        let (extra, mut rejected) = oneshot::channel();
        // Act
        session.add_start_waiter(extra);
        // Assert
        assert_eq!(rejected.try_recv().unwrap(), Err(CommandError::Busy));
        assert_eq!(session.start_waiters.len(), policy::MAX_START_WAITERS);
        // Act: a disconnected subscriber releases admission for the next retry.
        drop(receivers.pop());
        let (next, mut admitted) = oneshot::channel();
        session.add_start_waiter(next);
        session.settle_start(Ok("run".into()));
        // Assert
        assert_eq!(admitted.try_recv().unwrap(), Ok("run".into()));
        for mut receiver in receivers {
            assert_eq!(receiver.try_recv().unwrap(), Ok("run".into()));
        }
        assert!(session.start_waiters.is_empty());
    }

    #[test]
    fn published_snapshots_are_shared_and_remain_immutable_across_transitions() {
        // Arrange
        let mut session = Session::new(Phase::Starting, None);
        session.publish();
        let before = session.snapshot.lock().unwrap().clone();
        let another_reader = session.snapshot.lock().unwrap().clone();
        assert!(Arc::ptr_eq(&before, &another_reader));
        // Act
        session.begin_stopping();
        // Assert
        let after = session.snapshot.lock().unwrap().clone();
        assert_eq!(before.phase, Phase::Starting as i32);
        assert_eq!(after.phase, Phase::Stopping as i32);
        assert!(!Arc::ptr_eq(&before, &after));
    }

    #[test]
    fn shutdown_is_one_way_and_terminal_sessions_cannot_be_stopped_again() {
        // Arrange
        let mut session = Session::new(Phase::Running, None);
        // Act / Assert: automatic quiescence publishes Stopping immediately.
        session.begin_stopping();
        assert_eq!(session.phase, Phase::Stopping);
        assert_eq!(
            session.snapshot.lock().unwrap().phase,
            Phase::Stopping as i32
        );
        // Act / Assert: a repeated stop cannot reopen a completed session.
        session.phase = Phase::Finished;
        session.begin_stopping();
        assert_eq!(session.phase, Phase::Finished);
    }

    #[test]
    fn preparing_a_new_review_clears_completed_run_telemetry_and_outcome() {
        // Arrange
        let mut session = Session::new(
            Phase::Finished,
            Some(Marker {
                run_id: "old-run".into(),
                outcome: Some(crate::protocol::Outcome {
                    execution: "Completed".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
        session.progress.apply(workflow_protocol::Event {
            version: workflow_protocol::VERSION,
            sequence: 1,
            elapsed_ns: 90_000_000_000,
            state: workflow_protocol::event::State::Succeeded as i32,
            ..Default::default()
        });
        session.metrics.push(crate::protocol::RunMetric::default());
        session
            .cpu_snapshots
            .push(telemetry_protocol::pb::Snapshot::default());
        session.cpu_group_names.push(crate::protocol::CpuGroupName {
            id: 1,
            name: "old".into(),
        });
        session.monitor_ready = true;
        // Act
        session.reset_activity();
        session.phase = Phase::Preparing;
        session.publish();
        // Assert
        let snapshot = session.snapshot.lock().unwrap();
        assert!(snapshot.run_id.is_empty());
        assert_eq!(snapshot.sequence, 0);
        assert!(snapshot.steps.is_empty());
        assert!(snapshot.metrics.is_empty());
        assert!(snapshot.cpu_snapshots.is_empty());
        assert!(snapshot.cpu_group_names.is_empty());
        assert!(snapshot.outcome.is_none());
        assert!(!snapshot.monitor_ready);
        assert_eq!(session.marker.as_ref().unwrap().run_id, "old-run");
    }

    #[test]
    fn restored_accepted_request_keeps_its_retry_identity() {
        // Arrange
        let marker = Marker {
            version: 1,
            run_id: "run".into(),
            request_key: "key".into(),
            phase: Phase::Starting as i32,
            start_accepted: true,
            ..Default::default()
        };
        // Act
        let restored = Session::new(Phase::Blocked, Some(marker));
        // Assert
        assert_eq!(restored.start_key.as_deref(), Some("key"));
        assert_eq!(restored.start_result, Some(Ok("run".into())));
        assert_eq!(restored.phase, Phase::Blocked);
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker, a prebuilt workload image and Linux capabilities"]
    async fn run_without_system_apps_stops_cleanly_and_persists_selection() {
        use prost::Message;
        tokio::task::LocalSet::new()
            .run_until(async {
                // Arrange
                let root = tempfile::tempdir().unwrap().keep();
                println!("private test evidence: {}", root.display());
                let project = root.join("project");
                std::fs::create_dir(&project).unwrap();
                std::fs::write(
                    project.join("compose.yaml"),
                    r#"services:
  worker:
    image: node:22-bookworm-slim
    command: ["node", "-e", "process.on('SIGTERM',()=>process.exit(0)); require('fs').writeFileSync('/tmp/ready','ready'); setInterval(()=>{},1000)"]
    healthcheck:
      test: ["CMD", "test", "-f", "/tmp/ready"]
      interval: 1s
      timeout: 1s
      retries: 20
"#,
                )
                .unwrap();
                let config = ConsoleConfig { runtime: None,
                    listen_address: "127.0.0.1:8080".into(),
                    public_origin: "http://127.0.0.1:8080".into(),
                    project_root: root.to_string_lossy().into_owned(),
                    data_root: root.join("data").to_string_lossy().into_owned(),
                    docker_socket: "/var/run/docker.sock".into(),
                    access_secret_file: "/tmp/secret".into(),
                    system_apps: Some(SystemAppsConfig {
                        cpu_monitor: Some(CpuMonitor {
                            image: "console-missing-monitor:selection-test".into(),
                        }),
                        ..Default::default()
                    }),
                };
                let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
                let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
                let prepared = prepare::prepare(
                    adapter.clone(),
                    &config,
                    PrepareRequest {
                        directory: project.to_string_lossy().into_owned(),
                        profiles: vec![],
                        system_apps: Some(crate::protocol::SystemAppSelection::default()),
                    },
                )
                .await
                .unwrap().into_launchable().unwrap();
                let id = prepared.review.run_id.clone();
                let mut session = Session::new(Phase::Reviewed, None);
                session.prepared = Some(prepared);
                let shared = Rc::new(RefCell::new(session));
                let mut state = NativeState {
                    runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))),
                    store: Rc::new(store),
                    adapter,
                    config,
                    session: shared.clone(),
                };
                let request = StartRequest {
                    prepared_id: id.clone(),
                    request_key: "no-apps".into(),
                };
                // Act: retries join the reviewed run with its explicit empty selection.
                let first = start(&mut state, request.clone());
                let retry = start(&mut state, request);
                assert_eq!(first.await.unwrap().unwrap(), id);
                assert_eq!(retry.await.unwrap().unwrap(), id);
                tokio::time::timeout(std::time::Duration::from_secs(90), async {
                    while shared.borrow().phase == Phase::Starting {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                })
                .await
                .unwrap();
                // Assert
                assert_eq!(shared.borrow().phase, Phase::Running);
                assert!(!shared.borrow().monitor_ready);
                assert!(shared.borrow().metrics.is_empty());
                // Act: Stop awaits finalization and cleanup with no recorder.
                let (done, finished) = oneshot::channel();
                shared.borrow_mut().shutdown_waiter = Some(done);
                shared.borrow_mut().stop_run();
                tokio::time::timeout(std::time::Duration::from_secs(60), finished)
                    .await
                    .unwrap()
                    .unwrap();
                // Assert
                let session = shared.borrow();
                assert_eq!(session.phase, Phase::Finished);
                let marker = session.marker.as_ref().unwrap();
                assert_eq!(marker.outcome.as_ref().unwrap().execution, "Completed");
                assert_eq!(marker.outcome.as_ref().unwrap().boundary_clean, Some(true));
                assert_eq!(marker.outcome.as_ref().unwrap().recording_finalized, None);
                let restored = Marker::decode(
                    std::fs::read(root.join("data/current.pb"))
                        .unwrap()
                        .as_slice(),
                )
                .unwrap();
                assert!(restored.system_apps.unwrap().ids.is_empty());
                assert!(!root.join("data/runs").join(&id).join("cpu.mcap").exists());
                assert!(session.cpu_snapshots.is_empty());
                assert!(
                    session
                        .snapshot
                        .lock()
                        .unwrap()
                        .system_apps
                        .as_ref()
                        .unwrap()
                        .ids
                        .is_empty()
                );
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker, prebuilt worker/monitor images and Linux capabilities"]
    async fn real_run_retries_once_stops_and_finalizes() {
        tokio::task::LocalSet::new().run_until(async {
            // Arrange
            let root = tempfile::tempdir().unwrap().keep();
            println!("private test evidence: {}", root.display());
            let project = root.join("project");
            std::fs::create_dir(&project).unwrap();
            std::fs::write(project.join("compose.yaml"), "services:\n  worker:\n    image: node:22-bookworm-slim\n    entrypoint: [\"/bin/sh\", \"-c\"]\n    command: [\"trap 'exit 0' TERM INT; while :; do sleep 1; done\"]\n").unwrap();
            let config = ConsoleConfig { runtime: None, listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap().into_launchable().unwrap();
            let id = review.review.run_id.clone();
            println!("test run id: {id}");
            let mut session = Session::new(Phase::Reviewed, None);
            session.prepared = Some(review);
            let shared = Rc::new(RefCell::new(session));
            let mut state = NativeState { runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))), store: Rc::new(store), adapter: adapter.clone(), config, session: shared.clone() };
            let request = StartRequest { prepared_id: id.clone(), request_key: "same-key".into() };
            // Act: duplicate admission joins the same start; another key conflicts.
            let first = start(&mut state, request.clone());
            let retry = start(&mut state, request.clone());
            let conflict = start(&mut state, StartRequest { request_key: "other-key".into(), ..request });
            let first_result = tokio::time::timeout(Duration::from_secs(90), first).await.unwrap().unwrap();
            let retry_result = tokio::time::timeout(Duration::from_secs(2), retry).await.unwrap().unwrap();
            let conflict_result = conflict.await.unwrap();
            let ready = tokio::time::timeout(Duration::from_secs(90), async {
                loop {
                    let phase = shared.borrow().phase;
                    if phase == Phase::Running { break true; }
                    if matches!(phase, Phase::Finished | Phase::Blocked | Phase::Idle) { break false; }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await.unwrap_or(false);
            if ready { tokio::time::sleep(Duration::from_secs(3)).await; }
            shared.borrow_mut().stop_run();
            let terminal = tokio::time::timeout(Duration::from_secs(120), async {
                loop {
                    if matches!(shared.borrow().phase, Phase::Finished | Phase::Blocked | Phase::Idle) { break; }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await;
            // Assert after giving cleanup an opportunity to finish.
            assert!(terminal.is_ok(), "run failed to finish cleanup");
            assert_eq!(first_result, Ok(id.clone()));
            assert_eq!(retry_result, Ok(id.clone()));
            assert_eq!(conflict_result, Err(CommandError::Conflict));
            let session = shared.borrow();
            assert!(ready, "monitor never became ready; phase {:?}", session.phase);
            assert_eq!(session.phase, Phase::Finished);
            let snapshot = session.snapshot.lock().unwrap().clone();
            assert_eq!(snapshot.phase, Phase::Finished as i32);
            assert_eq!(snapshot.run_id, id);
            assert!(snapshot.monitor_ready);
            assert!(snapshot.steps.iter().any(|event| event.step.ends_with("PrepareRun")));
            assert!(snapshot.steps.iter().any(|event| event.step.ends_with("ObserveProjects")));
            let outcome = session.marker.as_ref().unwrap().outcome.as_ref().unwrap();
            assert_eq!(outcome.recording_finalized, Some(true));
            assert_eq!(outcome.boundary_clean, Some(true));
            let bytes = std::fs::read(root.join("data/runs").join(id).join("cpu.mcap")).unwrap();
            mcap::read::footer(&bytes).unwrap();
            let mut session_id = None;
            let mut sequence = 0_u64;
            for message in mcap::MessageStream::new(&bytes).unwrap() {
                let message = message.unwrap();
                if message.channel.topic != "doramcap/snapshots" { continue; }
                let frame = telemetry_protocol::decode(&message.data).unwrap();
                assert_eq!(frame.source, "bpf");
                assert_eq!(frame.snapshot.dropped_batches, 0);
                assert_eq!(frame.snapshot.sequence, sequence);
                if let Some(previous) = &session_id { assert_eq!(previous, &frame.session_id); }
                else { session_id = Some(frame.session_id); }
                sequence += 1;
            }
            assert!(sequence >= 3);
            println!("capture session: {} snapshots: {sequence}", session_id.unwrap());
        }).await;
    }
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker and prebuilt worker/monitor images"]
    async fn stop_during_preparation_prevents_launch() {
        tokio::task::LocalSet::new().run_until(async {
            // Arrange
            let root = tempfile::tempdir().unwrap().keep();
            let project = root.join("project");
            std::fs::create_dir(&project).unwrap();
            std::fs::write(project.join("compose.yaml"), "services:\n  worker:\n    image: node:22-bookworm-slim\n    command: [\"sleep\", \"60\"]\n").unwrap();
            let config = ConsoleConfig { runtime: None, listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap().into_launchable().unwrap();
            let id = review.review.run_id.clone();
            let mut session = Session::new(Phase::Reviewed, None);
            session.prepared = Some(review);
            let shared = Rc::new(RefCell::new(session));
            let mut state = NativeState { runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))), store: Rc::new(store), adapter: adapter.clone(), config, session: shared.clone() };
            // Act: wait for PrepareRun's actual Started event, then latch Stop.
            let result = start(&mut state, StartRequest { prepared_id: id.clone(), request_key: "stop-before-launch".into() });
            let observed = tokio::time::timeout(Duration::from_secs(20), async {
                loop {
                    if shared.borrow().progress.rows().iter().any(|event| event.step.ends_with("PrepareRun")) { break true; }
                    if shared.borrow().phase != Phase::Starting { break false; }
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }).await.unwrap_or(false);
            shared.borrow_mut().stop_run();
            let result = tokio::time::timeout(Duration::from_secs(60), result).await.unwrap().unwrap();
            tokio::time::timeout(Duration::from_secs(60), async {
                while matches!(shared.borrow().phase, Phase::Starting | Phase::Stopping | Phase::Running) { tokio::time::sleep(Duration::from_millis(10)).await; }
            }).await.unwrap();
            // Assert
            assert!(observed, "preparation event was not observed before Stop");
            assert_eq!(result, Err(CommandError::Stopped));
            assert_eq!(shared.borrow().phase, Phase::Finished);
            assert_eq!(shared.borrow().marker.as_ref().unwrap().outcome.as_ref().unwrap().execution, "Cancelled");
            assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
            assert!(!std::fs::read_to_string("/proc/self/mountinfo").unwrap().contains(root.to_str().unwrap()));
        }).await;
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker and prebuilt worker/monitor images"]
    async fn changed_inputs_reject_before_launch() {
        tokio::task::LocalSet::new().run_until(async {
            // Arrange
            let root = tempfile::tempdir().unwrap().keep();
            let project = root.join("project");
            std::fs::create_dir(&project).unwrap();
            let base = "include: [included.yaml]\nservices:\n  worker:\n    image: node:22-bookworm-slim\n    command: [\"sleep\", \"10\"]\n    environment:\n      VALUE: '${VALUE:-default}'\n";
            let included = "services:\n  auxiliary:\n    image: node:22-bookworm-slim\n    labels:\n      revision: before\n";
            std::fs::write(project.join("compose.yaml"), base).unwrap();
            std::fs::write(project.join("included.yaml"), included).unwrap();
            std::fs::write(project.join(".env"), "VALUE=before\n").unwrap();
            let config = ConsoleConfig { runtime: None, listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let shared = Rc::new(RefCell::new(Session::new(Phase::Idle, None)));
            let mut state = NativeState { runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))), store: Rc::new(store), adapter: adapter.clone(), config, session: shared.clone() };
            for case in ["environment", "new-env", "override", "command", "include"] {
                std::fs::write(project.join(".env"), "VALUE=before\n").unwrap();
                if case == "new-env" { std::fs::remove_file(project.join(".env")).unwrap(); }
                std::fs::write(project.join("compose.yaml"), base).unwrap();
                std::fs::write(project.join("included.yaml"), included).unwrap();
                let review = prepare::prepare(adapter.clone(), &state.config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap().into_launchable().unwrap();
                let id = review.review.run_id.clone();
                { let mut session = shared.borrow_mut(); session.phase = Phase::Reviewed; session.prepared = Some(review); session.start_key = None; session.start_result = None; }
                match case {
                    "environment" => std::fs::write(project.join(".env"), "VALUE=after\n").unwrap(),
                    "new-env" => std::fs::write(project.join(".env"), "VALUE=after\n").unwrap(),
                    "override" => std::fs::write(project.join("compose.override.yaml"), "services: {}\n").unwrap(),
                    "command" => std::fs::write(project.join("compose.yaml"), base.replace("\"10\"", "\"11\"")).unwrap(),
                    "include" => std::fs::write(project.join("included.yaml"), included.replace("before", "after")).unwrap(),
                    _ => unreachable!(),
                }
                // Act
                let result = start(&mut state, StartRequest { prepared_id: id.clone(), request_key: case.into() }).await.unwrap();
                tokio::time::timeout(Duration::from_secs(60), async {
                    while matches!(shared.borrow().phase, Phase::Starting | Phase::Stopping | Phase::Running) { tokio::time::sleep(Duration::from_millis(10)).await; }
                }).await.unwrap();
                // Assert
                assert_eq!(result, Err(CommandError::StaleReview));
                assert_eq!(shared.borrow().phase, Phase::Idle);
                assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
                assert!(!std::fs::read_to_string("/proc/self/mountinfo").unwrap().contains(root.to_str().unwrap()));
                if case == "override" { std::fs::remove_file(project.join("compose.override.yaml")).unwrap(); }
            }
        }).await;
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker and two prebuilt local images"]
    async fn retagged_local_image_rejects_before_launch() {
        tokio::task::LocalSet::new()
            .run_until(async {
                struct RemoveTag(String);
                impl Drop for RemoveTag {
                    fn drop(&mut self) {
                        let _ = std::process::Command::new("docker")
                            .args(["image", "rm", &self.0])
                            .output();
                    }
                }
                // Arrange: the unique alias is owned and removed by this test.
                let tag = format!("console-stale-{}:test", std::process::id());
                assert!(
                    std::process::Command::new("docker")
                        .args(["tag", "node:22-bookworm-slim", &tag])
                        .status()
                        .unwrap()
                        .success()
                );
                let _cleanup = RemoveTag(tag.clone());
                let root = tempfile::tempdir().unwrap().keep();
                let project = root.join("project");
                std::fs::create_dir(&project).unwrap();
                std::fs::write(
                    project.join("compose.yaml"),
                    format!("services:\n  worker:\n    image: '{tag}'\n"),
                )
                .unwrap();
                let config = ConsoleConfig {
                    runtime: None,
                    listen_address: "127.0.0.1:8080".into(),
                    public_origin: "http://127.0.0.1:8080".into(),
                    project_root: root.to_string_lossy().into_owned(),
                    data_root: root.join("data").to_string_lossy().into_owned(),
                    docker_socket: "/var/run/docker.sock".into(),
                    access_secret_file: "/tmp/secret".into(),
                    system_apps: Some(SystemAppsConfig {
                        cpu_monitor: Some(CpuMonitor {
                            image: "doramcap/system-monitor:native-cleanup-20261029".into(),
                        }),
                        ..Default::default()
                    }),
                };
                let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
                let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
                let review = prepare::prepare(
                    adapter.clone(),
                    &config,
                    PrepareRequest {
                        system_apps: None,
                        directory: project.to_string_lossy().into_owned(),
                        profiles: vec![],
                    },
                )
                .await
                .unwrap()
                .into_launchable()
                .unwrap();
                let id = review.review.run_id.clone();
                let mut session = Session::new(Phase::Reviewed, None);
                session.prepared = Some(review);
                let shared = Rc::new(RefCell::new(session));
                let mut state = NativeState {
                    runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))),
                    store: Rc::new(store),
                    adapter: adapter.clone(),
                    config,
                    session: shared.clone(),
                };
                // Act
                assert!(
                    std::process::Command::new("docker")
                        .args(["tag", "ubuntu:24.04", &tag])
                        .status()
                        .unwrap()
                        .success()
                );
                let result = start(
                    &mut state,
                    StartRequest {
                        prepared_id: id.clone(),
                        request_key: "retag".into(),
                    },
                )
                .await
                .unwrap();
                tokio::time::timeout(Duration::from_secs(60), async {
                    while matches!(
                        shared.borrow().phase,
                        Phase::Starting | Phase::Stopping | Phase::Running
                    ) {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .unwrap();
                // Assert
                assert_eq!(result, Err(CommandError::StaleReview));
                assert_eq!(shared.borrow().phase, Phase::Idle);
                assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
                assert!(
                    !std::fs::read_to_string("/proc/self/mountinfo")
                        .unwrap()
                        .contains(root.to_str().unwrap())
                );
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires Docker, a native monitor image and Linux capabilities"]
    async fn unclean_workload_blocks_admission_with_saved_recording() {
        tokio::task::LocalSet::new().run_until(async {
            // Arrange
            let root = tempfile::tempdir().unwrap().keep();
            let project = root.join("project");
            std::fs::create_dir(&project).unwrap();
            std::fs::write(project.join("compose.yaml"), "services:\n  worker:\n    image: node:22-bookworm-slim\n    entrypoint: [\"/bin/sh\", \"-c\"]\n    command: [\"trap 'exit 0' TERM INT; while :; do sleep 1; done\"]\n").unwrap();
            let config = ConsoleConfig { runtime: None, listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap().into_launchable().unwrap();
            let id = review.review.run_id.clone();
            println!("blocked-test run id: {id}, evidence: {}", root.display());
            let mut session = Session::new(Phase::Reviewed, None); session.prepared = Some(review);
            let shared = Rc::new(RefCell::new(session));
            let mut state = NativeState { runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))), store: Rc::new(store), adapter: adapter.clone(), config, session: shared.clone() };
            let started = start(&mut state, StartRequest { prepared_id: id.clone(), request_key: "blocked-test".into() });
            let accepted = tokio::time::timeout(Duration::from_secs(90), started).await.unwrap().unwrap();
            let ready = tokio::time::timeout(Duration::from_secs(90), async {
                loop {
                    let phase = shared.borrow().phase;
                    if phase == Phase::Running { break true; }
                    if matches!(phase, Phase::Finished | Phase::Blocked | Phase::Idle) { break false; }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await.unwrap_or(false);
            // Act: kill only this test run's worker to force unclean shutdown.
            if ready {
                let output = std::process::Command::new("docker").args(["ps", "-q", "--filter", &format!("label=io.doramcap.run={id}"), "--filter", "label=com.docker.compose.service=worker"]).output().unwrap();
                let container = String::from_utf8(output.stdout).unwrap();
                assert!(!container.trim().is_empty());
                assert!(std::process::Command::new("docker").args(["kill", container.trim()]).status().unwrap().success());
            } else { shared.borrow_mut().stop_run(); }
            let terminal = tokio::time::timeout(Duration::from_secs(120), async {
                while matches!(shared.borrow().phase, Phase::Starting | Phase::Running | Phase::Stopping) { tokio::time::sleep(Duration::from_millis(100)).await; }
            }).await;
            let admission = start(&mut state, StartRequest { prepared_id: id.clone(), request_key: "different-key".into() }).await.unwrap();
            // Cleanup only this test's exact labeled resources, even on failed assertions below.
            for kind in ["container", "network", "volume"] {
                let mut list = std::process::Command::new("docker");
                if kind == "container" { list.args(["ps", "-aq"]); } else { list.args([kind, "ls", "-q"]); }
                let output = list.args(["--filter", &format!("label=io.doramcap.run={id}")]).output().unwrap();
                for resource in String::from_utf8_lossy(&output.stdout).lines() {
                    let mut remove = std::process::Command::new("docker");
                    if kind == "container" { remove.args(["rm", "-fv", resource]); } else { remove.args([kind, "rm", resource]); }
                    assert!(remove.status().unwrap().success());
                }
            }
            let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
            for mount in mounts.lines().filter_map(|line| line.split_whitespace().nth(4)).filter(|path| path.starts_with(root.to_str().unwrap()) && path.contains("/overlays/") && path.ends_with("/merged")).collect::<Vec<_>>().into_iter().rev() {
                assert!(std::process::Command::new("umount").arg(mount).status().unwrap().success());
            }
            // Assert
            assert_eq!(accepted, Ok(id.clone())); assert!(ready); assert!(terminal.is_ok());
            assert_eq!(shared.borrow().phase, Phase::Blocked);
            let outcome = shared.borrow().marker.as_ref().unwrap().outcome.clone().unwrap();
            assert_eq!(outcome.recording_finalized, Some(true));
            assert_eq!(outcome.boundary_clean, Some(false));
            assert_eq!(admission, Err(CommandError::Conflict));
            assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
            assert!(!std::fs::read_to_string("/proc/self/mountinfo").unwrap().contains(root.to_str().unwrap()));
        }).await;
    }
}
