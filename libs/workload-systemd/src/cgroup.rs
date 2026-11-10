//! Systemd-owned system-app slices shared by native and Compose launchers.
use crate::{
    Result, Scope, error,
    manager::{Manager, STOP_REPORT_MARGIN, Unit, value},
};
use futures_util::StreamExt;
use orchestrator::{LocalExecutor, Step};
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::{
    cell::RefCell,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
use workload_lifecycle::launch::SystemAppCgroup;
use zbus::proxy::CacheProperties;
use zbus_systemd::systemd1::SliceProxy;

pub use crate::{SystemAppsCgroupsInput, system_apps_cgroups_input::Action};

/// Create or remove the registered launcher's accounting slices with lifecycle events.
/// Create returns the kernel identity; Remove returns None after verified removal.
/// The launcher and factory retain ownership when an invocation fails or is cancelled.
pub struct SystemAppsCgroups {
    slice: SystemAppSlice,
}

/// Register the shared step with the launcher's existing ownership handle.
pub fn register(executor: &mut LocalExecutor, slice: SystemAppSlice) {
    executor.register::<SystemAppsCgroups>(move || SystemAppsCgroups {
        slice: slice.clone(),
    });
}

impl Step for SystemAppsCgroups {
    const ID: Option<&'static str> = Some("systemd.system-apps.cgroups");
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
                    self.slice
                        .create(artifacts, Duration::from_secs(input.readiness_seconds))
                        .await?;
                    self.slice.identity().map(Some)
                }
                Action::Remove => self.slice.remove().await.map(|()| None),
                Action::Unspecified => Err("system-app cgroup action missing".into()),
            }
        }
        .await;
        result.map_err(orchestrator::Error::message)
    }
}

/// Host cgroup v2 mount used for kernel identity and membership checks.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";
/// Typed runtime identity retained in the experiment's artifacts.
pub const IDENTITY_FILE: &str = "system-app-cgroup.pbtxt";
const COMPOSE_SCOPE_PREFIX: &str = "doramcap-compose";

fn compose_prefix(repository: &Path) -> Result<String> {
    Ok(format!(
        "{COMPOSE_SCOPE_PREFIX}-{}",
        crate::repository_id(repository)?
    ))
}

pub(crate) fn native_name(scope: &Scope) -> String {
    format!("{}-system.slice", scope.prefix)
}

/// Owns an experiment's parent and nested system-app slice from the first start attempt.
/// Creation and removal are explicit; dropping this handle leaves resources for recovery.
/// Clones share the connection and recorded kernel identity across step invocations.
#[derive(Clone)]
pub struct SystemAppSlice {
    manager: Rc<Manager>,
    prefix: String,
    identity: Rc<RefCell<Option<SystemAppCgroup>>>,
    owned_parent: bool,
}

impl SystemAppSlice {
    /// Connect for a Compose repository without creating resources.
    /// Compose and native runs have separate ownership names and recovery paths.
    pub async fn connect(repository: &Path) -> Result<Self> {
        Ok(Self::new(
            Rc::new(Manager::connect().await?),
            compose_prefix(repository)?,
        ))
    }

    pub(crate) fn native(manager: Rc<Manager>, scope: &Scope) -> Self {
        Self {
            // Native recovery also owns parent slices created by earlier releases.
            owned_parent: false,
            ..Self::new(manager, scope.prefix.clone())
        }
    }

    fn new(manager: Rc<Manager>, prefix: String) -> Self {
        Self {
            manager,
            prefix,
            identity: Rc::new(RefCell::new(None)),
            owned_parent: true,
        }
    }

    /// Native parent name for Compose's generated execution configuration.
    pub fn compose_name(repository: &Path) -> Result<String> {
        Ok(format!("{}-system.slice", compose_prefix(repository)?))
    }

    /// Slice name passed to native services or Docker's systemd cgroup driver.
    pub fn name(&self) -> String {
        format!("{}-system.slice", self.prefix)
    }

