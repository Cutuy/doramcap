//! Authorized fixed-workflow worker. HTTP remains in a separate unprivileged process.
use orchestrator::{Observer, telemetry::TelemetryObserver};
use serde_json::json;
use std::{collections::BTreeMap, rc::Rc, time::Duration};
use std::{io, path::Path};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
};
use workload_docker::{
    compose::{
        ComposeProject, DockerAdapter,
        filesystem::{FileChanges, FileMetadata},
    },
    workflows::recorded_compose::{ComposeRun, ComposeRunReport, ComposeRunner, CpuMonitor},
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
    let mut runner = ComposeRunner::new(Rc::new(
        DockerAdapter::connect("/var/run/docker.sock").await?,
    ));
    loop {
        let line = tokio::select! {
            line = lines.next_line() => line?,
            _ = tokio::signal::ctrl_c() => break,
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
        let session = format!("doramcap-live-{}-{now}-{index}", std::process::id());
        let observer = telemetry.clone();
        // This await is deliberately not cancelled by a browser/server disconnect.
        let result = recorded_demo(
            &mut runner,
            &args[2],
            Path::new(&args[3]),
            &session,
            inject,
            move |event: &orchestrator::Event| observer.observe(event),
        )
        .await;
        let preview = match &result {
            Ok(report) => filesystem_preview(report).await,
            Err(_) => json!({}),
        };
        let status = json!({"type":"finished", "session":session, "report":result.as_ref().ok(),
            "filesystem_preview":preview,
            "error":result.as_ref().err().map(ToString::to_string), "dropped_events":telemetry.dropped_events()});
        if tx.send((2, serde_json::to_vec(&status)?)).await.is_err() {
            break;
        }
    }
    drop(telemetry);
    drop(tx);
    writer.await??;
    Ok(())
}

/// Fixed fixture only: browser requests cannot choose commands, mounts or files.
async fn recorded_demo(
    runner: &mut ComposeRunner,
    image: &str,
    root: &Path,
    session: &str,
    inject: bool,
    observer: impl Observer + 'static,
) -> Result<ComposeRunReport, Box<dyn std::error::Error>> {
    let workspace = root.join(format!("{session}-inputs"));
    tokio::fs::create_dir(&workspace).await?;
    let host = workspace.join("host");
    tokio::fs::create_dir(&host).await?;
    tokio::fs::write(host.join("settings.txt"), b"before run\n").await?;
    tokio::fs::write(host.join("obsolete.txt"), b"delete during run\n").await?;
    let command = "printf 'host output\n' > /workspace/result.txt; printf 'after run\n' > /workspace/settings.txt; /bin/rm /workspace/obsolete.txt; printf 'container output\n' > /container-result.txt; printf changed > /seed-mod; /bin/rm /seed-delete; printf 'volume output\n' > /results/result.txt; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done";
    let fixture = json!({"services":{"writer":{"image":image,"entrypoint":["/bin/sh","-c"],"command":[command],"volumes":[format!("{}:/workspace",host.display()),"results:/results"],"healthcheck":{"test":["CMD",if inject {"/bin/false"} else {"/bin/true"}],"interval":"100ms","timeout":"1s","retries":1}}},"volumes":{"results":{}}});
    let file = workspace.join("compose.json");
    tokio::fs::write(&file, serde_json::to_vec_pretty(&fixture)?).await?;
    Ok(runner
        .run(
            ComposeRun {
                filesystem: Default::default(),
                id: session.into(),
                artifacts: root.join(session),
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
                monitor: CpuMonitor {
                    image: image.into(),
                    backend: "bpf".into(),
                    allow_unconfined_seccomp: true,
                },
                readiness_seconds: 10,
                stop_seconds: 5,
            },
            tokio::time::sleep(Duration::from_secs(6)),
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
