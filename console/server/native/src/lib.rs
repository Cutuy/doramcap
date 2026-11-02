//! Protobuf execution transport and OS capabilities for the TypeScript server.
mod execution;
mod runtime;

use execution::{State, start};
use napi::{
    Env, Error, Result, Status,
    bindgen_prelude::{AsyncBlock, AsyncBlockBuilder, Buffer},
};
use napi_derive::napi;
use prost::Message;
use std::{
    fs::File,
    os::unix::{fs::OpenOptionsExt, io::IntoRawFd},
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

pub mod protocol {
    include!(concat!(
        env!("OUT_DIR"),
        "/doramcap.console.execution.v1.rs"
    ));
}
fn failure(error: impl std::fmt::Display) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}
fn dispatch_failure(error: runtime::DispatchError) -> Error {
    failure(match error {
        runtime::DispatchError::Busy => "Busy",
        runtime::DispatchError::Closed => "Closing",
    })
}

fn descriptors() -> &'static prost_reflect::DescriptorPool {
    static POOL: OnceLock<prost_reflect::DescriptorPool> = OnceLock::new();
    POOL.get_or_init(|| {
        prost_reflect::DescriptorPool::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/console.bin")).as_slice(),
        )
        .expect("compiled descriptors")
    })
}

/// Encode ConsoleConfig protobuf bytes as canonical protobuf text.
#[napi]
pub fn format_config(bytes: Buffer) -> Result<String> {
    let descriptor = descriptors()
        .get_message_by_name("doramcap.console.v1.ConsoleConfig")
        .expect("compiled configuration");
    Ok(
        prost_reflect::DynamicMessage::decode(descriptor, bytes.as_ref())
            .map_err(failure)?
            .to_text_format(),
    )
}

/// Decode protobuf text with the existing parser; configuration policy belongs to TypeScript.
#[napi]
pub fn parse_config(text: String) -> Result<Buffer> {
    use prost_reflect::DynamicMessage;
    let pool = descriptors();
    let descriptor = pool
        .get_message_by_name("doramcap.console.v1.ConsoleConfig")
        .expect("compiled configuration");
    Ok(DynamicMessage::parse_text_format(descriptor, &text)
        .map_err(failure)?
        .encode_to_vec()
        .into())
}

/// Expand native Dora modules; the server supplies an approved graph path.
#[napi]
pub async fn expand_graph(path: String) -> Result<Buffer> {
    workload_docker::workflows::recorded_compose::preparation::expand_graph(Path::new(&path))
        .await
        .map(Buffer::from)
        .map_err(failure)
}

/// OS lock held until explicitly closed or its handle is released.
#[napi]
pub struct FileLock {
    file: Option<File>,
}
#[napi]
impl FileLock {
    #[napi]
    pub fn close(&mut self) {
        self.file = None;
    }
}
#[napi]
pub fn lock_file(path: String) -> Result<FileLock> {
    use rustix::fs::{FlockOperation, flock};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(Path::new(&path))
        .map_err(failure)?;
    if !file.metadata().map_err(failure)?.is_file() {
        return Err(failure("lock is not a regular file"));
    }
    flock(&file, FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| failure("lock is already in use"))?;
    Ok(FileLock { file: Some(file) })
}

/// Open a server-authorized regular file beneath a root; the caller owns the descriptor.
#[napi]
pub fn open_file(root: String, relative: String) -> Result<i32> {
    let root = workload_docker::evidence::root(Path::new(&root)).map_err(failure)?;
    workload_docker::evidence::open(&root, &relative, false)
        .map(IntoRawFd::into_raw_fd)
        .map_err(failure)
}