    /// Return the recorded identity after checking that the kernel cgroup still matches.
    pub fn identity(&self) -> Result<SystemAppCgroup> {
        let identity = self
            .identity
            .borrow()
            .clone()
            .ok_or("system-app cgroup has not been created")?;
        if std::fs::metadata(&identity.path).map_err(error)?.ino() != identity.id {
            return Err("system-app cgroup identity changed".into());
        }
        Ok(identity)
    }

    fn parent(&self) -> String {
        format!("{}.slice", self.prefix)
    }

    fn description(name: &str) -> String {
        format!("Doramcap experiment accounting: {name}")
    }

    /// Create both slices within the startup budget and persist their system-app identity.
    /// Existing units fail admission; the caller retains this handle on error or cancellation.
    pub async fn create(&mut self, artifacts: &Path, startup: Duration) -> Result<()> {
        tokio::time::timeout(startup, async {
            self.start_slice(&self.parent()).await?;
            self.start_slice(&self.name()).await?;
            let path = self.path(&self.name()).await?;
            if !path.join("cgroup.controllers").is_file() || !path.join("memory.current").is_file()
            {
                return Err(format!(
                    "{}: system apps require cgroup v2 memory accounting",
                    path.display()
                ));
            }
            let identity = SystemAppCgroup {
                slice: self.name(),
                id: std::fs::metadata(&path).map_err(error)?.ino(),
                path: path.to_string_lossy().into_owned(),
            };
            *self.identity.borrow_mut() = Some(identity.clone());
            let pool = DescriptorPool::decode(
                include_bytes!(concat!(env!("OUT_DIR"), "/systemd_plan.bin")).as_slice(),
            )
            .map_err(error)?;
            let descriptor = pool
                .get_message_by_name("doramcap.launch.v1.SystemAppCgroup")
                .ok_or("system-app cgroup descriptor missing")?;
            let mut message = DynamicMessage::new(descriptor);
            message.transcode_from(&identity).map_err(error)?;
            tokio::fs::write(artifacts.join(IDENTITY_FILE), message.to_text_format())
                .await
                .map_err(error)
        })
        .await
        .map_err(|_| format!("system-app cgroup not created within {startup:?}"))?
    }

    async fn start_slice(&self, name: &str) -> Result<()> {
        let mut events = self
            .manager
            .manager
            .receive_job_removed()
            .await
            .map_err(error)?;
        let properties = vec![
            ("Description".into(), value(Self::description(name))?),
            ("AddRef".into(), value(true)?),
            ("MemoryAccounting".into(), value(true)?),
            ("IOAccounting".into(), value(true)?),
            ("TasksAccounting".into(), value(true)?),
        ];
        let job = self
            .manager
            .manager
            .start_transient_unit(name.into(), "fail".into(), properties, vec![])
            .await
            .map_err(error)?;
        self.manager.started(name, &mut events, job).await
    }

    async fn path(&self, name: &str) -> Result<PathBuf> {
        let object = self
            .manager
            .manager
            .get_unit(name.into())
            .await
            .map_err(error)?;
        let slice = SliceProxy::builder(&self.manager.connection)
            .path(object)
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(error)?;
        host_path(&slice.control_group().await.map_err(error)?)
    }

    /// Verify the live host PID belongs to this slice or a nested descendant.
    /// The launcher must obtain the PID from its own authoritative backend.
    pub fn verify_pid(&self, pid: u32) -> Result<()> {
        let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
            .map_err(|e| format!("system-app PID {pid}: {e}"))?;
        let path = membership
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .ok_or_else(|| format!("system-app PID {pid} has no cgroup v2 membership"))?;
        self.verify_path(&host_path(path)?)
    }

    fn verify_path(&self, path: &Path) -> Result<()> {
        let identity = self.identity()?;
        let parent = Path::new(&identity.path);
        if !path.starts_with(parent) {
            return Err(format!(
                "system app is in {}, expected {} or a descendant",
                path.display(),
                parent.display()
            ));
        }
        Ok(())
    }

