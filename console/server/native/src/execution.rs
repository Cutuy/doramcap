//! Active workflow resources and bounded event transport; Console state lives in TypeScript.
use crate::protocol::*;
use orchestrator::{Observer, telemetry::TelemetryObserver};
use prost::Message;
use std::{
    cell::{Cell, RefCell},
    io,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use workload_docker::{
    compose::DockerAdapter,
    workflows::recorded_compose::{ComposeRun, ComposeRunner, Recording, RunSignals},
};

const EVENT_QUEUE_CAPACITY: usize = 512;
const MAX_EVENT_BYTES: usize = 16 * 1024;
pub(crate) type Reply = oneshot::Receiver<Result<Vec<u8>, String>>;
struct Active {
    id: String,
    stop: Option<oneshot::Sender<()>>,
    stopped: Rc<Cell<bool>>,
    done: Option<oneshot::Receiver<()>>,
}
pub struct State {
    pub adapter: Rc<DockerAdapter>,
    runner: Rc<tokio::sync::Mutex<ComposeRunner>>,
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
pub fn run(request: RunRequest) -> Result<ComposeRun, String> {
    Ok(ComposeRun {
        id: request.id,
        repository_root: request.repository_root.into(),
        artifacts: request.artifacts.into(),
        // Console executions retain the workflow's normal OverlayFS isolation.
        filesystem: Some(Default::default()),
        system_apps: request.system_apps,
        projects: request.projects,
        dora: request.dora.unwrap_or_default(),
        recordings: request
            .recordings
            .into_iter()
            .map(|r| Recording {
                stream: PathBuf::from(r.stream),
                recording: PathBuf::from(r.recording),
            })
            .collect(),
        readiness_seconds: request.readiness_seconds,
        stop_seconds: request.stop_seconds,
    })
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
    let spec = match request.run.ok_or("InvalidInput".to_string()).and_then(run) {
        Ok(spec) => spec,
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
    tokio::task::spawn_local(async move {
        if teardown_rx.await.is_ok() {
            let mut update = teardown.lock().unwrap();
            if update.run_id == run_id {
                update.teardown_started = true;
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
                        error: event.error.clone(),
                    });
                    loss.lock().unwrap().dropped_events = observe.dropped_events();
                },
                RunSignals {
                    ready: Some(ready_tx),
                    quiescing: Some(teardown_tx),
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
                boundary_clean: Some(report.boundary_clean),
                event_stream_healthy: Some(report.event_stream_healthy),
            },
            Err(error) => ExecutionResult {
                blocked,
                error: Some(error.to_string()),
                ..Default::default()
            },
        };
        if let Some(reply) = admission.borrow_mut().take() {
            let _ = reply.send(Err("Internal".into()));
        }
        updates.lock().unwrap().result = Some(result);
        let _ = done_tx.send(());
    });
    reply
}
