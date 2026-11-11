#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{operation}: {source}")]
    Engine {
        operation: String,
        source: bollard::errors::Error,
    },
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
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
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn engine(operation: impl Into<String>, source: bollard::errors::Error) -> Error {
    Error::Engine {
        operation: operation.into(),
        source,
    }
}
pub(crate) fn invalid(message: impl Into<String>) -> Error {
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
