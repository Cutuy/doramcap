use super::{Error, Result, error::invalid};
use std::{
    collections::BTreeMap, os::unix::process::CommandExt, path::Path, process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

// Compose may launch its CLI plugin as a child. Kill the owned process group on
// timeout, I/O failure or cancellation; Engine cleanup remains a separate action.
struct ProcessGroup(u32);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: this PGID was assigned to our child at spawn, never the caller's.
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}
async fn capture(reader: impl AsyncRead + Unpin, stream: &'static str) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::OutputLimit {
            stream,
            limit: 16 * 1024 * 1024,
        });
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Execute Docker Compose directly with bounded diagnostics and a deadline.
///
/// `socket` selects Engine, `directory` sets cwd, `environment` supplies explicit
/// variables, and `args` are passed verbatim after `docker compose`. PATH is inherited;
/// Compose control variables are fixed here. No shell expansion occurs.
///
/// Returns `(stdout, stderr)` decoded lossily on success. Each stream is limited
/// to 16 MiB. Spawn/I/O failures, excess output, nonzero exit and elapsed `timeout`
/// return typed errors. Output collection and child waiting run concurrently.
///
/// Owns the child process group and kills it on scope exit, including cancellation.
/// This does not stop Engine-managed containers; the caller owns their teardown.
pub(crate) async fn compose(
    socket: &str,
    directory: &Path,
    environment: &BTreeMap<String, String>,
    args: &[String],
    timeout: Duration,
) -> Result<(String, String)> {
    let mut command = Command::new("docker");
    command
        .args(["--host", &format!("unix://{socket}"), "compose"])
        .args(args)
        .current_dir(directory)
        .env_clear()
        .env(
            "PATH",
            std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
        )
        .envs(environment)
        .env("COMPOSE_PROFILES", "")
        .env("COMPOSE_DISABLE_ENV_FILE", "true")
        .env("COMPOSE_ANSI", "never")
        .env("COMPOSE_PROGRESS", "plain")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command.as_std_mut().process_group(0);
    let mut child = command.spawn()?;
    let group = ProcessGroup(child.id().ok_or_else(|| invalid("missing Compose PID"))?);
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let operation = async {
        tokio::try_join!(capture(out, "stdout"), capture(err, "stderr"), async {
            child.wait().await.map_err(Error::from)
        })
    };
    let result = tokio::time::timeout(timeout, operation).await;
    drop(group);
    match result {
        Ok(Ok((stdout, stderr, status))) if status.success() => Ok((stdout, stderr)),
        Ok(Ok((stdout, stderr, status))) => Err(Error::Command {
            operation: args.join(" "),
            status: status.code(),
            stdout,
            stderr,
        }),
        Ok(Err(error)) => {
            let _ = child.wait().await;
            Err(error)
        }
        Err(_) => {
            let _ = child.wait().await;
            Err(Error::Timeout {
                operation: args.join(" "),
                seconds: timeout.as_secs(),
            })
        }
    }
}