    pub(crate) async fn verify_unit(&self, unit: &Unit) -> Result<()> {
        if unit.service.slice().await.map_err(error)? != self.name() {
            return Err(format!(
                "{}: system-app slice assignment differs",
                unit.name
            ));
        }
        let pid = unit.service.main_pid().await.map_err(error)?;
        if pid != 0 {
            self.verify_pid(pid)?;
        }
        let path = unit.service.control_group().await.map_err(error)?;
        // Completed oneshots have no live cgroup; systemd retains their authoritative Slice.
        if path.is_empty() {
            let state = unit.unit.active_state().await.map_err(error)?;
            let exited = pid == 0 && unit.unit.sub_state().await.map_err(error)? == "exited";
            if !matches!(state.as_str(), "inactive" | "failed") && !exited {
                return Err(format!("{}: live system-app cgroup unavailable", unit.name));
            }
            return self.identity().map(|_| ());
        }
        self.verify_path(&host_path(&path)?)
    }

    /// Remove owned, empty slices after backend shutdown and evidence finalization.
    /// Missing slices are accepted; foreign ownership, live processes and read failures preserve them.
    pub async fn remove(&self) -> Result<()> {
        self.remove_slices(self.owned_parent).await
    }

    async fn remove_slices(&self, owned_parent: bool) -> Result<()> {
        for name in [self.name(), self.parent()] {
            let mut removed = self
                .manager
                .manager
                .receive_unit_removed()
                .await
                .map_err(error)?;
            if self.manager.units(&name).await?.is_empty() {
                continue;
            }
            let unit = self.manager.watch(&name).await?;
            if (owned_parent || name != self.parent())
                && unit.unit.description().await.map_err(error)? != Self::description(&name)
            {
                return Err(format!(
                    "{name}: accounting slice is not owned by this launcher"
                ));
            }
            // Read after subscribing; StopUnit also cancels any pending creation job.
            let object = self
                .manager
                .manager
                .get_unit(name.clone())
                .await
                .map_err(error)?;
            let slice = SliceProxy::builder(&self.manager.connection)
                .path(object)
                .cache_properties(CacheProperties::No)
                .build()
                .await
                .map_err(error)?;
            let group = slice.control_group().await.map_err(error)?;
            if !group.is_empty() {
                let path = host_path(&group)?;
                if name == self.name() && self.identity.borrow().is_some() {
                    self.identity()?;
                }
                let events = std::fs::read_to_string(path.join("cgroup.events")).map_err(error)?;
                if !events.lines().any(|line| line == "populated 0") {
                    return Err(format!(
                        "{name}: accounting slice still contains live processes"
                    ));
                }
            }
            let mut events = self
                .manager
                .manager
                .receive_job_removed()
                .await
                .map_err(error)?;
            let job = self
                .manager
                .manager
                .stop_unit(name.clone(), "replace".into())
                .await
                .map_err(error)?;
            tokio::time::timeout(
                STOP_REPORT_MARGIN,
                self.manager.started(&name, &mut events, job),
            )
            .await
            .map_err(|_| format!("{name}: slice stop not confirmed"))??;
            self.manager.release(&name).await;
            tokio::time::timeout(STOP_REPORT_MARGIN, async {
                while !self.manager.units(&name).await?.is_empty() {
                    let signal = removed
                        .next()
                        .await
                        .ok_or_else(|| format!("{name}: unit removal notifications ended"))?;
                    if signal.args().map_err(error)?.id != name {
                        continue;
                    }
                }
                Ok::<_, String>(())
            })
            .await
            .map_err(|_| format!("{name}: slice unload not confirmed"))??;
            if !group.is_empty() && host_path(&group)?.try_exists().map_err(error)? {
                return Err(format!("{name}: cgroup remains after removal"));
            }
        }
        Ok(())
    }
}

fn host_path(group: &str) -> Result<PathBuf> {
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
