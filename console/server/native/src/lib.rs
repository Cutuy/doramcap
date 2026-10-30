//! Native contracts and execution owner for the Console.
pub mod config;
mod metrics;
pub mod prepare;
pub mod progress;
pub mod project;
pub mod runtime;
mod session;
pub mod state;

pub mod protocol {
    include!(concat!(env!("OUT_DIR"), "/doramcap.console.v1.rs"));
}

#[cfg(test)]
mod tests {
    use super::protocol::Status;
    use prost::Message;

    #[test]
    fn status_preserves_large_sequence_number() {
        // Arrange
        let status = Status {
            sequence: (1_u64 << 53) + 1,
            ..Default::default()
        };
        // Act
        let decoded = Status::decode(status.encode_to_vec().as_slice()).unwrap();
        // Assert
        assert_eq!(decoded.sequence, status.sequence);
    }
}

use napi::{
    Env, Error, Result, Status,
    bindgen_prelude::{AsyncBlock, AsyncBlockBuilder, Buffer},
};
use napi_derive::napi;
use prost::Message;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use workload_docker::{compose::DockerAdapter, workflows::recorded_compose::ComposeRunner};

const CONTRACT_VERSION: u32 = 1;

use session::{NativeState, Session};

/// JavaScript handle to the dedicated execution owner.
#[napi]
pub struct Console {
    owner: Mutex<Option<runtime::Owner<NativeState>>>,
    close_lock: tokio::sync::Mutex<()>,
    snapshot: Arc<Mutex<protocol::Status>>,
    loss: Arc<AtomicU64>,
    data_root: String,
}

/// Validate configuration and construct Docker execution state on the owner thread.
#[napi]
pub async fn open(config_path: String, contract_version: u32) -> Result<Console> {
    if contract_version != CONTRACT_VERSION {
        return Err(Error::new(
            Status::InvalidArg,
            "Console contract version mismatch",
        ));
    }
    let config = config::load(std::path::Path::new(&config_path))
        .map_err(|e| Error::new(Status::InvalidArg, e))?;
    let data_root = config.data_root.clone();
    let snapshot = Arc::new(Mutex::new(protocol::Status::default()));
    let loss = Arc::new(AtomicU64::new(0));
    let owner_snapshot = snapshot.clone();
    let owner_loss = loss.clone();
    let owner = runtime::Owner::open(move || async move {
        let (store, restored) = state::StateStore::open(std::path::Path::new(&config.data_root))?;
        let (phase, marker) = match restored {
            state::Restored::Idle => (protocol::Phase::Idle, None),
            state::Restored::Finished(marker) => (protocol::Phase::Finished, Some(marker)),
            state::Restored::Blocked(marker) => (protocol::Phase::Blocked, marker),
        };
        let adapter = Rc::new(
            DockerAdapter::connect(config.docker_socket.clone())
                .await
                .map_err(|e| e.to_string())?,
        );
        let mut session = Session::new(phase, marker);
        session.snapshot = owner_snapshot;
        session.loss = owner_loss;
        session.publish();
        metrics::restore(&mut session, std::path::Path::new(&config.data_root));
        session.publish();
        Ok::<_, String>(NativeState {
            runner: Rc::new(tokio::sync::Mutex::new(ComposeRunner::new(adapter.clone()))),
            store: Rc::new(store),
            adapter,
            config,
            session: Rc::new(RefCell::new(session)),
        })
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, e))?;
    Ok(Console {
        owner: Mutex::new(Some(owner)),
        close_lock: tokio::sync::Mutex::new(()),
        snapshot,
        loss,
        data_root,
    })
}

