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
