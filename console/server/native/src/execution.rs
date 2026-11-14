//! Active workflow resources and bounded event transport; Console state lives in TypeScript.
use crate::protocol::*;
use compose_apps::compose::DockerAdapter;
use host_filesystem::filesystem_proto::ReportState;
use orchestrator::{Observer, telemetry::TelemetryObserver};
use prost::Message;
use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::{OnceCell, oneshot, watch};
use workload_docker::workflows::recorded_compose::{
    ComposeRun, ComposeRunner, MachineCapture, RunSignals,
};

const EVENT_QUEUE_CAPACITY: usize = 512;
const MAX_EVENT_BYTES: usize = 16 * 1024;
const MAX_ERROR_CHARS: usize = Limit::MaxErrorChars as usize;
pub(crate) type Reply = oneshot::Receiver<Result<Vec<u8>, String>>;

/// Keep the end of an error, where its cause is, so its event fits `MAX_EVENT_BYTES`.
fn bounded_error(error: &str) -> String {
    let start = error
        .char_indices()
        .rev()
        .nth(MAX_ERROR_CHARS - 1)
        .map_or(0, |(index, _)| index);
    error[start..].to_owned()
}
struct Active {
    id: String,
    stop: Option<oneshot::Sender<()>>,
    stopped: Rc<Cell<bool>>,
    done: Option<oneshot::Receiver<()>>,
}
pub struct ComposeResources {
    pub adapter: Rc<DockerAdapter>,
    runner: tokio::sync::Mutex<ComposeRunner>,
}

/// Connect once on first Compose use; failed or cancelled attempts can retry.
pub struct Compose {
    socket: String,
    resources: OnceCell<ComposeResources>,
}
impl Compose {
    pub async fn get(&self) -> Result<&ComposeResources, String> {
        if self.socket.is_empty() {
            return Err("Docker is not configured".into());
        }
        self.resources
            .get_or_try_init(|| async {
                let adapter = Rc::new(
                    DockerAdapter::connect(self.socket.clone())
                        .await
                        .map_err(|error| error.to_string())?,
                );
                Ok(ComposeResources {
                    runner: tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone())),
                    adapter,
                })
            })
            .await
    }
}

