//! Passive host snapshots and a bounded, cancellation-owned Docker event stream.
use crate::compose::{
    Result,
    adapter::{RUN_LABEL, private_write},
    error::invalid,
};
use futures_util::StreamExt;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncWriteExt, sync::watch};

/// Own the background stream until teardown; dropping the owner aborts capture.
pub(super) struct EventCapture {
    finish: watch::Sender<bool>,
    fault: watch::Receiver<Option<String>>,
    task: tokio::task::JoinHandle<Result<()>>,
}
impl EventCapture {
    pub fn start(
        docker: bollard::Docker,
        run: String,
        project: String,
        path: std::path::PathBuf,
    ) -> Self {
        let (finish, finished) = watch::channel(false);
        let (fault_tx, fault) = watch::channel(None);
        let task = tokio::spawn(async move {
            let result = record_events(docker, run, project, &path, finished).await;
            if let Err(error) = &result {
                let _ = fault_tx.send(Some(error.to_string()));
            }
            result
        });
        Self {
            finish,
            fault,
            task,
        }
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

pub(super) async fn snapshot(root: &Path, phase: &str) -> Result<()> {
    let mut state = BTreeMap::new();
    for name in ["meminfo", "loadavg", "uptime", "sys/kernel/random/boot_id"] {
        state.insert(
            name,
            tokio::fs::read_to_string(format!("/proc/{name}")).await?,
        );
    }
    private_write(
        &root.join(format!("machine-{phase}.json")),
        &serde_json::to_vec_pretty(&state)?,
    )
    .await
}
async fn record_events(
    docker: bollard::Docker,
    run: String,
    project: String,
    path: &Path,
    mut finish: watch::Receiver<bool>,
) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| invalid(e.to_string()))?
        .as_secs()
        .to_string();
    let stream = docker.events(Some(bollard::query_parameters::EventsOptions {
        since: Some(since),
        filters: Some(HashMap::from([(
            "label".into(),
            vec![
                format!("{RUN_LABEL}={run}"),
                format!("com.docker.compose.project={project}"),
            ],
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
