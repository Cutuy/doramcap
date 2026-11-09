//! Passive host snapshots and a bounded, cancellation-owned Docker event stream.
use crate::compose::{
    DockerAdapter, Result, StoppingService, adapter::stop_timeout, command, error::invalid,
};
use bollard::{models::EventMessage, query_parameters::ListContainersOptions};
use futures_util::StreamExt;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use system_config::{DockerConfiguration, MachineState};
use tokio::{io::AsyncWriteExt, sync::watch};

/// Signal number Docker reports in the `kill` event that ends a stop wait.
const SIGKILL_SIGNAL: &str = "9";
const NANOS_PER_MILLI: i64 = 1_000_000;
/// Bounds each Docker fact source in a machine capture, so a slow daemon cannot hold up a run.
const DOCKER_FACTS_DEADLINE: Duration = Duration::from_secs(5);

/// One container's progress through a stop, read from a Docker event.
#[derive(Debug, PartialEq)]
enum StopProgress {
    Signaled {
        container: String,
        service: String,
        signaled_unix_ms: u64,
    },
    Exited {
        container: String,
    },
}

/// Classify a stop-related event; other events, and the final SIGKILL, return `None`.
fn stop_progress(event: &EventMessage) -> Option<StopProgress> {
    let actor = event.actor.as_ref()?;
    let container = actor.id.clone()?;
    let attributes = actor.attributes.as_ref()?;
    match event.action.as_deref()? {
        "kill" if attributes.get("signal")? != SIGKILL_SIGNAL => Some(StopProgress::Signaled {
            container,
            service: attributes.get("com.docker.compose.service")?.clone(),
            signaled_unix_ms: u64::try_from(event.time_nano? / NANOS_PER_MILLI).ok()?,
        }),
        "die" | "destroy" => Some(StopProgress::Exited { container }),
        _ => None,
    }
}

/// Keep `waiting` current and publish its changes; a container Docker cannot inspect is left out.
async fn track_stop(
    docker: &bollard::Docker,
    waiting: &mut BTreeMap<String, StoppingService>,
    stopping: &watch::Sender<Vec<StoppingService>>,
    event: &EventMessage,
) {
    let changed = match stop_progress(event) {
        Some(StopProgress::Signaled {
            container,
            service,
            signaled_unix_ms,
        }) if !waiting.contains_key(&container) => {
            match docker.inspect_container(&container, None).await {
                Ok(info) => {
                    let force_stop_unix_ms =
                        signaled_unix_ms + stop_timeout(&info).as_millis() as u64;
                    waiting.insert(
                        container,
                        StoppingService {
                            service,
                            force_stop_unix_ms,
                        },
                    );
                    true
                }
                Err(_) => false,
            }
        }
        Some(StopProgress::Exited { container }) => waiting.remove(&container).is_some(),
        _ => false,
    };
    if changed {
        stopping.send_replace(waiting.values().cloned().collect());
    }
}

/// Only Docker's defined exec actions skip readiness inspection; every event remains in the audit stream.
fn notify_state_change(changed: &watch::Sender<()>, event: &EventMessage) {
    if !event.action.as_deref().is_some_and(|action| {
        matches!(
            action.split_once(':').map_or(action, |(name, _)| name),
            "exec_create" | "exec_start" | "exec_die" | "exec_detach"
        )
    }) {
        changed.send_replace(());
    }
}

