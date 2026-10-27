//! External steps implemented by Python, Node, or another executable.
//!
//! Protocol: stdin is one JSON object containing `protocol: 2` and `input`, followed by EOF. Stdout must be
//! exactly one JSON value. Diagnostics go to stderr. Successful stdout and stderr
//! become the step result; nonzero exits return `Error::Process`.
//!
//! Commands run directly, without an implicit shell. All three streams and child
//! exit are handled concurrently in the caller's async task. Captures are bounded.
//! Timeout/I/O failure kills and waits for the direct child; caller cancellation
//! uses Tokio's kill-on-drop behavior. This is not process-tree containment: the
//! domain launcher must own descendant/cgroup cleanup. A script starting persistent
//! work must return owned resource IDs and provide an explicit stop operation.

use crate::{Error, Result, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

/// Per-invocation command and protocol input. Defaults bound time and captured I/O.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInput {
    /// Executable path/name. PATH lookup follows the host environment.
    pub program: String,
    /// Arguments passed literally; no shell interpolation.
    #[serde(default)]
    pub args: Vec<String>,
    /// Script-specific payload, available as request `input`.
    pub input: Value,
    /// Wall-clock budget including I/O and direct-child completion.
    #[serde(default = "timeout_default")]
    pub timeout_ms: u64,
    /// Maximum captured stdout bytes (default 1 MiB).
    #[serde(default = "stdout_default")]
    pub max_stdout_bytes: usize,
    /// Maximum captured stderr bytes (default 64 KiB).
    #[serde(default = "stderr_default")]
    pub max_stderr_bytes: usize,
}
fn timeout_default() -> u64 {
    30_000
}
fn stdout_default() -> usize {
    1024 * 1024
}
fn stderr_default() -> usize {
    64 * 1024
}
impl ProcessInput {
    /// Construct a command with default timeout/capture limits and no arguments.
    pub fn new(program: impl Into<String>, input: Value) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            input,
            timeout_ms: timeout_default(),
            max_stdout_bytes: stdout_default(),
            max_stderr_bytes: stderr_default(),
        }
    }
}

/// Captured successful external step result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessOutput {
    /// JSON value decoded from stdout.
    pub value: Value,
    /// Bounded diagnostics, decoded with replacement for invalid UTF-8.
    pub stderr: String,
}

/// Transient external step. Runs on Tokio without launching an executor/thread pool.
#[derive(Default)]
pub struct ProcessStep;

impl Step for ProcessStep {
    type Input = ProcessInput;
    type Output = ProcessOutput;

    async fn run(&mut self, input: ProcessInput) -> Result<ProcessOutput> {
        if input.program.is_empty()
            || input.timeout_ms == 0
            || input.max_stdout_bytes == 0
            || input.max_stderr_bytes == 0
        {
            return Err(Error::Configuration(
                "external program, timeout and capture limits must be nonempty/positive".into(),
            ));
        }
        let request = serde_json::to_vec(&json!({
            "protocol": 2, "input": input.input,
        }))
        .map_err(|source| Error::Serialization {
            boundary: "external request",
            source,
        })?;
        let mut child = tokio::process::Command::new(&input.program)
            .args(&input.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| io("spawn external step", source))?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let operation = async {
            tokio::try_join!(
                async {
                    let result = stdin.write_all(&request).await;
                    drop(stdin);
                    // Wait/inspect the exit even if the child rejected input early.
                    Ok::<_, Error>(result)
                },
                read_bounded(stdout, input.max_stdout_bytes, "stdout"),
                read_bounded(stderr, input.max_stderr_bytes, "stderr"),
                async {
                    child
                        .wait()
                        .await
                        .map_err(|source| io("wait external step", source))
                },
            )
        };
        let result =
            match tokio::time::timeout(Duration::from_millis(input.timeout_ms), operation).await {
                Ok(result) => result,
                Err(_) => Err(Error::Timeout),
            };
        match result {
            Ok((written, stdout, stderr, status)) => {
                let stderr = String::from_utf8_lossy(&stderr).into_owned();
                if !status.success() {
                    return Err(Error::Process {
                        code: status.code(),
                        stderr,
                    });
                }
                written.map_err(|source| io("write external input", source))?;
                let value =
                    serde_json::from_slice(&stdout).map_err(|source| Error::Serialization {
                        boundary: "external stdout",
                        source,
                    })?;
                Ok(ProcessOutput { value, stderr })
            }
            Err(error) => {
                child
                    .kill()
                    .await
                    .map_err(|source| io("kill/wait external step", source))?;
                Err(error)
            }
        }
    }
}

fn io(operation: &'static str, source: std::io::Error) -> Error {
    Error::Io { operation, source }
}
async fn read_bounded(
    reader: impl AsyncRead + Unpin,
    limit: usize,
    boundary: &'static str,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .await
        .map_err(|source| io("read external stream", source))?;
    if bytes.len() > limit {
        return Err(Error::Limit { boundary, limit });
    }
    Ok(bytes)
}