#[napi]
impl Console {
    /// Return host-only server wiring; never expose this record over HTTP.
    #[napi]
    pub fn settings(&self, env: Env) -> Result<AsyncBlock<Buffer>> {
        let guard = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?;
        let owner = guard
            .as_ref()
            .ok_or_else(|| Error::new(Status::Closing, "Console is closed"))?;
        let reply = owner
            .dispatch_control(|state| {
                protocol::ServerSettings {
                    listen_address: state.config.listen_address.clone(),
                    access_secret_file: state.config.access_secret_file.clone(),
                    public_origin: state.config.public_origin.clone(),
                }
                .encode_to_vec()
            })
            .map_err(dispatch_error)?;
        drop(guard);
        AsyncBlockBuilder::new(async move {
            let bytes = reply
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "owner unavailable"))?;
            Ok(Buffer::from(bytes))
        })
        .build(&env)
    }

    /// Prepare a project using an owned protobuf request.
    #[napi]
    pub fn prepare(&self, env: Env, input: Buffer) -> Result<AsyncBlock<Buffer>> {
        let request = protocol::PrepareRequest::decode(input.as_ref())
            .map_err(|_| Error::new(Status::InvalidArg, "invalid prepare request"))?;
        let guard = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?;
        let owner = guard
            .as_ref()
            .ok_or_else(|| Error::new(Status::Closing, "Console is closed"))?;
        let reply = owner
            .dispatch(move |state| {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let mut session = state.session.borrow_mut();
                if session.closing
                    || !matches!(
                        session.phase,
                        protocol::Phase::Idle
                            | protocol::Phase::Reviewed
                            | protocol::Phase::Finished
                    )
                {
                    let _ = sender.send(Err("Conflict".to_owned()));
                    return receiver;
                }
                if let Some(old) = session.prepared.take() {
                    let _ = std::fs::remove_dir_all(old.directory);
                }
                session.phase = protocol::Phase::Preparing;
                session.start_key = None;
                session.start_result = None;
                session.publish();
                drop(session);
                let adapter = state.adapter.clone();
                let config = state.config.clone();
                let session = state.session.clone();
                tokio::task::spawn_local(async move {
                    let result = prepare::prepare(adapter, &config, request).await;
                    let mut state = session.borrow_mut();
                    match result {
                        Ok(prepared) => {
                            let bytes = prepared.review.encode_to_vec();
                            if prepared.review.prepared_id.is_empty() {
                                state.phase = protocol::Phase::Idle;
                            } else {
                                state.prepared = Some(prepared);
                                state.phase = protocol::Phase::Reviewed;
                            }
                            let _ = sender.send(Ok(bytes));
                        }
                        Err(error) => {
                            prepare::save_diagnostic(&config, &error);
                            state.phase = protocol::Phase::Idle;
                            let _ = sender.send(Err("InvalidInput".to_owned()));
                        }
                    }
                    state.publish();
                    state.finish_shutdown();
                });
                receiver
            })
            .map_err(dispatch_error)?;
        drop(guard);
        AsyncBlockBuilder::new(async move {
            let receiver = reply
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "owner unavailable"))?;
            let bytes = receiver
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "preparation interrupted"))?
                .map_err(|e| Error::new(Status::GenericFailure, e))?;
            Ok(Buffer::from(bytes))
        })
        .build(&env)
    }

    /// Start the reviewed run or join an earlier request with the same key.
    #[napi]
    pub fn start(&self, env: Env, input: Buffer) -> Result<AsyncBlock<Buffer>> {
        let request = protocol::StartRequest::decode(input.as_ref())
            .map_err(|_| Error::new(Status::InvalidArg, "InvalidInput"))?;
        let guard = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?;
        let owner = guard
            .as_ref()
            .ok_or_else(|| Error::new(Status::Closing, "Console is closed"))?;
        let reply = owner
            .dispatch(move |state| session::start(state, request))
            .map_err(dispatch_error)?;
        drop(guard);
        AsyncBlockBuilder::new(async move {
            let receiver = reply
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "Internal"))?;
            let run_id = receiver
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "Internal"))?
                .map_err(|error| Error::new(Status::GenericFailure, error))?;
            Ok(Buffer::from(
                protocol::StartResponse { run_id }.encode_to_vec(),
            ))
        })
        .build(&env)
    }

    /// Signal a cooperative stop on the reserved control channel.
    #[napi]
    pub fn stop(&self, env: Env, input: Buffer) -> Result<AsyncBlock<Buffer>> {
        let request = protocol::StopRequest::decode(input.as_ref())
            .map_err(|_| Error::new(Status::InvalidArg, "InvalidInput"))?;
        let guard = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?;
        let owner = guard
            .as_ref()
            .ok_or_else(|| Error::new(Status::Closing, "Console is closed"))?;
        let reply = owner
            .dispatch_control(move |state| {
                let mut session = state.session.borrow_mut();
                if session
                    .marker
                    .as_ref()
                    .is_none_or(|marker| marker.run_id != request.run_id)
                {
                    return Err("Conflict");
                }
                if matches!(
                    session.phase,
                    protocol::Phase::Starting
                        | protocol::Phase::Running
                        | protocol::Phase::Stopping
                ) {
                    session.stop_run();
                } else if !matches!(
                    session.phase,
                    protocol::Phase::Finished | protocol::Phase::Blocked | protocol::Phase::Idle
                ) {
                    return Err("Conflict");
                }
                Ok(protocol::StopResponse { accepted: true }.encode_to_vec())
            })
            .map_err(dispatch_error)?;
        drop(guard);
        AsyncBlockBuilder::new(async move {
            let bytes = reply
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "Internal"))?
                .map_err(|error| Error::new(Status::GenericFailure, error))?;
            Ok(Buffer::from(bytes))
        })
        .build(&env)
    }

    /// Read the current snapshot independently of the owner mailbox.
    #[napi]
    pub fn status(&self, env: Env) -> Result<AsyncBlock<Buffer>> {
        if self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?
            .is_none()
        {
            return Err(Error::new(Status::Closing, "Console is closed"));
        }
        let mut status = self
            .snapshot
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "snapshot lock poisoned"))?
            .clone();
        status.lost_events = self.loss.load(Ordering::Relaxed);
        status.progress_incomplete |= status.lost_events != 0;
        let data_root = self.data_root.clone();
        AsyncBlockBuilder::new(async move {
            status.cpu_available = open_snapshot_artifact(&data_root, &status, "cpu").is_ok();
            status.report_available = open_snapshot_artifact(&data_root, &status, "report").is_ok();
            Ok(Buffer::from(status.encode_to_vec()))
        })
        .build(&env)
    }

    /// Return a Rust-encoded checkpoint or delta and its opaque next cursor.
    #[napi]
    pub fn update(&self, env: Env, cursor: Buffer) -> Result<AsyncBlock<LiveUpdate>> {
        if self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?
            .is_none()
        {
            return Err(Error::new(Status::Closing, "Console is closed"));
        }
        let cursor = protocol::StreamCursor::decode(cursor.as_ref())
            .map_err(|_| Error::new(Status::InvalidArg, "invalid stream cursor"))?;
        let status = self
            .snapshot
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "snapshot lock poisoned"))?
            .clone();
        let data_root = self.data_root.clone();
        let lost = self.loss.load(Ordering::Relaxed);
        AsyncBlockBuilder::new(async move {
            let (mut status, cursor) = stream_update(status, &cursor);
            status.lost_events = lost;
            status.progress_incomplete |= lost != 0;
            status.cpu_available = open_snapshot_artifact(&data_root, &status, "cpu").is_ok();
            status.report_available = open_snapshot_artifact(&data_root, &status, "report").is_ok();
            Ok(LiveUpdate {
                data: Buffer::from(status.encode_to_vec()),
                cursor: Buffer::from(cursor.encode_to_vec()),
            })
        })
        .build(&env)
    }

    /// Return the current snapshot, including any completed outcome.
    #[napi]
    pub fn completed(&self, env: Env) -> Result<AsyncBlock<Buffer>> {
        self.status(env)
    }

    /// Open an authorized current-run artifact and transfer its descriptor to Node.
    #[napi]
    pub fn open_artifact(&self, env: Env, run_id: String, kind: String) -> Result<AsyncBlock<i32>> {
        if run_id.len() != 32
            || !run_id
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(Error::new(Status::InvalidArg, "Missing"));
        }
        if kind != "cpu" && kind != "report" {
            return Err(Error::new(Status::InvalidArg, "Missing"));
        }
        let guard = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?;
        let owner = guard
            .as_ref()
            .ok_or_else(|| Error::new(Status::Closing, "Console is closed"))?;
        let reply = owner
            .dispatch_control(move |state| {
                let session = state.session.borrow();
                open_artifact_file(&state.config, &session, &run_id, &kind)
            })
            .map_err(dispatch_error)?;
        drop(guard);
        AsyncBlockBuilder::build_with_map(
            &env,
            async move {
                reply
                    .await
                    .map_err(|_| Error::new(Status::GenericFailure, "Internal"))?
                    .map_err(|message| Error::new(Status::GenericFailure, message))
            },
            |_env, file| {
                use std::os::fd::IntoRawFd;
                Ok(file.into_raw_fd())
            },
        )
    }

    /// Close the owner and await its thread.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let _close = self.close_lock.lock().await;
        let owner = self
            .owner
            .lock()
            .map_err(|_| Error::new(Status::GenericFailure, "owner lock poisoned"))?
            .take();
        if let Some(owner) = owner {
            let reply = owner
                .dispatch_control(|state| {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    let mut session = state.session.borrow_mut();
                    session.closing = true;
                    if matches!(
                        session.phase,
                        protocol::Phase::Starting
                            | protocol::Phase::Running
                            | protocol::Phase::Stopping
                    ) {
                        session.stop_run();
                        session.shutdown_waiters.push(sender);
                    } else if session.phase == protocol::Phase::Preparing {
                        session.shutdown_waiters.push(sender);
                    } else {
                        let _ = sender.send(());
                    }
                    receiver
                })
                .map_err(dispatch_error)?;
            let completed = reply
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "owner unavailable"))?;
            completed
                .await
                .map_err(|_| Error::new(Status::GenericFailure, "shutdown interrupted"))?;
            owner
                .close()
                .await
                .map_err(|e| Error::new(Status::GenericFailure, e))?;
        }
        Ok(())
    }
}

