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
/// Split an observation result at the protobuf boundary; pending capture has neither field.
fn capture_parts(
    capture: Option<std::result::Result<system_config::MachineState, String>>,
) -> (Option<system_config::MachineState>, String) {
    match capture {
        Some(Ok(state)) => (Some(state), String::new()),
        Some(Err(error)) => (None, error),
        None => (None, String::new()),
    }
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
    parse_text("doramcap.console.v1.ConsoleConfig", &text)
}

/// Decode strict protobuf text into AppManifest protobuf bytes.
#[napi]
pub fn parse_manifest(text: String) -> Result<Buffer> {
    use prost::Message;
    let bytes = parse_text("doramcap.launch.v1.AppManifest", &text)?;
    let manifest =
        workload_lifecycle::launch::AppManifest::decode(bytes.as_ref()).map_err(failure)?;
    if let Some(process) = &manifest.process {
        process.validate().map_err(failure)?;
    }
    Ok(bytes)
}

fn parse_text(message: &str, text: &str) -> Result<Buffer> {
    use prost_reflect::DynamicMessage;
    let descriptor = descriptors()
        .get_message_by_name(message)
        .expect("compiled configuration");
    Ok(DynamicMessage::parse_text_format(descriptor, text)
        .map_err(failure)?
        .encode_to_vec()
        .into())
}

/// Expand native Dora modules; the server supplies an approved graph path.
#[napi]
pub async fn expand_graph(path: String) -> Result<Buffer> {
    compose_apps::preparation::expand_graph(Path::new(&path))
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

/// Dedicated owner of non-Send Compose and systemd workflows.
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
    pub async fn compose_profiles(&self, input: Buffer) -> Result<Buffer> {
        let request = protocol::ComposeProfilesRequest::decode(input.as_ref()).map_err(failure)?;
        let reply = self.dispatch(move |state| {
            let compose = state.compose.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let result = async {
                    let project = request.project.ok_or("project missing".to_string())?;
                    let resources = compose.get().await?;
                    let names = compose_apps::preparation::profiles(
                        &resources.adapter,
                        &request.run_id,
                        Path::new(&request.repository_root),
                        &project,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(protocol::ComposeProfiles { names }.encode_to_vec())
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
            let compose = state.compose.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::task::spawn_local(async move {
                let result = async {
                    let spec = match execution::launch(request)? {
                        execution::Launch::Compose(spec) => spec,
                        execution::Launch::Systemd(run, plan) => {
                            let snapshot = workload_systemd::inspect(&run, &plan).await?;
                            return tokio::task::spawn_blocking(move || {
                                let data = std::path::Path::new(&run.artifacts)
                                    .parent()
                                    .map(std::path::Path::to_path_buf)
                                    .unwrap_or_default();
                                let (machine, machine_error) =
                                    capture_parts(Some(system_config::observe(&data)));
                                Ok(protocol::Inspection {
                                    plan: None,
                                    filesystem_notes: Vec::new(),
                                    backend: Some(protocol::inspection::Backend::SystemdSnapshot(
                                        snapshot,
                                    )),
                                    machine,
                                    machine_error,
                                }
                                .encode_to_vec())
                            })
                            .await
                            .map_err(|error| error.to_string())?;
                        }
                    };
                    let resources = compose.get().await?;
                    let resolved = compose_apps::preparation::resolve(
                        &resources.adapter,
                        spec.try_into()
                            .map_err(|e: compose_apps::compose::Error| e.to_string())?,
                    )
                    .await
                    .map_err(|e| {
                        if matches!(e, compose_apps::compose::Error::EmptySelection) {
                            "EmptySelection".to_owned()
                        } else {
                            e.to_string()
                        }
                    })?;
                    let machine =
                        workload_docker::workflows::recorded_compose::observation::machine_state(
                            &resources.adapter,
                            &resolved.artifacts,
                        )
                        .await;
                    let (machine, machine_error) = capture_parts(Some(machine));
                    Ok(protocol::Inspection {
                        filesystem_notes: resolved.plan.diagnostics.clone(),
                        plan: Some(resolved.plan),
                        backend: Some(protocol::inspection::Backend::ComposeSnapshot(
                            resolved.snapshot,
                        )),
                        machine,
                        machine_error,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systemd_inspection_opens_without_a_docker_daemon() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let request = protocol::RunRequest {
                run: Some(workload_lifecycle::launch::RunPlan {
                    host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
                    id: "systemd-only".into(),
                    repository_root: "/tmp".into(),
                    artifacts: "/tmp/systemd-only/run".into(),
                    readiness_seconds: 1,
                    stop_seconds: 1,
                    ..Default::default()
                }),
                backend: Some(protocol::run_request::Backend::Systemd(
                    workload_systemd::SystemdPlan {
                        programs: vec![workload_systemd::Program {
                            id: "workload".into(),
                            argv: vec!["/bin/true".into()],
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                )),
            };

            // Act
            let execution = open("/nonexistent/console-docker.sock".into(), 1)
                .await
                .unwrap();
            let bytes = execution
                .inspect(request.encode_to_vec().into())
                .await
                .unwrap();
            let inspection = protocol::Inspection::decode(bytes.as_ref()).unwrap();
            execution.close().await.unwrap();

            // Assert
            assert!(inspection.plan.is_none());
            assert!(inspection.machine.is_some());
            assert!(matches!(
                inspection.backend,
                Some(protocol::inspection::Backend::SystemdSnapshot(snapshot)) if !snapshot.sha256.is_empty()
            ));
        });
    }
}
