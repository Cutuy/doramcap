//! Docker connection, ownership cleanup and native storage observations.
pub mod error;
pub mod filesystem;
pub mod labels;
pub mod namespace;
mod scope;
use bollard::Docker;
pub use error::{Error, Result};
use error::{engine, invalid};
pub use scope::OwnedResources;
use std::path::Path;

const ENGINE_REQUEST_TIMEOUT_SECONDS: u64 = 120;

pub use labels::{ARTIFACTS_LABEL, CONTROLLER_LABEL, RUN_LABEL};

/// Local Docker connection; clones share the underlying client.
/// Callers own lifecycle ordering and cleanup of the resources they create.
#[derive(Clone)]
pub struct Engine {
    docker: Docker,
    socket: String,
}
impl Engine {
    /// Create a client for an absolute Unix socket with the supplied request timeout in seconds.
    /// Makes no Engine request; invalid paths or transport setup return errors.
    pub fn new(socket: impl Into<String>, seconds: u64) -> Result<Self> {
        let socket = socket.into();
        if !Path::new(&socket).is_absolute() {
            return Err(invalid("Docker socket must be absolute"));
        }
        let docker = Docker::connect_with_unix(&socket, seconds, bollard::API_DEFAULT_VERSION)
            .map_err(|error| engine("connect", error))?;
        Ok(Self { docker, socket })
    }
    /// Connect to an absolute Unix socket and negotiate the Engine API version.
    /// Errors retain the Engine operation and source diagnostic.
    pub async fn connect(socket: impl Into<String>) -> Result<Self> {
        let mut connection = Self::new(socket, ENGINE_REQUEST_TIMEOUT_SECONDS)?;
        connection.docker = connection
            .docker
            .negotiate_version()
            .await
            .map_err(|error| engine("negotiate", error))?;
        Ok(connection)
    }
    /// Borrow the client connected to this Engine's socket.
    pub fn client(&self) -> &Docker {
        &self.docker
    }
    /// Absolute Unix socket used by this client and native Docker commands.
    pub fn socket_path(&self) -> &Path {
        Path::new(&self.socket)
    }
    /// Borrow resources bearing this controller label; the reservation names its exclusive owner.
    /// No resources change until quiesce or removal is called.
    pub fn resources<'a>(&'a self, scope: &'a str, reservation: &'a str) -> OwnedResources<'a> {
        OwnedResources {
            engine: self,
            scope,
            owner: reservation,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

/// Resolved Docker mounts and owned container identities.
pub mod docker_proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.docker.v1.rs"));
}
