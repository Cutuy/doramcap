//! Docker connection, ownership cleanup and filesystem operations shared by backends.
pub mod error;
pub mod files;
pub mod filesystem;
pub mod overlay;
mod scope;
use bollard::Docker;
pub use error::{Error, Result};
use error::{engine, invalid};
pub use scope::OwnedResources;
use std::path::Path;

pub const RUN_LABEL: &str = "io.doramcap.run";
pub const ARTIFACTS_LABEL: &str = "io.doramcap.artifacts";
pub const CONTROLLER_LABEL: &str = "io.doramcap.controller";

/// Negotiated local Docker connection; clones share the underlying client.
/// Callers own lifecycle ordering and cleanup of the resources they create.
#[derive(Clone)]
pub struct Engine {
    docker: Docker,
    socket: String,
}
impl Engine {
    /// Connect to an absolute Unix socket and negotiate the Engine API version.
    /// Errors retain the Engine operation and source diagnostic.
    pub async fn connect(socket: impl Into<String>) -> Result<Self> {
        let socket = socket.into();
        if !Path::new(&socket).is_absolute() {
            return Err(invalid("Docker socket must be absolute"));
        }
        let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| engine("connect", e))?
            .negotiate_version()
            .await
            .map_err(|e| engine("negotiate", e))?;
        Ok(Self { docker, socket })
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
    #[cfg(test)]
    pub(crate) fn test_connection(socket: &str, timeout: u64) -> Self {
        Self {
            docker: Docker::connect_with_unix(socket, timeout, bollard::API_DEFAULT_VERSION)
                .unwrap(),
            socket: socket.into(),
        }
    }
}

/// Kill an owned child process group when its command finishes or is cancelled.
/// The caller creates this PGID at spawn and ensures it is never its own group.
pub struct ProcessGroup(pub u32);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: the caller assigned this PGID to its owned child at spawn.
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
