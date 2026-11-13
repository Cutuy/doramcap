//! Native systemd allocation and placement for the shared accounting step.
use crate::{
    Result, Scope, error,
    manager::{Manager, STOP_REPORT_MARGIN, Unit, value},
};
use futures_util::future::LocalBoxFuture;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};
use workload_lifecycle::cgroup::{self, Backend, Group, Removal};
use zbus::proxy::CacheProperties;
use zbus_systemd::systemd1::SliceProxy;

pub(crate) fn native_name(scope: &Scope) -> String {
    format!("{}-system.slice", scope.prefix)
}

/// Slice holding the run's user units, each in its own service group.
pub(crate) fn user_name(scope: &Scope) -> String {
    format!("{}-user.slice", scope.prefix)
}

/// Systemd owns native unit allocation and keeps its names out of the shared contract.
pub struct SystemAppSlice {
    manager: Rc<Manager>,
    prefix: String,
}
impl SystemAppSlice {
    /// Connect to the native systemd backend for this repository without creating resources.
    pub async fn connect(repository: &Path) -> Result<Self> {
        Ok(Self::native(
            Rc::new(Manager::connect().await?),
            &Scope::of(repository)?,
        ))
    }
    pub(crate) fn native(manager: Rc<Manager>, scope: &Scope) -> Self {
        Self {
            manager,
            prefix: scope.prefix.clone(),
        }
    }
    pub fn name(&self) -> String {
        format!("{}-system.slice", self.prefix)
    }
    fn parent(&self) -> String {
        format!("{}.slice", self.prefix)
    }
    fn description(name: &str) -> String {
        format!("Doramcap experiment accounting: {name}")
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
        cgroup::host_path(&slice.control_group().await.map_err(error)?)
    }

    async fn remove_slices(&self) -> Result<()> {
        let user = format!("{}-user.slice", self.prefix);
        for name in [self.name(), user, self.parent()] {
            if self.manager.units(&name).await?.is_empty() {
                continue;
            }
            let unit = self.manager.watch(&name).await?;
            if name == self.name()
                && unit.unit.description().await.map_err(error)? != Self::description(&name)
            {
                return Err(format!(
                    "{name}: accounting slice is not owned by this launcher"
                ));
            }
            // Drop this launcher's reference so the stopped slice can unload.
            self.manager.release(&name).await;
            tokio::time::timeout(
                STOP_REPORT_MARGIN,
                cgroup::slice::stop(&self.manager.manager, &name),
            )
            .await
            .map_err(|_| format!("{name}: slice removal not confirmed"))??;
        }
        Ok(())
    }
}
impl Backend for SystemAppSlice {
    fn create(&self) -> LocalBoxFuture<'_, Result<PathBuf>> {
        Box::pin(async {
            self.start_slice(&self.parent()).await?;
            self.start_slice(&self.name()).await?;
            self.path(&self.name()).await
        })
    }
    fn remove(&self) -> LocalBoxFuture<'_, Result<Removal>> {
        Box::pin(async {
            self.remove_slices().await?;
            Ok(Removal::Removed)
        })
    }
}

pub(crate) async fn verify_unit(group: &Group, slice: &str, unit: &Unit) -> Result<()> {
    if unit.service.slice().await.map_err(error)? != slice {
        return Err(format!(
            "{}: system-app slice assignment differs",
            unit.name
        ));
    }
    let pid = unit.service.main_pid().await.map_err(error)?;
    if pid != 0 {
        group.verify_pid(pid)?;
    }
    let path = unit.service.control_group().await.map_err(error)?;
    if path.is_empty() {
        let state = unit.unit.active_state().await.map_err(error)?;
        let exited = pid == 0 && unit.unit.sub_state().await.map_err(error)? == "exited";
        if !matches!(state.as_str(), "inactive" | "failed") && !exited {
            return Err(format!("{}: live system-app cgroup unavailable", unit.name));
        }
        return group.identity().map(|_| ());
    }
    group.verify_path(&cgroup::host_path(&path)?)
}
