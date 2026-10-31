mod configuration;
mod project;
mod scope;
use super::{
    ResolvedComposeProject, Result,
    error::{engine, invalid},
};
use bollard::Docker;
pub use configuration::ComposeConfiguration;
pub use project::ComposeExecution;
use scope::ExperimentResources;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use tokio::io::AsyncWriteExt;
pub(crate) const RUN_LABEL: &str = "io.doramcap.run";
pub(crate) const ARTIFACTS_LABEL: &str = "io.doramcap.artifacts";
pub(crate) const CONTROLLER_LABEL: &str = "io.doramcap.controller";
pub(crate) const DIRECTORY_OWNER_LABEL: &str = "io.doramcap.directory-owner";
type StartupCommand = tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(String, String)>>>>;
/// Shared Docker connection providing configuration, execution and ownership capabilities.
/// Clones share pending startup commands; callers serialize conflicting lifecycle operations.
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
    /// Connect to `socket` and negotiate the Engine API version.
    ///
    /// Returns an adapter using the absolute Unix socket for both Engine and CLI
    /// operations. Engine requests use a 120-second timeout. Relative paths return
    /// a validation error; connection/negotiation failures retain Engine details.
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

    /// Borrow configuration processing; no resource ownership is transferred.
    pub fn configuration(&self) -> ComposeConfiguration<'_> {
        ComposeConfiguration { adapter: self }
    }
    /// Borrow execution operations for one immutable plan; dropping the view changes no resources.
    pub fn project<'a>(&'a self, plan: &'a ResolvedComposeProject) -> ComposeExecution<'a> {
        ComposeExecution {
            adapter: self,
            plan,
        }
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

pub(crate) fn project_name(run: &str, key: &str) -> String {
    format!("dc-{run}-{key}")
}
/// Stable project identity for one canonical repository/key.
pub(crate) fn repository_project_name(key: &str, repository: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    format!(
        "dc-{}-{key}",
        &digest(repository.as_os_str().as_bytes())[..16]
    )
}
/// Private admission and temporary OverlayFS directory for one repository.
pub(crate) fn repository_scope(repository: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "/tmp/doramcap-experiments-{}-{}",
        unsafe { libc::geteuid() },
        repository_project_name("experiment", repository)
    ))
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
