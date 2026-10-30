//! Owner-local session state and command transitions.
use crate::{
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

pub(crate) struct Session {
    pub phase: Phase,
    pub prepared: Option<PreparedReview>,
    pub marker: Option<Marker>,
    pub closing: bool,
    pub shutdown_waiters: Vec<oneshot::Sender<()>>,
    pub start_key: Option<String>,
    pub start_result: Option<Result<String, String>>,
    pub start_waiters: Vec<oneshot::Sender<Result<String, String>>>,
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
    pub snapshot: Arc<Mutex<crate::protocol::Status>>,
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
            shutdown_waiters: Vec::new(),
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
            snapshot: Arc::new(Mutex::new(crate::protocol::Status::default())),
            loss: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn publish(&self) {
        let marker = self.marker.as_ref();
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
                .map(|p| p.project.directory.to_string_lossy().into_owned())
                .or_else(|| marker.map(|m| m.project_directory.clone()))
                .unwrap_or_default(),
            profiles: self
                .prepared
                .as_ref()
                .map(|p| p.project.profiles.clone())
                .or_else(|| marker.map(|m| m.profiles.clone()))
                .unwrap_or_default(),
            system_apps: self
                .prepared
                .as_ref()
                .map(|p| crate::system_apps::selection(&p.system_apps))
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
            activity_available: !self.progress.rows().is_empty(),
            monitor_ready: self.monitor_ready,
            cpu_available: false,
            report_available: false,
        };
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = status;
        }
    }
    pub fn settle_start(&mut self, result: Result<String, String>) {
        self.start_result = Some(result.clone());
        for waiter in self.start_waiters.drain(..) {
            let _ = waiter.send(result.clone());
        }
    }
    pub fn stop_run(&mut self) {
        self.stop_latched.set(true);
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if matches!(self.phase, Phase::Starting | Phase::Running) {
            self.phase = Phase::Stopping;
        }
        self.publish();
    }
    pub fn finish_shutdown(&mut self) {
        for waiter in self.shutdown_waiters.drain(..) {
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
) -> oneshot::Receiver<Result<String, String>> {
    use crate::{
        project,
        protocol::{Marker, Outcome},
    };
    use orchestrator::{Observer, telemetry::TelemetryObserver};
    use std::{io, os::unix::fs::DirBuilderExt, path::Path};
    use workload_docker::{compose::Error, workflows::recorded_compose::ComposeRun};

    let (sender, receiver) = oneshot::channel();
    let mut session = state.session.borrow_mut();
    if request.request_key.is_empty() || request.request_key.len() > 128 {
        let _ = sender.send(Err("InvalidInput".into()));
        return receiver;
    }
    if session.start_key.as_deref() == Some(&request.request_key) {
        if session
            .marker
            .as_ref()
            .is_none_or(|marker| marker.run_id != request.prepared_id)
        {
            let _ = sender.send(Err("Conflict".into()));
            return receiver;
        }
        if let Some(result) = &session.start_result {
            let _ = sender.send(result.clone());
        } else {
            session.start_waiters.push(sender);
        }
        return receiver;
    }
    if session.phase != Phase::Reviewed || session.closing {
        let _ = sender.send(Err("Conflict".into()));
        return receiver;
    }
    let Some(prepared) = session.prepared.take() else {
        let _ = sender.send(Err("Conflict".into()));
        return receiver;
    };
    if prepared.review.prepared_id != request.prepared_id {
        session.prepared = Some(prepared);
        let _ = sender.send(Err("Conflict".into()));
        return receiver;
    }
    let rediscovered = project::discover(
        Path::new(&state.config.project_root),
        &prepared.project.directory,
        prepared.project.profiles.clone(),
    );
    if !rediscovered.as_ref().is_ok_and(|fresh| {
        fresh.directory == prepared.project.directory
            && fresh.files == prepared.project.files
            && fresh.env_files == prepared.project.env_files
    }) {
        let _ = std::fs::remove_dir_all(&prepared.directory);
        session.phase = Phase::Idle;
        session.publish();
        let _ = sender.send(Err("StaleReview".into()));
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
        let _ = sender.send(Err("Internal".into()));
        return receiver;
    }
    let marker = Marker {
        version: 1,
        run_id: run_id.clone(),
        request_key: request.request_key.clone(),
        phase: Phase::Starting as i32,
        artifact_reference: run_id.clone(),
        outcome: None,
        start_accepted: false,
        project_directory: prepared.project.directory.to_string_lossy().into_owned(),
        profiles: prepared.project.profiles.clone(),
        system_apps: Some(crate::system_apps::selection(&prepared.system_apps)),
    };
    if state.store.write(&marker).is_err() {
        session.phase = Phase::Blocked;
        session.publish();
        let _ = sender.send(Err("Internal".into()));
        return receiver;
    }
    let (stop_tx, stop_rx) = oneshot::channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    session.phase = Phase::Starting;
    session.marker = Some(marker);
    session.start_key = Some(request.request_key);
    session.start_result = None;
    session.start_waiters.push(sender);
    session.stop = Some(stop_tx);
    session.stop_latched = Rc::new(Cell::new(false));
    session.progress = Progress::default();
    session.metrics.clear();
    session.cpu_snapshots.clear();
    session.cpu_group_names.clear();
    session.metrics_incomplete = false;
    session.lost_events = 0;
    session.loss.store(0, Ordering::Relaxed);
    session.monitor_ready = false;
    let latch = session.stop_latched.clone();
    let (events_tx, mut events_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(512);
    let observer = TelemetryObserver::new(16 * 1024, move |bytes| {
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
            session.publish();
        }
    });
    let cpu_enabled = prepared.system_apps.cpu_monitor.is_some();
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
    let spec = ComposeRun {
        filesystem: Some(Default::default()),
        id: run_id.clone(),
        artifacts: runs.join(&run_id),
        recovery_anchor_dir: None,
        projects: vec![prepared.project.clone()],
        first_party: vec![],
        system_apps: prepared.system_apps.clone(),
        readiness_seconds: 60,
        stop_seconds: 15,
    };
    let (metrics_stop, metrics_done) = crate::metrics::watch(
        state.session.clone(),
        spec.artifacts.clone(),
        spec.system_apps.cpu_monitor.is_some(),
    );
    let session_ref = state.session.clone();
    let check_session = session_ref.clone();
    let runner = state.runner.clone();
    let store = state.store.clone();
    let check_store = store.clone();
    let check_run = run_id.clone();
    let expected_digest = prepared.workload_digest.clone();
    let expected_monitor = prepared.monitor_image.clone();
    tokio::task::spawn_local(async move {
        let observer_for_runner = observer.clone();
        let (result, blocked) = {
            let mut runner = runner.lock().await;
            let result = runner
                .run_with_preflight(
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
                    Some(ready_tx),
                    move |prepared_run| {
                        let plans = prepared_run.projects();
                        let workload = plans.iter().find(|p| p.key == "workload");
                        let monitor = plans
                            .iter()
                            .find(|p| p.key == "system")
                            .and_then(|p| p.services.get("cpu-monitor"));
                        let rejection = if latch.get() {
                            Some("Stopped")
                        } else if workload.is_none_or(|p| p.sha256 != expected_digest)
                            || monitor.map(|m| &m.image_id) != expected_monitor.as_ref()
                        {
                            Some("StaleReview")
                        } else {
                            None
                        };
                        if let Some(reason) = rejection {
                            check_session.borrow_mut().settle_start(Err(reason.into()));
                            return Err(Error::Invalid(reason.into()));
                        }
                        let mut session = check_session.borrow_mut();
                        let mut marker = session.marker.clone().expect("starting marker");
                        marker.start_accepted = true;
                        if check_store.write(&marker).is_err() {
                            session.phase = Phase::Blocked;
                            session.publish();
                            session.settle_start(Err("Internal".into()));
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
                session.settle_start(Err("Internal".into()));
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
                    Phase::Idle,
                    Some(Outcome {
                        execution: "Cancelled".into(),
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
                let mut status = status.clone();
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
        let _ = std::fs::remove_dir_all(prepared.directory);
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
                let config = ConsoleConfig {
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
                .unwrap();
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
                shared.borrow_mut().shutdown_waiters.push(done);
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
            let config = ConsoleConfig { listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap();
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
            assert_eq!(conflict_result, Err("Conflict".into()));
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
            let config = ConsoleConfig { listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap();
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
            assert_eq!(result, Err("Stopped".into()));
            assert_eq!(shared.borrow().phase, Phase::Idle);
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
            let config = ConsoleConfig { listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let shared = Rc::new(RefCell::new(Session::new(Phase::Idle, None)));
            let mut state = NativeState { runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))), store: Rc::new(store), adapter: adapter.clone(), config, session: shared.clone() };
            for case in ["environment", "new-env", "override", "command", "include"] {
                std::fs::write(project.join(".env"), "VALUE=before\n").unwrap();
                if case == "new-env" { std::fs::remove_file(project.join(".env")).unwrap(); }
                std::fs::write(project.join("compose.yaml"), base).unwrap();
                std::fs::write(project.join("included.yaml"), included).unwrap();
                let review = prepare::prepare(adapter.clone(), &state.config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap();
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
                assert_eq!(result, Err("StaleReview".into()));
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
                assert_eq!(result, Err("StaleReview".into()));
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
            let config = ConsoleConfig { listen_address: "127.0.0.1:8080".into(), public_origin: "http://127.0.0.1:8080".into(), project_root: root.to_string_lossy().into_owned(), data_root: root.join("data").to_string_lossy().into_owned(), docker_socket: "/var/run/docker.sock".into(), access_secret_file: "/tmp/secret".into(), system_apps: Some(SystemAppsConfig { cpu_monitor: Some(CpuMonitor { image: "doramcap/system-monitor:native-cleanup-20261029".into() }), ..Default::default() }) };
            let (store, _) = StateStore::open(std::path::Path::new(&config.data_root)).unwrap();
            let adapter = Rc::new(DockerAdapter::connect(&config.docker_socket).await.unwrap());
            let review = prepare::prepare(adapter.clone(), &config, PrepareRequest { system_apps: None, directory: project.to_string_lossy().into_owned(), profiles: vec![] }).await.unwrap();
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
            assert_eq!(admission, Err("Conflict".into()));
            assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
            assert!(!std::fs::read_to_string("/proc/self/mountinfo").unwrap().contains(root.to_str().unwrap()));
        }).await;
    }
}
