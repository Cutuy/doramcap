use super::{Error, Result, error::invalid};
use std::{
    collections::BTreeMap,
    io::Write,
    os::unix::process::{CommandExt, ExitStatusExt},
    path::Path,
    process::{ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

// Compose may launch its CLI plugin as a child. Kill the owned process group on
// timeout, I/O failure or cancellation; Engine cleanup remains a separate action.
pub(crate) struct ProcessGroup(pub(crate) u32);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: this PGID was assigned to our child at spawn, never the caller's.
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}
pub(crate) async fn capture(
    reader: impl AsyncRead + Unpin,
    stream: &'static str,
) -> Result<String> {
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

/// Execute Compose with explicit environment, bounded output and a deadline.
/// Logs argv, cwd and outcome to stderr; environment maps and captured output are omitted.
/// Errors retain diagnostics; cancellation kills the CLI group and leaves Engine cleanup to callers.
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
        .env("COMPOSE_PROGRESS", "plain");
    execute(command, args, timeout, std::io::stderr()).await
}

async fn execute(
    mut command: Command,
    args: &[String],
    timeout: Duration,
    writer: impl Write,
) -> Result<(String, String)> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command.as_std_mut().process_group(0);
    let mut record = CommandLog::start(command.as_std(), timeout, writer);
    let mut child = command.spawn()?;
    let group = ProcessGroup(child.id().ok_or_else(|| invalid("missing Compose PID"))?);
    record.outcome = "cancelled";
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
        Ok(Ok((stdout, stderr, status))) => {
            record.status = Some(status);
            record.outcome = if status.success() {
                "success"
            } else {
                "failed"
            };
            if status.success() {
                Ok((stdout, stderr))
            } else {
                Err(Error::Command {
                    operation: args.join(" "),
                    status: status.code(),
                    stdout,
                    stderr,
                })
            }
        }
        Ok(Err(error)) => {
            record.outcome = if matches!(error, Error::OutputLimit { .. }) {
                "output_limit"
            } else {
                "io_failed"
            };
            record.status = child.wait().await.ok();
            Err(error)
        }
        Err(_) => {
            record.outcome = "timed_out";
            record.status = child.wait().await.ok();
            Err(Error::Timeout {
                operation: args.join(" "),
                seconds: timeout.as_secs(),
            })
        }
    }
}

static NEXT_COMMAND_ID: AtomicU64 = AtomicU64::new(1);

// A scoped record emits completion when cancellation drops the command future.
struct CommandLog<W: Write> {
    writer: W,
    id: String,
    started: Instant,
    outcome: &'static str,
    status: Option<ExitStatus>,
}
impl<W: Write> CommandLog<W> {
    fn start(command: &std::process::Command, timeout: Duration, mut writer: W) -> Self {
        let id = format!(
            "{}-{}",
            std::process::id(),
            NEXT_COMMAND_ID.fetch_add(1, Ordering::Relaxed)
        );
        let started = Instant::now();
        // Debug formatting preserves argument boundaries and escapes embedded newlines.
        let _ = writeln!(
            writer,
            "docker_cli start id={id} program={:?} args={:?} cwd={:?} timeout_ms={}",
            command.get_program(),
            command.get_args().collect::<Vec<_>>(),
            command.get_current_dir(),
            timeout.as_millis()
        );
        Self {
            writer,
            id,
            started,
            outcome: "start_failed",
            status: None,
        }
    }
}
impl<W: Write> Drop for CommandLog<W> {
    fn drop(&mut self) {
        // Logging errors must not interrupt process termination or workload teardown.
        let _ = writeln!(
            self.writer,
            "docker_cli finish id={} outcome={} exit_code={:?} signal={:?} duration_ms={}",
            self.id,
            self.outcome,
            self.status.and_then(|status| status.code()),
            self.status.and_then(|status| status.signal()),
            self.started.elapsed().as_millis()
        );
    }
}

/// Verify the sole listener on `port` is IPv4 loopback and belongs to `pid`.
/// The caller must recheck its owned container PID after this snapshot.
pub(crate) async fn verify_listener(pid: u32, port: u16) -> Result<()> {
    use std::collections::HashSet;
    if pid == 0 || port == 0 {
        return Err(invalid("Dora listener PID and port must be nonzero"));
    }
    let mut sockets = HashSet::new();
    for (path, loopback) in [("/proc/net/tcp", "0100007F"), ("/proc/net/tcp6", "")] {
        let file = tokio::fs::File::open(path).await?;
        let mut data = Vec::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .await?;
        if data.len() > 4 * 1024 * 1024 {
            return Err(invalid("Dora listener table exceeds its size limit"));
        }
        let table =
            std::str::from_utf8(&data).map_err(|_| invalid("invalid TCP listener table"))?;
        for row in table.lines().skip(1) {
            let fields: Vec<_> = row.split_whitespace().collect();
            if fields.len() < 10 {
                return Err(invalid("malformed TCP listener table"));
            }
            if fields[3] != "0A" {
                continue;
            }
            let (address, raw_port) = fields[1]
                .split_once(':')
                .ok_or_else(|| invalid("invalid TCP listener address"))?;
            let listener_port = u16::from_str_radix(raw_port, 16)
                .map_err(|_| invalid("invalid TCP listener port"))?;
            if listener_port != port {
                continue;
            }
            if address != loopback {
                return Err(invalid("Dora port has a non-loopback or IPv6 listener"));
            }
            sockets.insert(fields[9].to_owned());
        }
    }
    if sockets.len() != 1 {
        return Err(invalid(
            "Dora port does not have exactly one loopback listener",
        ));
    }
    let mut descriptors = tokio::fs::read_dir(format!("/proc/{pid}/fd")).await?;
    while let Some(entry) = descriptors.next_entry().await? {
        match tokio::fs::read_link(entry.path()).await {
            Ok(target) => {
                let name = target.to_string_lossy();
                if name
                    .strip_prefix("socket:[")
                    .and_then(|value| value.strip_suffix(']'))
                    .is_some_and(|inode| sockets.contains(inode))
                {
                    return Ok(());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(invalid("Dora listener belongs to another process"))
}

#[cfg(test)]
mod tests;
