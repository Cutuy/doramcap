//! Active workflow resources and bounded event transport; Console state lives in TypeScript.
use crate::protocol::*;
use orchestrator::{Observer, telemetry::TelemetryObserver};
use prost::Message;
use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::{oneshot, watch};
use workload_docker::{
    compose::DockerAdapter,
    workflows::recorded_compose::{ComposeRun, ComposeRunner, MachineCapture, RunSignals},
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
pub struct State {
    pub adapter: Rc<DockerAdapter>,
    runner: Rc<tokio::sync::Mutex<ComposeRunner>>,
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
        let adapter = Rc::new(
            DockerAdapter::connect(socket)
                .await
                .map_err(|e| e.to_string())?,
        );
        Ok(Self {
            runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))),
            systemd: Rc::default(),
            adapter,
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
        Some(run_request::Backend::Compose(compose)) => {
            ComposeRun::from_plan(run, compose)
                .map(Launch::Compose)
                .map_err(|error| error.to_string())
        }
        Some(run_request::Backend::Systemd(systemd)) => Ok(Launch::Systemd(run, systemd)),
        None => Err("run request has no backend".into()),
    }
}
pub fn start(state: &mut State, request: StartRequest) -> Reply {
    let (accepted, reply) = oneshot::channel();
    if state.closing || state.active.is_some() && state.updates.lock().unwrap().result.is_none() {
        let _ = accepted.send(Err("Conflict".into()));
        return reply;
    }
    let Some(expected) = request.expected else {
        let _ = accepted.send(Err("InvalidInput".into()));
        return reply;
    };
    let spec = match request
        .run
        .ok_or("InvalidInput".to_string())
        .and_then(launch)
    {
        Ok(Launch::Compose(spec)) => spec,
        Ok(Launch::Systemd(run, plan)) => return start_systemd(state, run, plan, accepted, reply),
        Err(error) => {
            let _ = accepted.send(Err(error));
            return reply;
        }
    };
    let (stop_tx, stop_rx) = oneshot::channel();
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
    let runner = state.runner.clone();
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
                update.machine_before = capture.before;
                update.machine_after = capture.after;
            }
        }
    });
    tokio::task::spawn_local(async move {
        let admission = Rc::new(RefCell::new(Some(accepted)));
        let check = admission.clone();
        let mut runner = runner.lock().await;
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
                        return Err(workload_docker::compose::Error::Invalid(error.into()));
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

/// Run a systemd plan: accept at once, then report readiness, step events, the machine
/// before and after, and the result through the same update as Compose runs.
fn start_systemd(
    state: &mut State,
    run: workload_systemd::RunPlan,
    plan: workload_systemd::SystemdPlan,
    accepted: oneshot::Sender<Result<Vec<u8>, String>>,
    reply: Reply,
) -> Reply {
    let (stop_tx, stop_rx) = oneshot::channel();
    let (done_tx, done_rx) = oneshot::channel();
    state.active = Some(Active {
        id: run.id.clone(),
        stop: Some(stop_tx),
        stopped: Rc::new(Cell::new(false)),
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
        updates.lock().unwrap().machine_before = Some(system_config::observe(&data));
        let _ = accepted.send(Ok(Vec::new()));
        let mut runner = runner.lock().await;
        let loss = updates.clone();
        let result = runner
            .run(
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
            )
            .await;
        let blocked = runner.blocked();
        let result = match result {
            Ok(report) => ExecutionResult {
                blocked,
                error: report.execution_error,
                writers_stopped: Some(report.writers_stopped),
                recording_finalized: Some(report.recording_finalized),
                service_exits: report.units.into_iter().map(unit_exit).collect(),
                boundary_clean: Some(report.cleanup_verified),
                // systemd runs have no Docker event stream to lose.
                event_stream_healthy: Some(true),
            },
            Err(error) => ExecutionResult {
                blocked,
                error: Some(error),
                ..Default::default()
            },
        };
        let after = system_config::observe(&data);
        {
            let mut update = updates.lock().unwrap();
            update.machine_after = Some(after);
            update.result = Some(result);
        }
        let _ = done_tx.send(());
    });
    reply
}

/// How a unit ended, in the shape Console shows for every backend.
fn unit_exit(unit: workload_systemd::UnitExit) -> workload_docker::compose::ServiceExit {
    // `CLD_EXITED` from waitid(2); other codes mean a signal ended the main process.
    const CLD_EXITED: i32 = 1;
    const SIGNAL_EXIT_BASE: i32 = 128;
    const SIGKILL: i32 = 9;
    // Unreadable evidence shows as zero here; the run report already fails with a diagnostic.
    let code = unit.exec_main_code.unwrap_or_default();
    let status = unit.exec_main_status.unwrap_or_default();
    let signalled = code != 0 && code != CLD_EXITED;
    let oom_killed = unit.result.as_deref() == Some("oom-kill");
    workload_docker::compose::ServiceExit {
        service: unit.id,
        exit_code: if signalled {
            SIGNAL_EXIT_BASE + status
        } else {
            status
        },
        oom_killed,
        killed: oom_killed || (signalled && status == SIGKILL),
    }
}

/// Publish terminal state and the latest capture together before polling can finish.
fn publish_result(
    updates: &Mutex<ExecutionUpdate>,
    machine: &watch::Receiver<MachineCapture>,
    result: ExecutionResult,
) {
    let capture = machine.borrow().clone();
    let mut update = updates.lock().unwrap();
    update.machine_before = capture.before;
    update.machine_after = capture.after;
    update.result = Some(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_result_keeps_capture_before_the_listener_runs() {
        // Arrange
        let (sender, receiver) = watch::channel(MachineCapture::default());
        let updates = Mutex::new(ExecutionUpdate::default());
        sender
            .send(MachineCapture {
                before: Some(system_config::MachineState {
                    captured_unix_ms: 1,
                    ..Default::default()
                }),
                after: Some(system_config::MachineState {
                    captured_unix_ms: 2,
                    ..Default::default()
                }),
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