pub struct State {
    pub compose: Rc<Compose>,
    systemd: Rc<tokio::sync::Mutex<workload_systemd::SystemdRunner>>,
    active: Option<Active>,
    updates: Arc<Mutex<ExecutionUpdate>>,
    pub closing: bool,
}
impl State {
    pub async fn open(
        socket: String,
        updates: Arc<Mutex<ExecutionUpdate>>,
    ) -> Result<Self, String> {
        Ok(Self {
            compose: Rc::new(Compose {
                socket,
                resources: OnceCell::new(),
            }),
            systemd: Rc::default(),
            active: None,
            updates,
            closing: false,
        })
    }
    pub fn stop(&mut self, id: &str) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .filter(|active| active.id == id)
            .ok_or("Conflict")?;
        active.stopped.set(true);
        if let Some(stop) = active.stop.take() {
            let _ = stop.send(());
        }
        Ok(())
    }
    pub fn shutdown(&mut self) -> Option<oneshot::Receiver<()>> {
        if let Some(active) = &mut self.active {
            active.stopped.set(true);
            if let Some(stop) = active.stop.take() {
                let _ = stop.send(());
            }
            return active.done.take();
        }
        None
    }
}
/// One run's launch inputs for the backend its request selects.
pub enum Launch {
    Compose(ComposeRun),
    Systemd(workload_systemd::RunPlan, workload_systemd::SystemdPlan),
}
/// Split a run request by backend. Errors: no run inputs, or no backend.
pub fn launch(request: RunRequest) -> Result<Launch, String> {
    let run = request.run.ok_or("run request has no run inputs")?;
    match request.backend {
        Some(run_request::Backend::Compose(compose)) => ComposeRun::from_plan(run, compose)
            .map(Launch::Compose)
            .map_err(|error| error.to_string()),
        Some(run_request::Backend::Systemd(systemd)) => {
            if !matches!(
                host_filesystem::overlay::Disposition::from_policy(run.write_policy)
                    .map_err(|error| error.to_string())?,
                host_filesystem::overlay::Disposition::WriteBack
            ) {
                return Err("systemd write_policy requires systemd overlays".into());
            }
            Ok(Launch::Systemd(run, systemd))
        }
        None => Err("run request has no backend".into()),
    }
}
pub fn start(state: &mut State, request: StartRequest) -> Reply {
    let (accepted, reply) = oneshot::channel();
    if state.closing || state.active.is_some() && state.updates.lock().unwrap().result.is_none() {
        let _ = accepted.send(Err("Conflict".into()));
        return reply;
    }
    let launch = request
        .run
        .ok_or("InvalidInput".to_string())
        .and_then(launch);
    let (spec, expected) = match (launch, request.backend) {
        (Ok(Launch::Compose(spec)), Some(start_request::Backend::ComposeExpected(expected))) => {
            (spec, expected)
        }
        (
            Ok(Launch::Systemd(run, plan)),
            Some(start_request::Backend::SystemdExpected(expected)),
        ) => {
            return start_systemd(state, run, plan, expected, accepted, reply);
        }
        (Err(error), _) => {
            let _ = accepted.send(Err(error));
            return reply;
        }
        _ => {
            let _ = accepted.send(Err("InvalidInput".into()));
            return reply;
        }
    };
    let (stop_tx, mut stop_rx) = oneshot::channel();
    let (done_tx, done_rx) = oneshot::channel();
    let stopped = Rc::new(Cell::new(false));
    state.active = Some(Active {
        id: spec.id.clone(),
        stop: Some(stop_tx),
        stopped: stopped.clone(),
        done: Some(done_rx),
    });
    *state.updates.lock().unwrap() = ExecutionUpdate {
        run_id: spec.id.clone(),
        ..Default::default()
    };
    let recording_paths = spec.recordings.clone();
    let run_id = spec.id.clone();
    let updates = state.updates.clone();
    let compose = state.compose.clone();
    let sink = updates.clone();
    let observer = TelemetryObserver::new(MAX_EVENT_BYTES, move |bytes| {
        let mut update = sink
            .lock()
            .map_err(|_| io::Error::other("event queue unavailable"))?;
        if update.events.len() == EVENT_QUEUE_CAPACITY {
            return Err(io::Error::other("event queue full"));
        }
        update
            .events
            .push(workflow_protocol::Event::decode(bytes).map_err(io::Error::other)?);
        Ok(())
    });
    let (ready_tx, ready_rx) = oneshot::channel();
    let (teardown_tx, teardown_rx) = oneshot::channel();
    let readiness = updates.clone();
    let ready_id = run_id.clone();
    tokio::task::spawn_local(async move {
        if ready_rx.await.is_ok() {
            let mut update = readiness.lock().unwrap();
            if update.run_id == ready_id {
                update.ready = true;
            }
        }
    });
    let teardown = updates.clone();
    let teardown_id = run_id.clone();
    tokio::task::spawn_local(async move {
        if teardown_rx.await.is_ok() {
            let mut update = teardown.lock().unwrap();
            if update.run_id == teardown_id {
                update.teardown_started = true;
            }
        }
    });
    let (stopping_tx, mut stopping_rx) = watch::channel(Vec::new());
    let stopping = updates.clone();
    let stopping_id = run_id.clone();
    tokio::task::spawn_local(async move {
        while stopping_rx.changed().await.is_ok() {
            let waiting = stopping_rx.borrow_and_update().clone();
            let mut update = stopping.lock().unwrap();
            if update.run_id == stopping_id {
                update.stopping_services = waiting;
            }
        }
    });
    let (machine_tx, mut machine_rx) = watch::channel(MachineCapture::default());
    let final_capture = machine_rx.clone();
    let machine = updates.clone();
    tokio::task::spawn_local(async move {
        while machine_rx.changed().await.is_ok() {
            let capture = machine_rx.borrow_and_update().clone();
            let mut update = machine.lock().unwrap();
            if update.run_id == run_id {
                (update.machine_before, update.machine_before_error) =
                    crate::capture_parts(capture.before);
                (update.machine_after, update.machine_after_error) =
                    crate::capture_parts(capture.after);
            }
        }
    });
    tokio::task::spawn_local(async move {
        let connected = if let Some(resources) = compose.resources.get() {
            Ok(resources)
        } else {
            tokio::select! {
                biased;
                _ = &mut stop_rx => Err("Stopped".to_owned()),
                resources = compose.get() => resources,
            }
        };
        let resources = match connected {
            Ok(resources) => resources,
            Err(error) => {
                publish_result(
                    &updates,
                    &final_capture,
                    ExecutionResult {
                        error: Some(error.clone()),
                        ..Default::default()
                    },
                );
                let _ = accepted.send(Err(error));
                let _ = done_tx.send(());
                return;
            }
        };
        let admission = Rc::new(RefCell::new(Some(accepted)));
        let check = admission.clone();
        let mut runner = resources.runner.lock().await;
        let observe = observer.clone();
        let loss = updates.clone();
        let result = runner
            .run_with_signals(
                spec,
                async {
                    let _ = stop_rx.await;
                },
                move |event: &orchestrator::Event| {
                    // Omit bulky input/output payloads from the bounded transport.
                    observe.observe(&orchestrator::Event {
                        invocation: event.invocation.clone(),
                        state: event.state,
                        elapsed_ns: event.elapsed_ns,
                        duration_ns: event.duration_ns,
                        payload: None,
                        error: event.error.as_deref().map(bounded_error),
                    });
                    loss.lock().unwrap().dropped_events = observe.dropped_events();
                },
                RunSignals {
                    ready: Some(ready_tx),
                    quiescing: Some(teardown_tx),
                    stopping: stopping_tx,
                    machine: machine_tx,
                },
                move |prepared| {
                    let rejection = if stopped.get() {
                        Some("Stopped")
                    } else if prepared.snapshot != expected {
                        Some("ConfigurationChanged")
                    } else {
                        None
                    };
                    if let Some(error) = rejection {
                        if let Some(reply) = check.borrow_mut().take() {
                            let _ = reply.send(Err(error.into()));
                        }
                        return Err(compose_apps::compose::error::invalid(error));
                    }
                    // Prepare declared recording directories before releasing execution.
                    use std::os::unix::fs::DirBuilderExt;
                    for output in &recording_paths {
                        for path in [&output.recording, &output.stream] {
                            if !path.as_os_str().is_empty()
                                && let Some(parent) = path.parent()
                            {
                                std::fs::DirBuilder::new()
                                    .recursive(true)
                                    .mode(0o700)
                                    .create(parent)?;
                            }
                        }
                    }
                    if let Some(reply) = check.borrow_mut().take() {
                        let _ = reply.send(Ok(Vec::new()));
                    }
                    Ok(())
                },
            )
            .await;
        let blocked = runner.blocked();
        let result = match result {
            Ok(report) => ExecutionResult {
                blocked,
                file_changes: report
                    .filesystem_changes
                    .as_ref()
                    .map(|changes| changes.reference()),
                file_changes_state: if report.filesystem_changes.is_some() {
                    ReportState::Complete
                } else {
                    ReportState::Failed
                } as i32,
                error: report.execution_error,
                writers_stopped: Some(report.writers_stopped),
                recording_finalized: Some(report.recording_finalized),
                service_exits: report.service_exits,
                boundary_clean: Some(report.boundary_clean),
                event_stream_healthy: Some(report.event_stream_healthy),
            },
            Err(error) => ExecutionResult {
                blocked,
                error: Some(error.to_string()),
                file_changes_state: ReportState::CheckpointUnknown as i32,
                ..Default::default()
            },
        };
        let rejection = result.error.clone().unwrap_or_else(|| "Internal".into());
        // Publish terminal facts before waking a rejected Start caller.
        publish_result(&updates, &final_capture, result);
        if let Some(reply) = admission.borrow_mut().take() {
            let _ = reply.send(Err(rejection));
        }
        let _ = done_tx.send(());
    });
    reply
}

