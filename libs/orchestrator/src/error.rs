use thiserror::Error;

/// Errors from invocation boundaries, user code, or external step processes.
#[derive(Debug, Error)]
pub enum Error {
    /// Domain failure supplied by a step.
    #[error("{0}")]
    Step(String),
    /// JSON encoding or decoding failed before entry or after the body returned.
    #[error("{boundary} serialization: {source}")]
    Serialization {
        /// Boundary being checked.
        boundary: &'static str,
        /// Underlying codec failure.
        source: serde_json::Error,
    },
    /// Encoded payload or process stream exceeded its configured bound.
    #[error("{boundary} exceeds {limit} bytes")]
    Limit {
        /// Payload or stream.
        boundary: &'static str,
        /// Maximum permitted length.
        limit: usize,
    },
    /// Dependency refers to another executor.
    #[error("dependency belongs to another executor")]
    InvalidDependency,
    /// No factory was registered for the requested Rust step type.
    #[error("no factory registered for {0}")]
    UnregisteredStep(&'static str),
    /// Invocation IDs cannot wrap.
    #[error("invocation ID space exhausted")]
    IdExhausted,
    /// A supplied configuration is invalid.
    #[error("invalid configuration: {0}")]
    Configuration(String),
    /// External process or stream failure.
    #[error("{operation}: {source}")]
    Io {
        /// Operation being attempted.
        operation: &'static str,
        /// Underlying OS error.
        source: std::io::Error,
    },
    /// Process returned a failure status; stderr is bounded by its request limits.
    #[error("external step failed (exit code {code:?}): {stderr}")]
    Process {
        /// Exit code, or `None` for signal termination on Unix.
        code: Option<i32>,
        /// Captured stderr, decoded lossily.
        stderr: String,
    },
    /// External process exceeded its wall-clock budget.
    #[error("external step timed out")]
    Timeout,
}

impl Error {
    /// Construct a domain error without a custom error type.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Step(message.into())
    }
}

/// Result returned by step bodies and the runner.
pub type Result<T> = std::result::Result<T, Error>;