/// Own the background stream until teardown; dropping the owner aborts capture.
pub(super) struct EventCapture {
    finish: watch::Sender<bool>,
    fault: watch::Receiver<Option<String>>,
    changes: watch::Receiver<()>,
    task: tokio::task::JoinHandle<Result<()>>,
}
impl EventCapture {
    pub fn start(
        docker: bollard::Docker,
        project: String,
        path: std::path::PathBuf,
        stopping: watch::Sender<Vec<StoppingService>>,
    ) -> Self {
        let (finish, finished) = watch::channel(false);
        let (fault_tx, fault) = watch::channel(None);
        let (changed, changes) = watch::channel(());
        let since = SystemTime::now();
        let task = tokio::spawn(async move {
            let result =
                record_events(docker, project, &path, finished, changed, stopping, since).await;
            if let Err(error) = &result {
                let _ = fault_tx.send(Some(error.to_string()));
            }
            result
        });
        Self {
            finish,
            fault,
            changes,
            task,
        }
    }
    /// Subscribe before inspecting container state; event notifications coalesce.
    pub fn changes(&self) -> watch::Receiver<()> {
        self.changes.clone()
    }
    /// Cancelling this wait leaves event capture running and preserves pending failures.
    pub fn failure(&self) -> impl std::future::Future<Output = String> + use<> {
        let mut fault = self.fault.clone();
        async move {
            let _ = fault.changed().await;
            let error = fault.borrow().clone();
            error.unwrap_or_else(|| "Docker event stream ended".into())
        }
    }
    pub async fn finish(mut self) -> Result<()> {
        let _ = self.finish.send(true);
        (&mut self.task)
            .await
            .map_err(|e| invalid(format!("event capture: {e}")))?
    }
}
impl Drop for EventCapture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Read the Docker facts, each within its own deadline, so one slow source only leaves its own facts unset.
async fn docker_facts(adapter: &DockerAdapter, data_directory: &Path, state: &mut MachineState) {
    let docker = &adapter.docker;
    let (version, info, containers, compose_version) = tokio::join!(
        tokio::time::timeout(DOCKER_FACTS_DEADLINE, docker.version()),
        tokio::time::timeout(DOCKER_FACTS_DEADLINE, docker.info()),
        tokio::time::timeout(
            DOCKER_FACTS_DEADLINE,
            docker.list_containers(None::<ListContainersOptions>)
        ),
        compose_version(adapter, data_directory),
    );
    let info = info.ok().and_then(std::result::Result::ok);
    if let Some(configuration) = &mut state.configuration {
        configuration.docker = Some(DockerConfiguration {
            server_version: version
                .ok()
                .and_then(std::result::Result::ok)
                .and_then(|version| version.version)
                .unwrap_or_default(),
            compose_version,
            cgroup_driver: info
                .as_ref()
                .and_then(|i| i.cgroup_driver)
                .map(|d| d.to_string())
                .unwrap_or_default(),
            cgroup_version: info
                .as_ref()
                .and_then(|i| i.cgroup_version)
                .map(|v| v.to_string())
                .unwrap_or_default(),
            storage_driver: info.and_then(|i| i.driver).unwrap_or_default(),
        });
    }
    if let (Some(condition), Ok(Ok(containers))) = (&mut state.condition, containers) {
        let mut names: Vec<String> = containers
            .into_iter()
            .filter_map(|container| container.names?.into_iter().next())
            .map(|name| name.trim_start_matches('/').to_owned())
            .collect();
        names.sort();
        condition.other_containers = names;
    }
}
/// The installed Compose client version, through the helper that owns its plugin's process group.
/// Empty when it cannot be run within the deadline.
async fn compose_version(adapter: &DockerAdapter, directory: &Path) -> String {
    let socket = adapter.socket_path().to_string_lossy();
    let arguments = ["version".to_owned(), "--short".to_owned()];
    command::compose(
        &socket,
        directory,
        &BTreeMap::new(),
        &arguments,
        DOCKER_FACTS_DEADLINE,
    )
    .await
    .map_or_else(|_| String::new(), |(version, _)| version.trim().to_owned())
}
/// Observe this machine with its Docker facts. `data_directory` selects the disk reported as free.
/// Unreadable facts stay unset, so this never fails a run.
pub async fn machine_state(adapter: &DockerAdapter, data_directory: &Path) -> MachineState {
    let mut state = system_config::observe(data_directory);
    docker_facts(adapter, data_directory, &mut state).await;
    state
}
async fn record_events(
    docker: bollard::Docker,
    project: String,
    path: &Path,
    mut finish: watch::Receiver<bool>,
    changed: watch::Sender<()>,
    stopping: watch::Sender<Vec<StoppingService>>,
    since: SystemTime,
) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    let since = since
        .duration_since(UNIX_EPOCH)
        .map_err(|e| invalid(e.to_string()))?
        .as_secs()
        .to_string();
    let mut waiting = BTreeMap::new();
    let stream = docker.events(Some(bollard::query_parameters::EventsOptions {
        since: Some(since),
        // Include foreign ownership within the project so inspection can reject it.
        filters: Some(HashMap::from([(
            "label".into(),
            vec![format!("com.docker.compose.project={project}")],
        )])),
        ..Default::default()
    }));
    tokio::pin!(stream);
    let mut draining = false;
    let quiet = tokio::time::sleep(Duration::from_millis(200));
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(quiet, deadline);
    loop {
        tokio::select! {
            changed = finish.changed(), if !draining => {
                if changed.is_err() || *finish.borrow() {
                    draining = true;
                    quiet.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(200));
                    deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(5));
                }
            }
            event = stream.next() => {
                let event = event.ok_or_else(|| invalid("Docker events stream ended unexpectedly"))?.map_err(|e| crate::compose::error::engine("Docker events", e))?;
                notify_state_change(&changed, &event);
                track_stop(&docker, &mut waiting, &stopping, &event).await;
                let mut bytes = serde_json::to_vec(&event)?; bytes.push(b'\n');
                file.write_all(&bytes).await?;
                if draining { quiet.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(200)); }
            }
            _ = &mut quiet, if draining => { file.flush().await?; return Ok(()); }
            _ = &mut deadline, if draining => { return Err(invalid("Docker event stream did not become quiet after teardown")); }
        }
    }
}

#[cfg(test)]
mod tests;