/// Accept a systemd plan after captured inputs match Review and admission succeeds.
/// Readiness, machine capture, step events and completion use the shared update stream.
fn start_systemd(
    state: &mut State,
    run: workload_systemd::RunPlan,
    plan: workload_systemd::SystemdPlan,
    expected: workload_systemd::ConfigurationSnapshot,
    accepted: oneshot::Sender<Result<Vec<u8>, String>>,
    reply: Reply,
) -> Reply {
    let (stop_tx, stop_rx) = oneshot::channel();
    let (done_tx, done_rx) = oneshot::channel();
    let stopped = Rc::new(Cell::new(false));
    state.active = Some(Active {
        id: run.id.clone(),
        stop: Some(stop_tx),
        stopped: stopped.clone(),
        done: Some(done_rx),
    });
    *state.updates.lock().unwrap() = ExecutionUpdate {
        run_id: run.id.clone(),
        ..Default::default()
    };
    let updates = state.updates.clone();
    let runner = state.systemd.clone();
    let sink = updates.clone();
    let observer = TelemetryObserver::new(MAX_EVENT_BYTES, move |bytes| {
        let mut update = sink
            .lock()
            .map_err(|_| io::Error::other("event queue unavailable"))?;
        if update.events.len() == EVENT_QUEUE_CAPACITY {
            return Err(io::Error::other("event queue full"));
        }
        update
            .events
            .push(workflow_protocol::Event::decode(bytes).map_err(io::Error::other)?);
        Ok(())
    });
    let (ready_tx, ready_rx) = oneshot::channel();
    let readiness = updates.clone();
    let ready_id = run.id.clone();
    tokio::task::spawn_local(async move {
        if ready_rx.await.is_ok() {
            let mut update = readiness.lock().unwrap();
            if update.run_id == ready_id {
                update.ready = true;
            }
        }
    });
    tokio::task::spawn_local(async move {
        // Machine facts describe the disk that holds the run's artifacts.
        let data = std::path::Path::new(&run.artifacts)
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        {
            let before = system_config::observe(&data);
            let mut update = updates.lock().unwrap();
            (update.machine_before, update.machine_before_error) =
                crate::capture_parts(Some(before));
        }
        let admission = Rc::new(RefCell::new(Some(accepted)));
        let check = admission.clone();
        let mut runner = runner.lock().await;
        let loss = updates.clone();
        let result = runner
            .run_with_preflight(
                run,
                plan,
                async {
                    let _ = stop_rx.await;
                },
                move |event: &orchestrator::Event| {
                    // Omit bulky input/output payloads from the bounded transport.
                    observer.observe(&orchestrator::Event {
                        invocation: event.invocation.clone(),
                        state: event.state,
                        elapsed_ns: event.elapsed_ns,
                        duration_ns: event.duration_ns,
                        payload: None,
                        error: event.error.as_deref().map(bounded_error),
                    });
                    loss.lock().unwrap().dropped_events = observer.dropped_events();
                },
                Some(ready_tx),
                move |snapshot| {
                    if stopped.get() {
                        return Err("Stopped".into());
                    }
                    if *snapshot != expected {
                        return Err("ConfigurationChanged".into());
                    }
                    if let Some(reply) = check.borrow_mut().take() {
                        let _ = reply.send(Ok(Vec::new()));
                    }
                    Ok(())
                },
            )
            .await;
        let blocked = runner.blocked();
        let result = match result {
            Ok(report) => systemd_result(report, blocked),
            Err(error) => ExecutionResult {
                blocked,
                error: Some(error),
                ..Default::default()
            },
        };
        let rejection = result.error.clone().unwrap_or_else(|| "Internal".into());
        let after = system_config::observe(&data);
        {
            let mut update = updates.lock().unwrap();
            (update.machine_after, update.machine_after_error) = crate::capture_parts(Some(after));
            update.result = Some(result);
        }
        if let Some(reply) = admission.borrow_mut().take() {
            let _ = reply.send(Err(rejection));
        }
        let _ = done_tx.send(());
    });
    reply
}