/// Dedicated owner of the existing non-Send Compose workflow.
#[napi]
pub struct Execution {
    owner: Mutex<Option<runtime::Owner<State>>>,
    updates: Arc<Mutex<protocol::ExecutionUpdate>>,
    close_lock: tokio::sync::Mutex<()>,
}
#[napi]
pub async fn open(socket: String, version: u32) -> Result<Execution> {
    if version != 1 {
        return Err(failure("execution contract version mismatch"));
    }
    let updates = Arc::new(Mutex::new(protocol::ExecutionUpdate::default()));
    let shared = updates.clone();
    let owner = runtime::Owner::open(move || async move { State::open(socket, shared).await })
        .await
        .map_err(failure)?;
    Ok(Execution {
        owner: Mutex::new(Some(owner)),
        updates,
        close_lock: Default::default(),
    })
}
impl Execution {
    fn dispatch(
        &self,
        operation: impl FnOnce(&mut State) -> execution::Reply + Send + 'static,
    ) -> Result<tokio::sync::oneshot::Receiver<execution::Reply>> {
        self.owner
            .lock()
            .map_err(failure)?
            .as_ref()
            .ok_or_else(|| failure("Closing"))?
            .dispatch(operation)
            .map_err(dispatch_failure)
    }
}
#[napi]
impl Execution {
    #[napi]
    pub async fn profiles(&self, input: Buffer) -> Result<Buffer> {
        let request = protocol::ProfilesRequest::decode(input.as_ref()).map_err(failure)?;
        let reply = self.dispatch(move |state| {
            let adapter = state.adapter.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let result = async {
                    let project = request.project.ok_or("project missing".to_string())?;
                    let names =
                        workload_docker::workflows::recorded_compose::preparation::profiles(
                            &adapter,
                            &request.run_id,
                            Path::new(&request.repository_root),
                            &project,
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(protocol::Profiles { names }.encode_to_vec())
                }
                .await;
                let _ = tx.send(result);
            });
            rx
        })?;
        Ok(reply
            .await
            .map_err(failure)?
            .await
            .map_err(failure)?
            .map_err(failure)?
            .into())
    }
    #[napi]
    pub async fn inspect(&self, input: Buffer) -> Result<Buffer> {
        let request = protocol::RunRequest::decode(input.as_ref()).map_err(failure)?;
        let reply = self.dispatch(move |state| {
            let adapter = state.adapter.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let result = async {
                    let resolved =
                        workload_docker::workflows::recorded_compose::preparation::resolve(
                            &adapter,
                            execution::run(request)?,
                        )
                        .await
                        .map_err(|e| {
                            if matches!(e, workload_docker::compose::Error::EmptySelection) {
                                "EmptySelection".to_owned()
                            } else {
                                e.to_string()
                            }
                        })?;
                    Ok(protocol::Inspection {
                        plan: Some(resolved.plan),
                        snapshot: Some(resolved.snapshot),
                    }
                    .encode_to_vec())
                }
                .await;
                let _ = tx.send(result);
            });
            rx
        })?;
        Ok(reply
            .await
            .map_err(failure)?
            .await
            .map_err(failure)?
            .map_err(failure)?
            .into())
    }
    /// Enqueue synchronously so a subsequent Stop cannot overtake admission.
    #[napi]
    pub fn start(&self, env: Env, input: Buffer) -> Result<AsyncBlock<Buffer>> {
        let request = protocol::StartRequest::decode(input.as_ref()).map_err(failure)?;
        let reply = self
            .owner
            .lock()
            .map_err(failure)?
            .as_ref()
            .ok_or_else(|| failure("Closing"))?
            .dispatch_control(move |state| start(state, request))
            .map_err(dispatch_failure)?;
        AsyncBlockBuilder::new(async move {
            Ok(reply
                .await
                .map_err(failure)?
                .await
                .map_err(failure)?
                .map_err(failure)?
                .into())
        })
        .build(&env)
    }
    #[napi]
    pub async fn stop(&self, run_id: String) -> Result<()> {
        let reply = self
            .owner
            .lock()
            .map_err(failure)?
            .as_ref()
            .ok_or_else(|| failure("Closing"))?
            .dispatch_control(move |state| state.stop(&run_id))
            .map_err(dispatch_failure)?;
        reply.await.map_err(failure)?.map_err(failure)
    }
    /// Drain bounded raw events; execution completion remains available on later polls.
    #[napi]
    pub fn poll(&self) -> Result<Buffer> {
        let mut updates = self.updates.lock().map_err(failure)?;
        let events = std::mem::take(&mut updates.events);
        let mut batch = updates.clone();
        batch.events = events;
        drop(updates);
        Ok(batch.encode_to_vec().into())
    }
    /// Stop the active workflow and await cleanup before joining its owner thread.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let _serial = self.close_lock.lock().await;
        let owner = self.owner.lock().map_err(failure)?.take();
        let Some(owner) = owner else {
            return Ok(());
        };
        let reply = owner
            .dispatch_shutdown(|state| {
                state.closing = true;
                state.shutdown()
            })
            .await
            .map_err(|_| failure("Closing"))?;
        if let Some(done) = reply.await.map_err(failure)? {
            let _ = done.await;
        }
        owner.close().await.map_err(failure)?;
        Ok(())
    }
}
