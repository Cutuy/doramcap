//! Retained native event/output tasks; none controls Docker ownership.
use bollard::query_parameters::EventsOptions;
use docker_engine::{CONTROLLER_LABEL, Engine, RUN_LABEL};
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncRead, AsyncWriteExt},
    process::Child,
    sync::watch,
    task::JoinHandle,
};

pub(super) const STDOUT: &str = "holoscan-stdout.log";
pub(super) const STDERR: &str = "holoscan-stderr.log";
pub(super) const EVENTS: &str = "holoscan-events.jsonl";
pub(super) const RUNTIME: &str = "holoscan-runtime.log";

pub(super) struct Events {
    finish: watch::Sender<bool>,
    changed: watch::Receiver<Result<(), String>>,
    task: Option<JoinHandle<Result<(), String>>>,
}
impl Events {
    /// Capture from a prelaunch timestamp so subscription scheduling cannot omit creation events.
    /// Native event history and this explicit finish bound the available evidence.
    pub fn start(
        engine: Engine,
        controller: String,
        run: String,
        path: PathBuf,
    ) -> Result<Self, String> {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_secs()
            .to_string();
        let (finish, mut finished) = watch::channel(false);
        let (changed_tx, changed) = watch::channel(Ok(()));
        let task = tokio::spawn(async move {
            let operation = async {
                let mut file = private_file(&path).await?;
                let events = engine.client().events(Some(EventsOptions {
                    since: Some(since),
                    filters: Some(HashMap::from([
                        ("type".into(), vec!["container".into()]),
                        ("label".into(), vec![format!("{CONTROLLER_LABEL}={controller}"), format!("{RUN_LABEL}={run}")]),
                    ])),
                    ..Default::default()
                }));
                tokio::pin!(events);
                loop {
                    tokio::select! {
                        notification = finished.changed() => {
                            if notification.is_err() || *finished.borrow() { break; }
                        }
                        event = events.next() => {
                            let event = event.ok_or("native Docker event stream ended")?.map_err(error)?;
                            let mut bytes = serde_json::to_vec(&event).map_err(error)?;
                            bytes.push(b'\n');
                            file.write_all(&bytes).await.map_err(error)?;
                            let _ = changed_tx.send_replace(Ok(()));
                        }
                    }
                }
                file.sync_all().await.map_err(error)
            }.await;
            if let Err(error) = &operation {
                let _ = changed_tx.send_replace(Err(error.clone()));
            }
            operation
        });
        Ok(Self {
            finish,
            changed,
            task: Some(task),
        })
    }
    /// Subscribe before inspecting current Engine state, then recheck after each notification.
    pub fn changes(&self) -> watch::Receiver<Result<(), String>> {
        self.changed.clone()
    }
    pub async fn finish(mut self) -> Result<(), String> {
        self.finish.send_replace(true);
        self.task
            .take()
            .expect("owned event task")
            .await
            .map_err(error)?
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub(super) struct CliStreams {
    failure: watch::Receiver<Option<String>>,
    // Keep normal EOF from disconnecting a live failure waiter.
    _fault: watch::Sender<Option<String>>,
    tasks: Vec<JoinHandle<Result<u64, String>>>,
}
impl CliStreams {
    /// Move piped output into file-backed tasks while the caller retains the actual Child.
    pub fn start(child: &mut Child, artifacts: &Path) -> Result<Self, String> {
        let stdout = child
            .stdout
            .take()
            .ok_or("Holoscan CLI stdout was not piped")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Holoscan CLI stderr was not piped")?;
        let (fault, failure) = watch::channel(None);
        let tasks = vec![
            copy(stdout, artifacts.join(STDOUT), fault.clone()),
            copy(stderr, artifacts.join(STDERR), fault.clone()),
        ];
        Ok(Self {
            failure,
            _fault: fault,
            tasks,
        })
    }
    pub fn failure(&self) -> impl Future<Output = String> + use<> {
        let mut failure = self.failure.clone();
        async move {
            loop {
                if let Some(error) = failure.borrow_and_update().clone() {
                    return error;
                }
                if failure.changed().await.is_err() {
                    return "native CLI output observation disconnected".into();
                }
            }
        }
    }
    /// Call after all helper writers stop; abort instead if their quiescence remains unknown.
    pub async fn finish(mut self, writers_stopped: bool) -> Result<(), String> {
        if !writers_stopped {
            for task in &self.tasks {
                task.abort();
            }
        }
        let mut failures = Vec::new();
        for task in self.tasks.drain(..) {
            match task.await {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => failures.push(error),
                Err(error) => failures.push(error.to_string()),
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}
impl Drop for CliStreams {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
fn copy(
    mut reader: impl AsyncRead + Unpin + Send + 'static,
    path: PathBuf,
    fault: watch::Sender<Option<String>>,
) -> JoinHandle<Result<u64, String>> {
    tokio::spawn(async move {
        let operation = async {
            let mut file = private_file(&path).await?;
            let bytes = tokio::io::copy(&mut reader, &mut file)
                .await
                .map_err(error)?;
            file.sync_all().await.map_err(error)?;
            Ok::<_, String>(bytes)
        }
        .await;
        if let Err(error) = &operation {
            fault.send_modify(|first| {
                if first.is_none() {
                    *first = Some(error.clone());
                }
            });
        }
        operation
    })
}
/// Read available stopped-container logs through its native driver; failures remain independent facts.
pub(super) async fn runtime(
    engine: &Engine,
    container: &str,
    artifacts: &Path,
    deadline: tokio::time::Instant,
) -> Result<(), String> {
    tokio::time::timeout_at(deadline, async {
        let mut file = private_file(&artifacts.join(RUNTIME)).await?;
        let logs = engine.client().logs(
            container,
            Some(bollard::query_parameters::LogsOptions {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );
        tokio::pin!(logs);
        while let Some(record) = logs.next().await {
            file.write_all(record.map_err(error)?.as_ref())
                .await
                .map_err(error)?;
        }
        file.sync_all().await.map_err(error)
    })
    .await
    .map_err(|_| "available native runtime log deadline expired".to_owned())?
}

async fn private_file(path: &Path) -> Result<tokio::fs::File, String> {
    tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await
        .map_err(error)
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        process::Stdio,
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, BufReader},
        net::UnixListener,
        process::Command,
    };
    const DEADLINE: Duration = Duration::from_secs(2);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "holoscan-observation-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[tokio::test]
    async fn cli_streams_keep_raw_stdout_and_stderr_as_independent_synced_files() {
        // Arrange
        let fixture = Fixture::new();
        let mut child = Command::new("/bin/sh")
            .args(["-c", "printf stdout; printf stderr >&2"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Act
        let streams = CliStreams::start(&mut child, &fixture.0).unwrap();
        assert!(child.wait().await.unwrap().success());
        streams.finish(true).await.unwrap();
        // Assert
        assert_eq!(fs::read(fixture.0.join(STDOUT)).unwrap(), b"stdout");
        assert_eq!(fs::read(fixture.0.join(STDERR)).unwrap(), b"stderr");
    }

    #[tokio::test]
    async fn output_failure_is_signaled_and_the_other_stream_remains_available() {
        // Arrange
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join(STDOUT)).unwrap();
        let mut child = Command::new("/bin/sh")
            .args(["-c", "printf retained >&2; printf stdout"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let streams = CliStreams::start(&mut child, &fixture.0).unwrap();
        // Act
        let error = tokio::time::timeout(DEADLINE, streams.failure())
            .await
            .unwrap();
        let _ = child.wait().await.unwrap();
        let completion = streams.finish(true).await;
        // Assert
        assert!(!error.is_empty());
        assert!(completion.is_err());
        assert!(
            fs::read(fixture.0.join(STDERR))
                .unwrap()
                .starts_with(b"retained")
        );
    }

    #[tokio::test]
    async fn native_event_arrival_wakes_observation_and_finish_syncs_available_evidence() {
        // Arrange
        let fixture = Fixture::new();
        let socket = fixture.0.join("docker.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut connection = BufReader::new(connection);
            let mut request = String::new();
            loop {
                let mut line = String::new();
                connection.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            assert!(request.starts_with("GET /events?"));
            assert!(request.contains("since="));
            assert!(request.contains("io.doramcap.controller"));
            assert!(request.contains("io.doramcap.run"));
            let body = b"{\"Type\":\"container\",\"Action\":\"die\",\"Actor\":{\"ID\":\"owned\",\"Attributes\":{}},\"time\":1}\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n",
                body.len()
            );
            connection
                .get_mut()
                .write_all(response.as_bytes())
                .await
                .unwrap();
            connection.get_mut().write_all(body).await.unwrap();
            connection.get_mut().write_all(b"\r\n").await.unwrap();
            let mut end = Vec::new();
            connection.read_to_end(&mut end).await.unwrap();
        });
        let engine = Engine::new(socket.to_str().unwrap(), 2).unwrap();
        let capture = Events::start(
            engine,
            "controller".into(),
            "run".into(),
            fixture.0.join(EVENTS),
        )
        .unwrap();
        let mut changed = capture.changes();
        // Act
        tokio::time::timeout(DEADLINE, changed.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(changed.borrow_and_update().is_ok());
        capture.finish().await.unwrap();
        tokio::time::timeout(DEADLINE, server)
            .await
            .unwrap()
            .unwrap();
        // Assert
        let line = fs::read_to_string(fixture.0.join(EVENTS)).unwrap();
        let evidence: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(evidence["Action"], "die");
    }
}