/// Preserve execution and evidence failures while exposing only known unit exits.
fn systemd_result(report: workload_systemd::SystemdRunReport, blocked: bool) -> ExecutionResult {
    let errors: Vec<_> = report
        .execution_error
        .into_iter()
        .chain(report.diagnostics)
        .collect();
    ExecutionResult {
        blocked,
        error: (!errors.is_empty()).then(|| errors.join("\n")),
        writers_stopped: Some(report.writers_stopped),
        recording_finalized: Some(report.recording_finalized),
        service_exits: report.units.into_iter().filter_map(unit_exit).collect(),
        boundary_clean: Some(report.cleanup_verified),
        // systemd runs have no Docker event stream to lose.
        event_stream_healthy: Some(true),
        ..Default::default()
    }
}

/// How a unit ended, in the shape Console shows for every backend.
fn unit_exit(unit: workload_systemd::UnitExit) -> Option<compose_apps::compose::ServiceExit> {
    // `CLD_EXITED` from waitid(2); other codes mean a signal ended the main process.
    const CLD_EXITED: i32 = 1;
    const SIGNAL_EXIT_BASE: i32 = 128;
    const SIGKILL: i32 = 9;
    let code = unit.exec_main_code?;
    let status = unit.exec_main_status?;
    let result = unit.result.as_ref()?;
    let signalled = code != 0 && code != CLD_EXITED;
    let oom_killed = result == "oom-kill";
    Some(compose_apps::compose::ServiceExit {
        service: unit.id().to_owned(),
        exit_code: if signalled {
            SIGNAL_EXIT_BASE + status
        } else {
            status
        },
        oom_killed,
        killed: oom_killed || (signalled && status == SIGKILL),
    })
}

