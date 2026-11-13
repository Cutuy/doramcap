//! Compose configuration, execution and diagnostic failures.
use super::Diagnostic;
use std::panic::Location;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("selection contains no services")]
    EmptySelection,
    /// Rejected input or precondition; `check` is the source line of the failed check.
    #[error("{message}")]
    Invalid {
        message: String,
        check: &'static Location<'static>,
    },
    #[error("{operation}: {source}")]
    Engine {
        operation: String,
        source: bollard::errors::Error,
    },
    #[error("compose {operation} exited {status:?}: {stderr}")]
    Command {
        operation: String,
        status: Option<i32>,
        stdout: String,
        stderr: String,
    },
    #[error(
        "Compose {operation} deadline exceeded after {seconds}s; capture incomplete; Engine cleanup required"
    )]
    Timeout { operation: String, seconds: u64 },
    #[error("Compose {stream} exceeded {limit} bytes; capture incomplete")]
    OutputLimit { stream: &'static str, limit: usize },
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}
impl Error {
    pub fn diagnostic(&self, operation: &str) -> Diagnostic {
        let mut d = Diagnostic {
            operation: operation.into(),
            kind: "validation".into(),
            message: self.to_string(),
            ..Default::default()
        };
        match self {
            Self::Engine { operation, source } => {
                d.operation = operation.clone();
                d.kind = "engine".into();
                if let bollard::errors::Error::DockerResponseServerError { status_code, .. } =
                    source
                {
                    d.status = Some(*status_code as i64);
                }
            }
            Self::Command {
                operation,
                status,
                stdout,
                stderr,
            } => {
                d.operation = operation.clone();
                d.kind = "compose".into();
                d.status = status.map(i64::from);
                d.stdout = Some(stdout.clone());
                d.stderr = Some(stderr.clone());
            }
            Self::Timeout { operation, .. } => {
                d.kind = "timeout".into();
                d.operation = operation.clone();
            }
            Self::OutputLimit { .. } => d.kind = "output_limit".into(),
            Self::Io(_) => d.kind = "io".into(),
            Self::Json(_) => d.kind = "json".into(),
            Self::Invalid { check, .. } => d.check = check.to_string(),
            Self::EmptySelection => {}
        }
        d
    }
}
pub type Result<T> = std::result::Result<T, Error>;
/// Retain an Engine API failure and the operation that produced it.
pub fn engine(operation: impl Into<String>, source: bollard::errors::Error) -> Error {
    Error::Engine {
        operation: operation.into(),
        source,
    }
}
/// Reject an app configuration or workflow precondition at the caller's source line.
#[track_caller]
pub fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid {
        message: message.into(),
        check: Location::caller(),
    }
}
pub(crate) fn absent(error: &bollard::errors::Error) -> bool {
    matches!(
        error,
        bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}

impl From<docker_engine::Error> for Error {
    #[track_caller]
    fn from(error: docker_engine::Error) -> Self {
        match error {
            docker_engine::Error::Invalid(message) => invalid(message),
            docker_engine::Error::Engine { operation, source } => {
                Self::Engine { operation, source }
            }
            docker_engine::Error::Io(error) => Self::Io(error),
            docker_engine::Error::Json(error) => Self::Json(error),
        }
    }
}

impl From<host_filesystem::Error> for Error {
    #[track_caller]
    fn from(error: host_filesystem::Error) -> Self {
        match error {
            host_filesystem::Error::Invalid(message) => invalid(message),
            host_filesystem::Error::Io(error) => Self::Io(error),
            host_filesystem::Error::Json(error) => Self::Json(error),
        }
    }
}
