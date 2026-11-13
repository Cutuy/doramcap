//! System-app accounting lifecycle and kernel identity, independent of launch mechanism.
pub use crate::launch::{
    SystemAppCgroup, SystemAppsCgroupsInput, system_apps_cgroups_input::Action,
};
use futures_util::future::LocalBoxFuture;
use orchestrator::{LocalExecutor, Step};
use prost::Message;
use std::{
    cell::RefCell,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

pub mod slice;

type Result<T> = std::result::Result<T, String>;
/// Host cgroup v2 mount used for identity and membership checks.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";
/// Typed accounting identity retained in experiment artifacts.
pub const IDENTITY_FILE: &str = "system-app-cgroup.pb";

/// Whether the backend removed its group or retained an empty runtime-owned parent.
pub enum Removal {
    Removed,
    RetainedEmpty(PathBuf),
}

/// Native allocation/adoption and removal; the launcher serializes conflicting operations.
/// The owner retains attempted resources across errors and cancellation.
pub trait Backend {
    /// Establish or adopt the backend-allocated group and return its live host path.
    fn create(&self) -> LocalBoxFuture<'_, Result<PathBuf>>;
    /// Preserve foreign/populated groups; report removal or an empty retained runtime parent.
    fn remove(&self) -> LocalBoxFuture<'_, Result<Removal>>;
}

/// Owns one system-app boundary and shares its recorded identity across step invocations.
/// Dropping a handle leaves resources for the backend's recovery path.
pub struct Group {
    backend: Rc<dyn Backend>,
    identity: RefCell<Option<SystemAppCgroup>>,
}
impl Group {
    /// Bind native mechanics without creating resources.
    pub fn new(backend: Rc<dyn Backend>) -> Self {
        Self {
            backend,
            identity: RefCell::new(None),
        }
    }

    /// Return the recorded identity after verifying the live kernel object.
    pub fn identity(&self) -> Result<SystemAppCgroup> {
        let identity = self
            .identity
            .borrow()
            .clone()
            .ok_or("system-app cgroup has not been created")?;
        verify_identity(&identity)?;
        Ok(identity)
    }

    /// Establish the backend-allocated boundary, verify CPU/memory accounting and persist identity.
    /// Failure or cancellation retains ownership with this handle and the registered factory.
    pub async fn create(&self, artifacts: &Path, startup: Duration) -> Result<()> {
        tokio::time::timeout(startup, async {
            let path = self.backend.create().await?;
            let identity = inspect(&path)?;
            *self.identity.borrow_mut() = Some(identity.clone());
            tokio::fs::write(artifacts.join(IDENTITY_FILE), identity.encode_to_vec())
                .await
                .map_err(error)
        })
        .await
        .map_err(|_| format!("system-app cgroup not created within {startup:?}"))?
    }

    /// Verify the backend-provided host PID belongs to the accounting boundary or a descendant.
    pub fn verify_pid(&self, pid: u32) -> Result<()> {
        let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
            .map_err(|e| format!("system-app PID {pid}: {e}"))?;
        let path = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .ok_or_else(|| format!("system-app PID {pid} has no cgroup v2 membership"))?;
        self.verify_path(&host_path(path)?)
    }

    /// Verify a host cgroup path is within the still-live accounting boundary.
    pub fn verify_path(&self, path: &Path) -> Result<()> {
        let identity = self.identity()?;
        if !path.starts_with(&identity.path) {
            return Err(format!(
                "system app is in {}, expected {} or a descendant",
                path.display(),
                identity.path
            ));
        }
        Ok(())
    }

    /// Verify emptiness, then confirm backend removal or retention of an empty runtime-owned parent.
    /// Missing resources are accepted; changed identity, live descendants and read errors preserve them.
    pub async fn remove(&self) -> Result<Option<SystemAppCgroup>> {
        let identity = self.identity.borrow().clone();
        if let Some(identity) = &identity
            && Path::new(&identity.path).try_exists().map_err(error)?
        {
            verify_identity(identity)?;
            verify_empty(Path::new(&identity.path))?;
        }
        match self.backend.remove().await? {
            Removal::Removed => {
                if let Some(identity) = &identity
                    && Path::new(&identity.path).try_exists().map_err(error)?
                {
                    return Err(format!("{}: cgroup remains after removal", identity.path));
                }
                Ok(None)
            }
            Removal::RetainedEmpty(path) => {
                if path.try_exists().map_err(error)? {
                    if let Some(identity) = &identity {
                        verify_identity(identity)?;
                    }
                    verify_empty(&path)?;
                    Ok(Some(inspect(&path)?))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

/// Create/remove an accounting boundary using the launcher's native mechanics.
/// Create returns kernel identity; Remove returns any retained empty runtime-owned parent.
pub struct SystemAppsCgroups {
    group: Rc<Group>,
}

/// Retain the launcher's group across fresh step instances and cancelled invocations.
pub fn register(executor: &mut LocalExecutor, group: Rc<Group>) {
    executor.register::<SystemAppsCgroups>(move || SystemAppsCgroups {
        group: group.clone(),
    });
}
impl Step for SystemAppsCgroups {
    const ID: Option<&'static str> = Some("system-apps.cgroups");
    const DISPLAY_NAME: Option<&'static str> = Some("System apps cgroups");
    type Input = SystemAppsCgroupsInput;
    type Output = Option<SystemAppCgroup>;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        let result = async {
            match Action::try_from(input.action).map_err(error)? {
                Action::Create => {
                    let artifacts = Path::new(&input.artifacts);
                    if !artifacts.is_absolute() || !artifacts.is_dir() || input.readiness_seconds == 0 {
                        return Err("cgroup creation requires an existing absolute artifact directory and a positive readiness budget".into());
                    }
                    self.group.create(artifacts, Duration::from_secs(input.readiness_seconds)).await?;
                    for pid in input.pids { self.group.verify_pid(pid)?; }
                    self.group.identity().map(Some)
                }
                Action::Remove => self.group.remove().await,
                Action::Unspecified => Err("system-app cgroup action missing".into()),
            }
        }.await;
        result.map_err(orchestrator::Error::message)
    }
}

/// Read the kernel identity only when cgroup v2 CPU and memory accounting are available.
pub fn inspect(path: &Path) -> Result<SystemAppCgroup> {
    if !path.join("cgroup.controllers").is_file()
        || !path.join("cpu.stat").is_file()
        || !path.join("memory.current").is_file()
    {
        return Err(format!(
            "{}: system apps require cgroup v2 CPU and memory accounting",
            path.display()
        ));
    }
    Ok(SystemAppCgroup {
        path: path.to_string_lossy().into_owned(),
        id: std::fs::metadata(path).map_err(error)?.ino(),
    })
}

/// Reject a replaced kernel cgroup at the recorded host path.
pub fn verify_identity(identity: &SystemAppCgroup) -> Result<()> {
    if std::fs::metadata(&identity.path).map_err(error)?.ino() != identity.id {
        return Err("system-app cgroup identity changed".into());
    }
    Ok(())
}

/// Require no live processes anywhere beneath this group; failed reads cannot establish emptiness.
pub fn verify_empty(path: &Path) -> Result<()> {
    let events = std::fs::read_to_string(path.join("cgroup.events")).map_err(error)?;
    if !events.lines().any(|line| line == "populated 0") {
        return Err(format!(
            "{}: accounting cgroup still contains live processes",
            path.display()
        ));
    }
    Ok(())
}

/// Convert a non-root absolute kernel cgroup path to its host mount path.
pub fn host_path(group: &str) -> Result<PathBuf> {
    let relative = group
        .strip_prefix('/')
        .ok_or("cgroup path must be absolute")?;
    if relative.is_empty()
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!("invalid system-app cgroup path: {group:?}"));
    }
    Ok(Path::new(CGROUP_ROOT).join(relative))
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
