//! Opt-in real Docker validation. See scripts/docker-monitor-e2e.py.
//! The procedural workflow always attempts cleanup after ordinary errors.
use orchestrator::{Error, Event, LocalExecutor, Result, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::RefCell, path::Path, process::Stdio, rc::Rc, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

#[derive(Serialize, Deserialize)]
struct Config {
    session: String,
    image: String,
    directory: String,
    inject_failure: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Container {
    id: String,
}
fn fail(message: impl std::fmt::Display) -> Error {
    Error::message(message.to_string())
}
fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(fail(message))
    }
}
async fn docker(args: &[&str]) -> Result<String> {
    let mut child = Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(fail)?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    async fn read(stream: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        stream
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(fail)?;
        ensure(
            bytes.len() <= 2 * 1024 * 1024,
            "Docker output exceeded 2 MiB",
        )?;
        Ok(bytes)
    }
    let operation = async {
        tokio::try_join!(read(stdout), read(stderr), async {
            child.wait().await.map_err(fail)
        })
    };
    let result = tokio::time::timeout(Duration::from_secs(30), operation).await;
    match result {
        Ok(Ok((mut out, err, status))) => {
            ensure(
                status.success(),
                &format!("docker {}: {}", args[0], String::from_utf8_lossy(&err)),
            )?;
            if args[0] == "logs" {
                out.extend_from_slice(&err);
            }
            String::from_utf8(out).map_err(fail)
        }
        error => {
            child.kill().await.map_err(fail)?;
            Err(fail(format!(
                "Docker command timeout or I/O failure: {error:?}"
            )))
        }
    }
}
async fn save(config: &Config, name: &str, bytes: impl AsRef<[u8]>) -> Result<()> {
    tokio::fs::write(Path::new(&config.directory).join(name), bytes)
        .await
        .map_err(fail)
}
async fn inspect(config: &Config, container: &Container) -> Result<Value> {
    let value: Value =
        serde_json::from_str(&docker(&["inspect", &container.id]).await?).map_err(fail)?;
    let value = value[0].clone();
    ensure(
        value["Config"]["Labels"]["io.doramcap.e2e.session"] == config.session.as_str(),
        "container ownership mismatch",
    )?;
    Ok(value)
}
async fn owned_ids(config: &Config) -> Result<Vec<String>> {
    let label = format!("label=io.doramcap.e2e.session={}", config.session.as_str());
    Ok(docker(&["ps", "-aq", "--no-trunc", "--filter", &label])
        .await?
        .lines()
        .map(str::to_owned)
        .collect())
}