fn dispatch_error(error: runtime::DispatchError) -> Error {
    match error {
        runtime::DispatchError::Busy => Error::new(Status::QueueFull, "owner busy"),
        runtime::DispatchError::Closed => Error::new(Status::Closing, "owner closed"),
    }
}

fn open_artifact_file(
    config: &protocol::ConsoleConfig,
    session: &Session,
    run_id: &str,
    kind: &str,
) -> std::result::Result<std::fs::File, &'static str> {
    if run_id.len() != 32
        || !run_id
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err("Missing");
    }
    let marker = session.marker.as_ref().ok_or("Missing")?;
    if marker.run_id != run_id
        || !matches!(
            session.phase,
            protocol::Phase::Finished | protocol::Phase::Blocked
        )
    {
        return Err("Missing");
    }
    let file_name = match kind {
        "cpu" if marker.outcome.as_ref().and_then(|o| o.recording_finalized) == Some(true) => {
            "cpu.mcap"
        }
        "report" => "console-report.json",
        _ => return Err("Missing"),
    };
    let root = workload_docker::evidence::root(
        std::path::Path::new(&config.data_root)
            .join("runs")
            .as_path(),
    )
    .map_err(|_| "Missing")?;
    workload_docker::evidence::open(&root, &format!("{run_id}/{file_name}"), false)
        .map_err(|_| "Missing")
}

