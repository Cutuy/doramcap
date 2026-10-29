//! Authorized fixed-workflow worker. HTTP remains in a separate unprivileged process.
use orchestrator::{Observer, telemetry::TelemetryObserver};
use serde_json::json;
use std::{collections::BTreeMap, rc::Rc, time::Duration};
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::{mpsc, watch},
};
use workload_docker::workflows::system_apps::{CpuMonitor, SystemAppsConfig};
use workload_docker::{
    compose::{
        ComposeProject, DockerAdapter,
        filesystem::{FileChanges, FileMetadata},
    },
    workflows::recorded_compose::{ComposeRun, ComposeRunReport, ComposeRunner},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("worker SOCKET IMAGE ARTIFACT_DIRECTORY".into());
    }
    let stream = UnixStream::connect(&args[1]).await?;
    let (read, mut write) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<(u8, Vec<u8>)>(128);
    let writer = tokio::spawn(async move {
        while let Some((kind, bytes)) = rx.recv().await {
            write.write_u8(kind).await?;
            write.write_u32(bytes.len() as u32).await?;
            write.write_all(&bytes).await?;
        }
        Ok::<_, io::Error>(())
    });
    let output = tx.clone();
    let telemetry = TelemetryObserver::new(1024 * 1024, move |bytes| {
        output
            .try_send((1, bytes.to_vec()))
            .map_err(io::Error::other)
    });
    let mut lines = BufReader::new(read).lines();
    let mut index = 0;
    let docker_socket = std::env::var("ORCHESTRATOR_DOCKER_SOCKET")
        .unwrap_or_else(|_| "/var/run/docker.sock".into());
    let monitor_backend =
        std::env::var("ORCHESTRATOR_MONITOR_BACKEND").unwrap_or_else(|_| "bpf".into());
    let seccomp_profile = std::env::var_os("ORCHESTRATOR_MONITOR_SECCOMP")
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "ORCHESTRATOR_MONITOR_SECCOMP must be UTF-8")
        })
        .transpose()?;
    if monitor_backend == "synthetic" && seccomp_profile.is_none() {
        return Err("synthetic monitoring requires an explicit seccomp profile".into());
    }
    let monitor = CpuMonitor {
        image: args[2].clone(),
        backend: monitor_backend,
        allow_unconfined_seccomp: seccomp_profile.is_none(),
        seccomp_profile,
    };
    let run_prefix = std::env::var("ORCHESTRATOR_RUN_PREFIX").ok();
    let private_telemetry =
        std::env::var_os("ORCHESTRATOR_PRIVATE_TELEMETRY").is_some_and(|value| value == "1");
    if run_prefix
        .as_deref()
        .is_some_and(|prefix| !valid_run_prefix(prefix))
    {
        return Err(
            "ORCHESTRATOR_RUN_PREFIX must be gal- followed by 32 lowercase hexadecimal digits"
                .into(),
        );
    }
    let mut runner = ComposeRunner::new(Rc::new(DockerAdapter::connect(docker_socket).await?));
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let signal_task = tokio::spawn(async move {
        tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
        let _ = stop_tx.send(true);
    });
    loop {
        let line = tokio::select! {
            line = lines.next_line() => line?,
            _ = stop_rx.changed() => break,
        };
        let Some(line) = line else {
            break;
        };
        if line.len() > 128 {
            return Err("oversized control request".into());
        }
        let request: serde_json::Value = serde_json::from_str(&line)?;
        let inject = request["inject_failure"].as_bool().unwrap_or(false);
        index += 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let session = session_id(run_prefix.as_deref(), std::process::id(), now, index)?;
        let observer = telemetry.clone();
        // This await is deliberately not cancelled by a browser/server disconnect.
        let result = recorded_demo(
            &mut runner,
            &monitor,
            Path::new(&args[3]),
            &session,
            inject,
            stop_rx.clone(),
            move |event: &orchestrator::Event| {
                if private_telemetry {
                    observer.observe(&public_event(event));
                } else {
                    observer.observe(event);
                }
            },
        )
        .await;
        let preview = match &result {
            _ if private_telemetry => json!({}),
            Ok(report) => filesystem_preview(report).await,
            Err(_) => json!({}),
        };
        if private_telemetry && let Err(error) = &result {
            eprintln!("private worker failure: {error}");
        }
        let report = match &result {
            Ok(report) if private_telemetry => Some(public_report(report)),
            Ok(report) => Some(serde_json::to_value(report)?),
            Err(_) => None,
        };
        let error = result.as_ref().err().map(|error| {
            if private_telemetry {
                "Experiment failed; diagnostics are retained privately".to_owned()
            } else {
                error.to_string()
            }
        });
        let status = json!({"type":"finished", "session":session, "report":report,
            "filesystem_preview":preview,
            "error":error, "dropped_events":telemetry.dropped_events()});
        if tx.send((2, serde_json::to_vec(&status)?)).await.is_err() {
            break;
        }
        if *stop_rx.borrow() {
            break;
        }
    }
    signal_task.abort();
    drop(telemetry);
    drop(tx);
    writer.await??;
    Ok(())
}

