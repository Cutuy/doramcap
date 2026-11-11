mod configuration;
mod project;
use configuration::ComposeConfiguration;
pub use project::ComposeExecution;
pub use project::stop_timeout;

use super::{ResolvedComposeProject, Result, error::invalid};
use docker_engine::Engine;
pub use docker_engine::files::{digest, private_write};
pub use docker_engine::{ARTIFACTS_LABEL, CONTROLLER_LABEL, RUN_LABEL};
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub const EXPERIMENT_OWNER_LABEL: &str = "io.doramcap.experiment-owner";
enum StartupOutcome {
    Running(tokio::task::JoinHandle<Result<(String, String)>>),
    Finished(Result<(String, String)>),
}
type StartupCommand = tokio::sync::Mutex<Option<StartupOutcome>>;

/// Shared local Docker connection used by Compose configuration and workflow steps.
/// Clones share pending startup commands; callers serialize conflicting lifecycle operations.
/// Project views borrow a frozen plan; resources survive cancellation and adapter destruction.
#[derive(Clone)]
pub struct DockerAdapter {
    pub engine: Engine,
    starting: Arc<tokio::sync::Mutex<BTreeMap<String, Arc<StartupCommand>>>>,
}
impl DockerAdapter {
    /// Return the configured Unix socket path for trusted deployment wiring.
    pub fn socket_path(&self) -> &Path {
        self.engine.socket_path()
    }
    /// Connect to an absolute Unix socket and negotiate the Engine API version.
    /// Engine and Compose share this socket; connection errors retain Engine diagnostics.
    pub async fn connect(socket: impl Into<String>) -> Result<Self> {
        Ok(Self {
            engine: Engine::connect(socket).await?,
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
pub fn repository_scope(repository: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "/tmp/doramcap-experiments-{}-{}",
        unsafe { libc::geteuid() },
        project_name("experiment", repository)
    ))
}

#[cfg(test)]
pub(crate) mod tests;

/// Normalize real app fixtures with Compose's own YAML parser; no Engine is contacted.
#[cfg(test)]
pub(crate) fn fixture_model(project: super::ComposeProject) -> serde_json::Value {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let directory = tempfile::tempdir().unwrap();
                let socket_path = directory.path().join("engine.sock");
                // Keep the path for Bollard, but close the listener so Compose probes fail immediately.
                drop(std::os::unix::net::UnixListener::bind(&socket_path).unwrap());
                let socket = socket_path.to_str().unwrap();
                let adapter = DockerAdapter {
                    engine: Engine::new(socket, 120).unwrap(),
                    starting: Default::default(),
                };
                adapter
                    .configuration()
                    .normalize("fixture", &project, "fixture")
                    .await
                    .unwrap()
                    .model
            })
    })
    .join()
    .unwrap()
}