struct Create {
    config: Rc<Config>,
}
impl Step for Create {
    type Input = ();
    type Output = Container;
    async fn run(&mut self, _: ()) -> Result<Container> {
        let config = &self.config;
        let label = format!("io.doramcap.e2e.session={}", config.session.as_str());
        let mount = format!("type=bind,src={},dst=/recordings", config.directory);
        let id = docker(&[
            "create",
            "--name",
            config.session.as_str(),
            "--label",
            &label,
            "--network",
            "none",
            "--read-only",
            // The recorder uses io_uring, blocked by Docker's default seccomp.
            // This trusted local test uses a scoped container exception.
            "--security-opt",
            "seccomp=unconfined",
            "--cap-drop",
            "ALL",
            "--cap-add",
            "BPF",
            "--cap-add",
            "PERFMON",
            "--cap-add",
            "SYSLOG",
            "--ulimit",
            "memlock=268435456:268435456",
            "--cgroupns",
            "host",
            "--mount",
            "type=bind,src=/sys/fs/cgroup,dst=/sys/fs/cgroup,readonly",
            "--mount",
            "type=bind,src=/sys/kernel/btf,dst=/sys/kernel/btf,readonly",
            "--mount",
            &mount,
            &config.image,
            "--backend",
            "bpf",
            "--records",
            "32768",
            "--output",
            "/recordings/monitor.pb",
            "--record",
            "/recordings/capture.mcap",
            "--record-buffered",
        ])
        .await?;
        Ok(Container {
            id: id.trim().into(),
        })
    }
}
struct Start {
    config: Rc<Config>,
}
impl Step for Start {
    type Input = Container;
    type Output = Container;
    async fn run(&mut self, container: Container) -> Result<Container> {
        let config = &self.config;
        inspect(config, &container).await?;
        docker(&["start", &container.id]).await?;
        Ok(container)
    }
}
struct AuditRunning {
    config: Rc<Config>,
}
impl Step for AuditRunning {
    type Input = Container;
    type Output = Container;
    async fn run(&mut self, container: Container) -> Result<Container> {
        let config = &self.config;
        // Readiness requires live output, rather than only Docker's Running bit.
        let output = Path::new(&config.directory).join("monitor.pb");
        let mut first_size = None;
        for _ in 0..40 {
            let state = inspect(config, &container).await?;
            ensure(
                state["State"]["Running"] == true,
                "monitor exited before readiness",
            )?;
            if let Ok(metadata) = tokio::fs::metadata(&output).await
                && metadata.len() > 0
            {
                first_size = Some(metadata.len());
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let first_size = first_size
            .ok_or_else(|| fail("monitor produced no telemetry before readiness timeout"))?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let state = inspect(config, &container).await?;
        ensure(
            state["State"]["Running"] == true,
            "monitor stopped during observation",
        )?;
        let pid = state["State"]["Pid"].as_u64().unwrap_or(0);
        ensure(pid > 0, "missing monitor PID")?;
        ensure(
            state["Path"] == "/system-monitor",
            "unexpected container executable",
        )?;
        ensure(
            tokio::fs::metadata(output).await.map_err(fail)?.len() > first_size,
            "telemetry stopped advancing",
        )?;
        let top = docker(&["top", &container.id, "-eo", "pid,comm"]).await?;
        ensure(
            top.lines().any(|line| {
                line.split_whitespace().next() == Some(&pid.to_string())
                    && line.contains("system-monitor")
            }),
            "monitor PID absent from Docker process listing",
        )?;
        save(
            config,
            "running.json",
            serde_json::to_vec_pretty(&state).map_err(fail)?,
        )
        .await?;
        save(config, "processes.txt", top).await?;
        if config.inject_failure {
            return Err(fail("injected audit failure after readiness"));
        }
        Ok(container)
    }
}
struct Stop {
    config: Rc<Config>,
}
impl Step for Stop {
    type Input = Container;
    type Output = Container;
    async fn run(&mut self, container: Container) -> Result<Container> {
        let config = &self.config;
        inspect(config, &container).await?;
        docker(&["stop", "--time", "10", &container.id]).await?;
        Ok(container)
    }
}
struct AuditStopped {
    config: Rc<Config>,
}
impl Step for AuditStopped {
    type Input = Container;
    type Output = Container;
    async fn run(&mut self, container: Container) -> Result<Container> {
        let config = &self.config;
        let state = inspect(config, &container).await?;
        save(
            config,
            "stopped.json",
            serde_json::to_vec_pretty(&state).map_err(fail)?,
        )
        .await?;
        let logs = docker(&["logs", &container.id]).await?;
        save(config, "monitor.log", logs).await?;
        ensure(
            state["State"]["Status"] == "exited" && state["State"]["Pid"] == 0,
            "container still has a live process",
        )?;
        ensure(
            state["State"]["ExitCode"] == 0 && state["State"]["OOMKilled"] == false,
            "monitor failed or required forced termination",
        )?;
        Ok(container)
    }
}
struct AuditRecording {
    config: Rc<Config>,
}
impl Step for AuditRecording {
    type Input = Container;
    type Output = Value;
    async fn run(&mut self, container: Container) -> Result<Value> {
        let config = &self.config;
        let path = Path::new(&config.directory).join("capture.mcap");
        let size = tokio::fs::metadata(&path).await.map_err(fail)?.len();
        ensure(
            size < 64 * 1024 * 1024,
            "fixture recording exceeds 64 MiB audit bound",
        )?;
        let bytes = tokio::fs::read(&path).await.map_err(fail)?;
        // This writer intentionally emits a valid unindexed MCAP (no summary).
        // footer() validates the terminal footer and trailing magic.
        mcap::read::footer(&bytes).map_err(fail)?;
        let mut sequences = Vec::new();
        let mut dictionary_seen = false;
        let mut monitor_session = String::new();
        let mut first_counters = std::collections::BTreeMap::new();
        let mut counters_advanced = false;
        for message in mcap::MessageStream::new(&bytes).map_err(fail)? {
            let message = message.map_err(fail)?;
            if message.channel.topic == "doramcap/dictionary" {
                let dictionary =
                    telemetry_protocol::decode_dictionary(&message.data).map_err(fail)?;
                ensure(
                    !dictionary.entries.is_empty(),
                    "empty native cgroup dictionary",
                )?;
                dictionary_seen = true;
            } else {
                ensure(dictionary_seen, "snapshot preceded dictionary")?;
                let frame = telemetry_protocol::decode(&message.data).map_err(fail)?;
                ensure(frame.source == "bpf", "fixture did not use native BPF")?;
                ensure(
                    !frame.snapshot.records.is_empty(),
                    "native snapshot has no CPU records",
                )?;
                ensure(
                    frame.snapshot.dropped_batches == 0,
                    "monitor dropped batches",
                )?;
                if monitor_session.is_empty() {
                    monitor_session = frame.session_id.clone();
                }
                ensure(
                    monitor_session == frame.session_id,
                    "recording mixed monitor sessions",
                )?;
                for record in &frame.snapshot.records {
                    let key = (record.cgroup_id, record.cpu);
                    if sequences.is_empty() {
                        first_counters.insert(key, record.runtime_ns);
                    } else if let Some(first) = first_counters.get(&key) {
                        counters_advanced |= record.runtime_ns > *first;
                    }
                }
                sequences.push(frame.snapshot.sequence);
            }
        }
        ensure(sequences.len() >= 5, "too few native CPU snapshots")?;
        ensure(
            sequences.iter().copied().eq(0..sequences.len() as u64),
            "missing or reordered snapshots",
        )?;
        ensure(counters_advanced, "native CPU counters did not advance")?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        ensure(
            tokio::fs::metadata(path).await.map_err(fail)?.len() == size,
            "recording changed after stop",
        )?;
        let report = json!({"container": container.id, "monitor_session": monitor_session, "snapshots": sequences.len(), "bytes": size, "backend": "bpf", "finalized": true});
        save(
            config,
            "recording.json",
            serde_json::to_vec_pretty(&report).map_err(fail)?,
        )
        .await?;
        Ok(report)
    }
}
struct RemoveOwned {
    config: Rc<Config>,
}
impl Step for RemoveOwned {
    type Input = ();
    type Output = ();
    async fn run(&mut self, _: ()) -> Result<()> {
        let config = &self.config;
        for id in owned_ids(config).await? {
            inspect(config, &Container { id: id.clone() }).await?;
            docker(&["rm", "--force", &id]).await?;
        }
        Ok(())
    }
}
struct AuditAbsent {
    config: Rc<Config>,
}
impl Step for AuditAbsent {
    type Input = ();
    type Output = ();
    async fn run(&mut self, _: ()) -> Result<()> {
        let config = &self.config;
        ensure(
            owned_ids(config).await?.is_empty(),
            "owned container survived cleanup",
        )
    }
}

pub async fn experiment(
    image: &str,
    root: &Path,
    session: &str,
    inject_failure: bool,
    observer: impl Fn(&Event) + 'static,
) -> Result<Value> {
    let directory = root.join(session);
    tokio::fs::create_dir(&directory).await.map_err(fail)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let config = Rc::new(Config {
        session: session.into(),
        image: image.into(),
        directory: directory.to_string_lossy().into(),
        inject_failure,
    });
    let mut executor = LocalExecutor::new(move |event: &Event| {
        observer(event);
        sink.borrow_mut().push(event.clone());
    })
    .with_label(session)
    .capture_payloads(true);
    let settings = config.clone();
    executor.register::<Create>(move || Create {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<Start>(move || Start {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<AuditRunning>(move || AuditRunning {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<Stop>(move || Stop {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<AuditStopped>(move || AuditStopped {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<AuditRecording>(move || AuditRecording {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<RemoveOwned>(move || RemoveOwned {
        config: settings.clone(),
    });
    let settings = config.clone();
    executor.register::<AuditAbsent>(move || AuditAbsent {
        config: settings.clone(),
    });
    let mut recording = None;
    let mut cleanup_errors = Vec::new();
    let created = executor.run::<Create>(()).await;
    let outcome = match created {
        Ok(container) => {
            let result = async {
                let started = executor.run::<Start>(container.clone()).await?;
                executor.run::<AuditRunning>(started).await?;
                Ok::<_, Error>(())
            }
            .await;
            // Cleanup is ordinary awaited code, reached even after readiness failure.
            for _ in 0..2 {
                // Exercise idempotent stop on the same owned resource.
                if let Err(error) = executor.run::<Stop>(container.clone()).await {
                    cleanup_errors.push(error.to_string());
                }
            }
            match executor.run::<AuditStopped>(container).await {
                Ok(stopped) => match executor.run::<AuditRecording>(stopped).await {
                    Ok(report) => recording = Some(report.into_value()),
                    Err(error) => cleanup_errors.push(error.to_string()),
                },
                Err(error) => cleanup_errors.push(error.to_string()),
            }
            result
        }
        Err(error) => Err(error),
    };
    // Independent cleanup continues even if stop or readback fails.
    for _ in 0..2 {
        if let Err(error) = executor.run::<RemoveOwned>(()).await {
            cleanup_errors.push(error.to_string());
        }
    }
    if let Err(error) = executor.run::<AuditAbsent>(()).await {
        cleanup_errors.push(error.to_string());
    }
    let outcome_error = outcome.err().map(|e| e.to_string());
    let report = json!({"session": session, "injected_failure": inject_failure, "execution_error": outcome_error, "cleanup_errors": cleanup_errors, "recording": recording});
    save(
        &config,
        "report.json",
        serde_json::to_vec_pretty(&report).map_err(fail)?,
    )
    .await?;
    let event_bytes = events
        .borrow()
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(fail)?
        .join("\n")
        + "\n";
    save(&config, "events.jsonl", event_bytes).await?;
    ensure(executor.observer_failures() == 0, "lost step events")?;
    ensure(
        cleanup_errors.is_empty(),
        &format!("cleanup/audit failures: {cleanup_errors:?}"),
    )?;
    if inject_failure {
        ensure(
            outcome_error.as_deref() == Some("injected audit failure after readiness"),
            "expected injected failure was not observed",
        )?;
    } else {
        ensure(
            outcome_error.is_none(),
            &format!("experiment failed: {outcome_error:?}"),
        )?;
    }
    Ok(report)
}