fn public_event(event: &orchestrator::Event) -> orchestrator::Event {
    let mut event = event.clone();
    event.payload = None;
    if event.error.is_some() {
        event.error = Some("Step failed; diagnostics are retained privately".into());
    }
    event
}

fn public_report(report: &ComposeRunReport) -> serde_json::Value {
    json!({"id":report.id, "boundary_clean":report.boundary_clean,
        "recording_finalized":report.recording_finalized, "event_stream_healthy":report.event_stream_healthy,
        "execution_error":report.execution_error.as_ref().map(|_| "Experiment failed; diagnostics are retained privately")})
}

fn valid_run_prefix(value: &str) -> bool {
    value.strip_prefix("gal-").is_some_and(|hex| {
        hex.len() == 32
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn session_id(
    prefix: Option<&str>,
    pid: u32,
    now: u128,
    index: u32,
) -> Result<String, &'static str> {
    match prefix {
        Some(prefix) if valid_run_prefix(prefix) && (1..=999).contains(&index) => {
            Ok(format!("{prefix}-{index}"))
        }
        Some(_) => Err("invalid hosted run identity or more than 999 experiments"),
        None => Ok(format!("doramcap-live-{pid}-{now}-{index}")),
    }
}

/// Fixed fixture only: browser requests cannot choose commands, mounts or files.
async fn recorded_demo(
    runner: &mut ComposeRunner,
    monitor: &CpuMonitor,
    root: &Path,
    session: &str,
    inject: bool,
    mut stop: watch::Receiver<bool>,
    observer: impl Observer + 'static,
) -> Result<ComposeRunReport, Box<dyn std::error::Error>> {
    if runner.blocked() {
        return Err("manager blocked by an unfinished or unverified experiment".into());
    }
    let workspace = root.join(format!("{session}-inputs"));
    tokio::fs::create_dir(&workspace).await?;
    let host = workspace.join("host");
    tokio::fs::create_dir(&host).await?;
    tokio::fs::write(host.join("settings.txt"), b"before run\n").await?;
    tokio::fs::write(host.join("obsolete.txt"), b"delete during run\n").await?;
    let command = "printf 'host output\n' > /workspace/result.txt; printf 'after run\n' > /workspace/settings.txt; /bin/rm /workspace/obsolete.txt; printf 'container output\n' > /container-result.txt; printf changed > /seed-mod; /bin/rm /seed-delete; printf 'volume output\n' > /results/result.txt; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done";
    let fixture = json!({"services":{"writer":{"image":monitor.image,"entrypoint":["/bin/sh","-c"],"command":[command],"volumes":[format!("{}:/workspace",host.display()),"results:/results"],"healthcheck":{"test":["CMD",if inject {"/bin/false"} else {"/bin/true"}],"interval":"100ms","timeout":"1s","retries":1}}},"volumes":{"results":{}}});
    let file = workspace.join("compose.json");
    tokio::fs::write(&file, serde_json::to_vec_pretty(&fixture)?).await?;
    Ok(runner
        .run(
            ComposeRun {
                filesystem: Some(Default::default()),
                id: session.into(),
                artifacts: root.join(session),
                recovery_anchor_dir: std::env::var_os("ORCHESTRATOR_RECOVERY_DIRECTORY")
                    .map(PathBuf::from),
                projects: vec![ComposeProject {
                    key: "demo".into(),
                    directory: workspace,
                    files: vec![file],
                    env_files: vec![],
                    environment: BTreeMap::new(),
                    profiles: vec![],
                    services: vec![],
                    completed_services: vec![],
                    remove_volumes: true,
                }],
                first_party: vec![],
                system_apps: SystemAppsConfig {
                    cpu_monitor: Some(monitor.clone()),
                },
                readiness_seconds: 10,
                stop_seconds: 5,
            },
            async move {
                if !*stop.borrow() {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(6)) => {},
                        _ = stop.changed() => {},
                    }
                }
            },
            observer,
        )
        .await?)
}