fn open_snapshot_artifact(
    data_root: &str,
    status: &protocol::Status,
    kind: &str,
) -> std::result::Result<std::fs::File, &'static str> {
    let run_id = &status.run_id;
    if run_id.len() != 32
        || !run_id
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        || !matches!(
            protocol::Phase::try_from(status.phase),
            Ok(protocol::Phase::Finished | protocol::Phase::Blocked)
        )
    {
        return Err("Missing");
    }
    let name = match kind {
        "cpu" if status.outcome.as_ref().and_then(|o| o.recording_finalized) == Some(true) => {
            "cpu.mcap"
        }
        "report" => "console-report.json",
        _ => return Err("Missing"),
    };
    let root =
        workload_docker::evidence::root(std::path::Path::new(data_root).join("runs").as_path())
            .map_err(|_| "Missing")?;
    workload_docker::evidence::open(&root, &format!("{run_id}/{name}"), false)
        .map_err(|_| "Missing")
}

/// Binary update plus opaque Rust-owned stream position.
#[napi(object)]
pub struct LiveUpdate {
    pub data: Buffer,
    pub cursor: Buffer,
}

fn stream_update(
    mut status: protocol::Status,
    cursor: &protocol::StreamCursor,
) -> (protocol::Status, protocol::StreamCursor) {
    let next = protocol::StreamCursor {
        run_id: status.run_id.clone(),
        sequence: status.sequence,
        metric_elapsed_ns: status.metrics.last().map_or(0, |metric| metric.elapsed_ns),
        cpu_sequence: status
            .cpu_snapshots
            .last()
            .map_or(0, |snapshot| snapshot.sequence),
    };
    if !cursor.run_id.is_empty()
        && cursor.run_id == status.run_id
        && cursor.sequence <= status.sequence
        && cursor.metric_elapsed_ns <= next.metric_elapsed_ns
    {
        status
            .steps
            .retain(|event| event.sequence > cursor.sequence);
        status
            .metrics
            .retain(|metric| metric.elapsed_ns > cursor.metric_elapsed_ns);
        if cursor.cpu_sequence == next.cpu_sequence {
            status.cpu_snapshots.clear();
        }
    }
    (status, next)
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test]
    fn checkpoint_and_deltas_preserve_exact_ids_and_only_send_changed_rows() {
        // Arrange
        let id = (1_u64 << 53) + 1;
        let status = protocol::Status {
            run_id: "run".into(),
            sequence: id,
            steps: vec![workflow_protocol::Event {
                sequence: id,
                invocation: id,
                ..Default::default()
            }],
            metrics: vec![protocol::RunMetric {
                elapsed_ns: id,
                ..Default::default()
            }],
            cpu_snapshots: vec![telemetry_protocol::pb::Snapshot {
                sequence: id,
                ..Default::default()
            }],
            ..Default::default()
        };
        // Act
        let (checkpoint, cursor) =
            stream_update(status.clone(), &protocol::StreamCursor::default());
        let (heartbeat, _) = stream_update(status.clone(), &cursor);
        // Assert
        assert_eq!(checkpoint.steps[0].invocation, id);
        assert_eq!(
            protocol::StreamCursor::decode(cursor.encode_to_vec().as_slice())
                .unwrap()
                .sequence,
            id
        );
        assert!(
            heartbeat.steps.is_empty()
                && heartbeat.metrics.is_empty()
                && heartbeat.cpu_snapshots.is_empty()
        );
        // Act
        let mut changed = status;
        changed.sequence += 1;
        changed.steps.push(workflow_protocol::Event {
            sequence: id + 1,
            invocation: id + 1,
            ..Default::default()
        });
        changed.cpu_snapshots[0].sequence += 1;
        changed.metrics.push(protocol::RunMetric {
            elapsed_ns: id + 1,
            ..Default::default()
        });
        let (delta, _) = stream_update(changed.clone(), &cursor);
        changed.run_id = "next-run".into();
        let (next_run, _) = stream_update(changed, &cursor);
        // Assert
        assert_eq!(delta.steps.len(), 1);
        assert_eq!(delta.steps[0].invocation, id + 1);
        assert_eq!(delta.metrics.len(), 1);
        assert_eq!(delta.cpu_snapshots.len(), 1);
        assert_eq!(next_run.steps.len(), 2);
    }
}
