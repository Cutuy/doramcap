mod configuration;
mod project;
mod scope;
use configuration::ComposeConfiguration;
pub use project::ComposeExecution;
use scope::ExperimentResources;

use super::{
    ResolvedComposeProject, Result,
    error::{engine, invalid},
};
use bollard::Docker;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use tokio::io::AsyncWriteExt;

pub(crate) const RUN_LABEL: &str = "io.doramcap.run";
pub(crate) const CONTROLLER_LABEL: &str = "io.doramcap.controller";
pub(crate) const EXPERIMENT_OWNER_LABEL: &str = "io.doramcap.experiment-owner";
type StartupCommand = tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(String, String)>>>>;

/// Shared local Docker connection used by Compose configuration and workflow steps.
/// Clones share pending startup commands; callers serialize conflicting lifecycle operations.
/// Project views borrow a frozen plan; resources survive cancellation and adapter destruction.
#[derive(Clone)]
pub struct DockerAdapter {
    pub(crate) docker: Docker,
    socket: String,
    starting: Arc<tokio::sync::Mutex<BTreeMap<String, Arc<StartupCommand>>>>,
}
impl DockerAdapter {
    /// Return the configured Unix socket path for trusted deployment wiring.
    pub fn socket_path(&self) -> &Path {
        Path::new(&self.socket)
    }
    /// Connect to an absolute Unix socket and negotiate the Engine API version.
    /// Engine and Compose share this socket; connection errors retain Engine diagnostics.
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
        Ok(Self {
            docker,
            socket,
            starting: Default::default(),
        })
    }
    /// Borrow the runtime view for a frozen project; no Docker resources are created.
    pub fn project<'a>(&'a self, plan: &'a ResolvedComposeProject) -> ComposeExecution<'a> {
        ComposeExecution {
            adapter: self,
            plan,
        }
    }

    pub(crate) fn configuration(&self) -> ComposeConfiguration<'_> {
        ComposeConfiguration { adapter: self }
    }

    pub(crate) fn experiment_resources<'a>(&'a self, scope: &'a str) -> ExperimentResources<'a> {
        ExperimentResources {
            adapter: self,
            scope,
        }
    }
}
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 40
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(invalid(
            "identity must be 1..40 lowercase letters, digits or hyphens",
        ));
    }
    Ok(())
}

/// Stable identity for one repository/project key; run IDs remain ownership labels.
pub(crate) fn project_name(key: &str, repository: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    format!(
        "dc-{}-{key}",
        &digest(repository.as_os_str().as_bytes())[..16]
    )
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub(crate) async fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