/// Bounded metadata preview crosses the worker boundary; no host file serving.
async fn filesystem_preview(report: &ComposeRunReport) -> serde_json::Value {
    let Some(changes) = &report.filesystem_changes else {
        return json!({});
    };
    let mut result = serde_json::Map::new();
    for (key, category) in [
        ("host_binds", &changes.host_binds),
        ("container_layers", &changes.container_layers),
        ("named_volumes", &changes.named_volumes),
    ] {
        let mut rows = Vec::new();
        let mut bytes = 0;
        let read = async {
            let mut reader = FileChanges::open(&category.manifest).await?;
            while rows.len() < 100 {
                let Some(row) = reader.next_change().await? else {
                    break;
                };
                let value = json!({
                    "category": row.category, "source": row.source, "consumers": row.consumers,
                    "path": preview_path(&row.path), "kind": row.kind,
                    "before": row.before.as_ref().map(preview_metadata),
                    "after": row.after.as_ref().map(preview_metadata),
                });
                bytes += serde_json::to_vec(&value).map_err(io::Error::other)?.len();
                if bytes > 32 * 1024 {
                    break;
                }
                rows.push(value);
            }
            Ok::<(), workload_docker::compose::Error>(())
        }
        .await;
        result.insert(
            key.into(),
            json!({"rows":rows,"error":read.err().map(|e|e.to_string())}),
        );
    }
    serde_json::Value::Object(result)
}

// JSON is a display-only preview; the underlying protobuf retains exact Unix bytes.
fn preview_metadata(m: &FileMetadata) -> serde_json::Value {
    json!({"mode":m.mode,"size":m.size,"uid":m.uid,"gid":m.gid,
        "mtime":m.mtime,"ctime":m.ctime,
        "symlink":m.symlink.as_ref().map(|p|preview_path(p))})
}

fn preview_path(path: &Path) -> String {
    path.to_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{:?}", path.as_os_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_events_hide_sensitive_payloads_and_error_details() {
        // Arrange
        let event = orchestrator::Event {
            invocation: orchestrator::Invocation {
                session: "run".into(),
                id: 1,
                step: "step".into(),
                version: 1,
                executor: "local".into(),
                parent: None,
                dependencies: vec![],
            },
            state: orchestrator::State::Failed,
            elapsed_ns: 1,
            duration_ns: 2,
            payload: Some(json!({"secret":"private"})),
            error: Some("private path".into()),
        };

        // Act
        let public = public_event(&event);

        // Assert
        assert!(public.payload.is_none());
        assert_eq!(
            public.error.as_deref(),
            Some("Step failed; diagnostics are retained privately")
        );
        assert_eq!(public.invocation, event.invocation);
        assert_eq!(public.state, event.state);
        assert!(event.payload.is_some());
    }

    #[test]
    fn hosted_run_identity_is_bounded_and_generation_scoped() {
        // Arrange
        let prefix = "gal-0123456789abcdef0123456789abcdef";
        // Act / Assert
        assert_eq!(
            session_id(Some(prefix), 12, 34, 1).unwrap(),
            format!("{prefix}-1")
        );
        assert_eq!(session_id(Some(prefix), 12, 34, 999).unwrap().len(), 40);
        assert!(session_id(Some(prefix), 12, 34, 1000).is_err());
        assert!(session_id(Some("gal-../escape"), 12, 34, 1).is_err());
        assert_eq!(
            session_id(None, 12, 34, 1).unwrap(),
            "doramcap-live-12-34-1"
        );
    }
}
