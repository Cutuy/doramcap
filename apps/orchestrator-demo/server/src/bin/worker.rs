//! Authorized fixed-workflow worker. HTTP remains in a separate unprivileged process.
#[path = "../../../../../libs/orchestrator/examples/support/docker_workflow.rs"]
mod docker_workflow;
use orchestrator::{Observer, telemetry::TelemetryObserver};
use serde_json::json;
use std::{io, path::Path};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::mpsc,
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
        let result = docker_workflow::experiment(
            &args[2],
            Path::new(&args[3]),
            &session,
            inject,
            move |event| observer.observe(event),
        )
        .await;
        let status = json!({"type":"finished", "session":session, "report":result.as_ref().ok(),
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
