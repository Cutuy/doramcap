//! Bounded official CLI control; no shell or custom XML-RPC implementation.
use crate::{Result, error};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

pub(crate) async fn command(command: &mut Command, seconds: u64) -> Result<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(error)?;
    let mut stdout = child.stdout.take().unwrap().take(65537);
    let mut stderr = child.stderr.take().unwrap().take(65537);
    let capture = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let (_, _, status) = tokio::try_join!(
            stdout.read_to_end(&mut out),
            stderr.read_to_end(&mut err),
            child.wait(),
        )
        .map_err(error)?;
        if out.len() > 65536 || err.len() > 65536 {
            return Err("control output exceeds 64 KiB".into());
        }
        let out = String::from_utf8(out).map_err(error)?;
        if !status.success() {
            return Err(format!(
                "control exit {status}: {out} {}",
                String::from_utf8_lossy(&err)
            ));
        }
        Ok(out)
    };
    match tokio::time::timeout(Duration::from_secs(seconds), capture).await {
        Ok(result) => result,
        Err(_) => {
            child.kill().await.map_err(error)?;
            Err("control deadline exceeded (child killed and reaped)".into())
        }
    }
}

pub(crate) async fn ctl(run: &crate::MonitorRun, operation: &str, seconds: u64) -> Result<String> {
    let mut cmd = Command::new(&run.supervisorctl);
    cmd.env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .arg("-c")
        .arg(std::path::Path::new(&run.output_directory).join("supervisord.conf"))
        .arg("-s")
        .arg(format!("unix://{}/control.sock", run.output_directory))
        .arg(operation);
    if operation != "shutdown" {
        cmd.arg("system-monitor");
    }
    command(&mut cmd, seconds).await
}

pub(crate) fn running(status: &str) -> bool {
    let mut words = status.split_whitespace();
    words.next() == Some("system-monitor") && words.next() == Some("RUNNING")
}

pub(crate) fn monitor_pid(status: &str) -> Result<u32> {
    if !running(status) {
        return Err(format!("monitor is not RUNNING: {status}"));
    }
    let words: Vec<_> = status.split_whitespace().collect();
    if words.get(2) != Some(&"pid") {
        return Err("Supervisor status PID absent".into());
    }
    words
        .get(3)
        .ok_or("Supervisor status PID absent")?
        .trim_end_matches(',')
        .parse::<u32>()
        .map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn bounded_control_reports_exit_and_kills_timeout() {
        // Arrange
        let mut failure = Command::new("/bin/false");
        // Act / Assert
        assert!(
            command(&mut failure, 1)
                .await
                .unwrap_err()
                .contains("control exit")
        );
        // Arrange
        let mut slow = Command::new("/bin/sleep");
        slow.arg("10");
        let start = tokio::time::Instant::now();
        // Act
        let error = command(&mut slow, 1).await.unwrap_err();
        // Assert
        assert!(error.contains("killed and reaped"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn status_requires_exact_program_and_running_state() {
        // Arrange / Act / Assert
        assert!(running("system-monitor RUNNING pid 123, uptime 0:00:01"));
        for status in [
            "system-monitor STOPPED",
            "system-monitor STARTING",
            "other RUNNING",
            "system-monitor BACKOFF",
            "system-monitor EXITED",
        ] {
            assert!(!running(status));
        }
    }
}