/// Publish terminal state and the latest capture together before polling can finish.
fn publish_result(
    updates: &Mutex<ExecutionUpdate>,
    machine: &watch::Receiver<MachineCapture>,
    result: ExecutionResult,
) {
    let capture = machine.borrow().clone();
    let mut update = updates.lock().unwrap();
    (update.machine_before, update.machine_before_error) = crate::capture_parts(capture.before);
    (update.machine_after, update.machine_after_error) = crate::capture_parts(capture.after);
    update.result = Some(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket_directory(label: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "console-native-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    #[test]
    fn compose_retries_failed_connection_and_shares_successful_initialization() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Arrange
        let directory = socket_directory("retry");
        let socket = directory.join("docker.sock");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let state = State::open(socket.to_str().unwrap().into(), Arc::default())
                .await
                .unwrap();

            // Act / Assert: a missing daemon fails only when Compose is requested.
            assert!(state.compose.get().await.is_err());
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let engine = async {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.ends_with(b"\r\n\r\n") {
                    let count = connection.read(&mut buffer).await.unwrap();
                    assert_ne!(count, 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                assert!(request.starts_with(b"GET /version HTTP/1.1\r\n"));
                let body = r#"{"ApiVersion":"1.47"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                connection.write_all(response.as_bytes()).await.unwrap();
            };

            // Act: concurrent callers retry against the now-available daemon.
            let (first, second, ()) =
                tokio::join!(state.compose.get(), state.compose.get(), engine);

            // Assert: both requests share the same negotiated adapter and runner.
            assert!(std::ptr::eq(first.unwrap(), second.unwrap()));
        });
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn stop_cancels_pending_docker_connection() {
        // Arrange
        let directory = socket_directory("stop");
        let socket = directory.join("docker.sock");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio::task::LocalSet::new().block_on(&runtime, async {
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let updates = Arc::new(Mutex::new(ExecutionUpdate::default()));
            let mut state = State::open(socket.to_str().unwrap().into(), updates.clone())
                .await
                .unwrap();
            let request = StartRequest {
                run: Some(RunRequest {
                    run: Some(workload_lifecycle::launch::RunPlan {
                        host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough
                            as i32,
                        id: "pending".into(),
                        repository_root: directory.to_str().unwrap().into(),
                        artifacts: directory.join("run").to_str().unwrap().into(),
                        readiness_seconds: 1,
                        stop_seconds: 1,
                        ..Default::default()
                    }),
                    backend: Some(run_request::Backend::Compose(Default::default())),
                }),
                backend: Some(start_request::Backend::ComposeExpected(Default::default())),
            };

            // Act: stop after Docker accepts the connection but before it responds.
            let accepted = start(&mut state, request);
            let (_connection, _) = listener.accept().await.unwrap();
            state.stop("pending").unwrap();

            // Assert: admission and completion finish without a Docker response.
            assert_eq!(accepted.await.unwrap().unwrap_err(), "Stopped");
            state.shutdown().unwrap().await.unwrap();
            assert_eq!(
                updates
                    .lock()
                    .unwrap()
                    .result
                    .as_ref()
                    .unwrap()
                    .error
                    .as_deref(),
                Some("Stopped")
            );
        });
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn start_rejects_missing_or_wrong_backend_snapshots_before_admission() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut state = State::open("/nonexistent/docker.sock".into(), Arc::default())
                .await
                .unwrap();
            for (backend, wrong) in [
                (
                    run_request::Backend::Compose(Default::default()),
                    start_request::Backend::SystemdExpected(Default::default()),
                ),
                (
                    run_request::Backend::Systemd(Default::default()),
                    start_request::Backend::ComposeExpected(Default::default()),
                ),
            ] {
                for expected in [None, Some(wrong)] {
                    let request = StartRequest {
                        run: Some(RunRequest {
                            run: Some(workload_lifecycle::launch::RunPlan {
                                host_shm_access:
                                    workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
                                ..Default::default()
                            }),
                            backend: Some(backend.clone()),
                        }),
                        backend: expected,
                    };

                    // Act / Assert
                    assert_eq!(
                        start(&mut state, request).await.unwrap().unwrap_err(),
                        "InvalidInput"
                    );
                    assert!(state.active.is_none());
                    assert!(state.updates.lock().unwrap().run_id.is_empty());
                }
            }
        });
    }

    #[test]
    #[ignore = "requires root and a running systemd manager"]
    fn systemd_stop_latched_before_preflight_rejects_admission() {
        // Arrange
        let directory = socket_directory("systemd-stop");
        let artifacts = directory.join("run");
        let run = workload_lifecycle::launch::RunPlan {
            host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
            id: "stopped-before-preflight".into(),
            repository_root: directory.to_str().unwrap().into(),
            artifacts: artifacts.to_str().unwrap().into(),
            readiness_seconds: 5,
            stop_seconds: 1,
            ..Default::default()
        };
        let plan = workload_systemd::SystemdPlan {
            programs: vec![workload_systemd::Program {
                id: "workload".into(),
                argv: vec!["/bin/sleep".into(), "300".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio::task::LocalSet::new().block_on(&runtime, async {
            let expected = workload_systemd::inspect(&run, &plan).await.unwrap();
            let mut state = State::open("/nonexistent/docker.sock".into(), Arc::default())
                .await
                .unwrap();
            let request = StartRequest {
                run: Some(RunRequest {
                    run: Some(run),
                    backend: Some(run_request::Backend::Systemd(plan)),
                }),
                backend: Some(start_request::Backend::SystemdExpected(expected)),
            };

            // Act: latch Stop on the owner before yielding to the preparation task.
            let admission = start(&mut state, request);
            state.stop("stopped-before-preflight").unwrap();
            let result = admission.await.unwrap();
            let published = state.updates.lock().unwrap().result.clone();
            state.shutdown().unwrap().await.unwrap();

            // Assert: rejection publishes terminal facts and removes captured inputs.
            assert_eq!(result.unwrap_err(), "Stopped");
            let published = published.expect("terminal result precedes rejected admission");
            assert_eq!(published.error.as_deref(), Some("Stopped"));
            assert!(!published.blocked);
            assert!(!artifacts.exists());
        });
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn systemd_evidence_failures_remain_visible_without_invented_exit_codes() {
        // Arrange
        let report = workload_systemd::SystemdRunReport {
            id: "run".into(),
            execution_error: Some("start failed".into()),
            writers_stopped: true,
            recording_finalized: false,
            cleanup_verified: true,
            diagnostics: vec!["worker: required exit evidence unavailable".into()],
            units: vec![workload_systemd::UnitExit {
                unit: "doramcap-scope-worker.service".into(),
                result: None,
                exec_main_code: None,
                exec_main_status: None,
                cpu_usage_nsec: None,
                memory_peak_bytes: None,
            }],
        };
        // Act
        let result = systemd_result(report, false);
        // Assert
        assert_eq!(
            result.error.as_deref(),
            Some("start failed\nworker: required exit evidence unavailable")
        );
        assert!(result.service_exits.is_empty());
        assert_eq!(result.writers_stopped, Some(true));
        assert_eq!(result.recording_finalized, Some(false));
        assert_eq!(result.boundary_clean, Some(true));
    }

    #[test]
    fn systemd_known_exit_status_preserves_the_app_identity() {
        // Arrange
        let unit = workload_systemd::UnitExit {
            unit: "doramcap-scope-worker-recorder.service".into(),
            result: Some("exit-code".into()),
            exec_main_code: Some(1),
            exec_main_status: Some(7),
            cpu_usage_nsec: Some(0),
            memory_peak_bytes: Some(0),
        };
        // Act
        let exit = unit_exit(unit).unwrap();
        // Assert
        assert_eq!(exit.service, "worker-recorder");
        assert_eq!(exit.exit_code, 7);
        assert!(!exit.oom_killed);
        assert!(!exit.killed);
    }

    #[test]
    fn a_failed_result_keeps_capture_before_the_listener_runs() {
        // Arrange
        let (sender, receiver) = watch::channel(MachineCapture::default());
        let updates = Mutex::new(ExecutionUpdate::default());
        sender
            .send(MachineCapture {
                before: Some(Ok(system_config::MachineState {
                    captured_unix_ms: 1,
                    ..Default::default()
                })),
                after: Some(Ok(system_config::MachineState {
                    captured_unix_ms: 2,
                    ..Default::default()
                })),
            })
            .unwrap();
        // Act
        publish_result(
            &updates,
            &receiver,
            ExecutionResult {
                error: Some("failed".into()),
                ..Default::default()
            },
        );
        let update =
            ExecutionUpdate::decode(updates.lock().unwrap().encode_to_vec().as_slice()).unwrap();
        // Assert
        assert_eq!(update.result.unwrap().error.as_deref(), Some("failed"));
        assert_eq!(update.machine_before.unwrap().captured_unix_ms, 1);
        assert_eq!(update.machine_after.unwrap().captured_unix_ms, 2);
    }

    #[test]
    fn observation_errors_reach_the_console_without_failing_the_run() {
        // Arrange
        let (_, receiver) = watch::channel(MachineCapture {
            before: Some(Err("/dev/shm: permission denied".into())),
            after: Some(Err("/dev/shm: entry disappeared".into())),
        });
        let updates = Mutex::new(ExecutionUpdate::default());
        // Act
        publish_result(&updates, &receiver, ExecutionResult::default());
        let update =
            ExecutionUpdate::decode(updates.lock().unwrap().encode_to_vec().as_slice()).unwrap();
        // Assert
        assert!(update.machine_before.is_none());
        assert!(update.machine_after.is_none());
        assert!(update.machine_before_error.contains("permission denied"));
        assert!(update.machine_after_error.contains("entry disappeared"));
        assert!(update.result.unwrap().error.is_none());
    }

    #[test]
    fn a_long_error_keeps_its_end_within_the_event_limit() {
        // Arrange
        let error = format!("{} the cause", "é".repeat(MAX_EVENT_BYTES));
        // Act
        let bounded = bounded_error(&error);
        // Assert
        assert_eq!(bounded.chars().count(), MAX_ERROR_CHARS);
        assert!(bounded.len() < MAX_EVENT_BYTES);
        assert!(bounded.ends_with("the cause"));
        assert_eq!(bounded_error("short"), "short");
    }
}
