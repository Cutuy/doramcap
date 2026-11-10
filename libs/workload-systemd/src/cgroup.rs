//! Native systemd allocation and placement for the shared accounting step.
use crate::{
    Result, Scope, error,
    manager::{Manager, STOP_REPORT_MARGIN, Unit, value},
};
use futures_util::{StreamExt, future::LocalBoxFuture};
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
            if name != self.parent()
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
                let path = cgroup::host_path(&group)?;
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
            if !group.is_empty() && cgroup::host_path(&group)?.try_exists().map_err(error)? {
                return Err(format!("{name}: cgroup remains after removal"));
            }
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
