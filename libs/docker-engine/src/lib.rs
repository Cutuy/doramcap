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
use std::{io, os::unix::process::CommandExt, path::Path};
use tokio::process::{Child, Command};

const ENGINE_REQUEST_TIMEOUT_SECONDS: u64 = 120;

pub const RUN_LABEL: &str = "io.doramcap.run";
pub const ARTIFACTS_LABEL: &str = "io.doramcap.artifacts";
pub const CONTROLLER_LABEL: &str = "io.doramcap.controller";

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

/// Kill an owned child process group when this guard is dropped.
pub struct ProcessGroup(i32);
impl ProcessGroup {
    /// Spawn a child in a fresh process group and return the child and its cleanup guard.
    /// Spawn and PID lookup errors are returned as I/O errors.
    pub fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
        command.kill_on_drop(true);
        command.as_std_mut().process_group(0);
        let child = command.spawn()?;
        let group = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .filter(|&id| id > 1)
            .ok_or_else(|| io::Error::other("spawned child has no valid process group ID"))?;
        Ok((child, Self(group)))
    }
}
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: spawn created this group and excluded special kill targets.
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

/// Resolved Docker mounts, owned container identities and filesystem views.
pub mod docker_proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.docker.v1.rs"));
}
/// Binary filesystem record schema; files contain length-delimited messages.
pub mod filesystem_proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.filesystem.v1.rs"));
}
