//! Compose configuration, execution and diagnostic failures.
use super::Diagnostic;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("selection contains no services")]
    EmptySelection,
    #[error("at least one Compose file is required")]
    NoComposeFile,
    #[error("unresolved Compose environment: {detail}")]
    UnresolvedEnvironment { detail: String },
    #[error("{service}: prepare a named local image before the experiment")]
    ImageNotPrepared { service: String },
    #[error("{service}: anonymous volumes need explicit named-volume retention settings")]
    AnonymousVolume { service: String },
    #[error("{service}: image volume {target} needs an explicit mount for ownership/retention")]
    ImageVolume { service: String, target: String },
    #[error("{service}: zero replicas is unsupported; deselect via profiles")]
    ZeroReplicas { service: String },
    #[error("container name already occupied: {name}")]
    NameOccupied { name: String },
    #[error("reserved label {label}")]
    ReservedLabel { label: String },
    #[error("unscoped {resource}: explicit resource name requires external ownership")]
    UnscopedResource { resource: String },
    #[error("runtime project resource {resource} belongs to another run or controller")]
    ForeignResource { resource: String },
    #[error("{services}: {detail}")]
    NoJournal { services: String, detail: String },
    #[error("{0}")]
    Invalid(String),
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
    /// Stable rejection reason and affected subject for Review; faults have no reason.
    /// Image-volume subjects identify both service and mount target; journal subjects list services.
    pub fn reason(&self) -> Option<(&'static str, String)> {
        Some(match self {
            Self::EmptySelection => ("EmptySelection", String::new()),
            Self::NoComposeFile => ("NoComposeFile", String::new()),
            Self::UnresolvedEnvironment { .. } => ("UnresolvedEnvironment", String::new()),
            Self::ImageNotPrepared { service } => ("ImageNotPrepared", service.clone()),
            Self::AnonymousVolume { service } => ("AnonymousVolume", service.clone()),
            Self::ImageVolume { service, target } => {
                ("ImageVolume", format!("{service}: {target}"))
            }
            Self::ZeroReplicas { service } => ("ZeroReplicas", service.clone()),
            Self::NameOccupied { name } => ("NameOccupied", name.clone()),
            Self::ReservedLabel { label } => ("ReservedLabel", label.clone()),
            Self::UnscopedResource { resource } => ("UnscopedResource", resource.clone()),
            Self::ForeignResource { resource } => ("ForeignResource", resource.clone()),
            Self::NoJournal { services, .. } => ("NoJournal", services.clone()),
            _ => return None,
        })
    }

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
            _ => {}
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
/// Describe a rejected app configuration or workflow precondition.
pub fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
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
    fn from(error: docker_engine::Error) -> Self {
        match error {
            docker_engine::Error::Invalid(message) => Self::Invalid(message),
            docker_engine::Error::Engine { operation, source } => {
                Self::Engine { operation, source }
            }
            docker_engine::Error::Io(error) => Self::Io(error),
            docker_engine::Error::Json(error) => Self::Json(error),
        }
    }
}

impl From<host_filesystem::Error> for Error {
    fn from(error: host_filesystem::Error) -> Self {
        match error {
            host_filesystem::Error::Invalid(message) => Self::Invalid(message),
            host_filesystem::Error::Io(error) => Self::Io(error),
            host_filesystem::Error::Json(error) => Self::Json(error),
        }
    }
}
