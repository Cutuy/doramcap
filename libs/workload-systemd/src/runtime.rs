//! Observed setup and verified removal of the run's native runtime resources.
use crate::{
    PreparationInput, Result, RunPlan, Scope, SystemdPlan, error,
    manager::{Manager, ROOTS, remove_path, runtime_files},
};
use orchestrator::{LocalExecutor, Step};
use rustix::mount::{MountFlags, UnmountFlags};
use std::{
    path::PathBuf,
    rc::{Rc, Weak},
};

pub(crate) fn register(
    executor: &mut LocalExecutor,
    manager: Rc<Manager>,
    scope: Scope,
    child: Weak<LocalExecutor>,
) {
    let setup_manager = manager.clone();
    executor.register::<PrepareRuntime>(move || PrepareRuntime {
        manager: setup_manager.clone(),
        scope: scope.clone(),
    });
    executor.register::<RemoveRuntime>(move || RemoveRuntime {
        manager: manager.clone(),
        executor: child.clone(),
    });
}

/// Live resources after successful setup; the scope owns partial setup before this output exists.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PreparedRuntime {
    pub scope: Scope,
    pub filesystem: crate::filesystem::PreparedFilesystem,
}

/// Load native shm declarations and create the run's shared filesystem before apps start.
/// The running handle owns `scope` before this call, including partial failure or cancellation.
pub(crate) struct PrepareRuntime {
    manager: Rc<Manager>,
    scope: Scope,
}
impl Step for PrepareRuntime {
    const ID: Option<&'static str> = Some("systemd.runtime.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare systemd runtime");
    type Input = PreparationInput;
    type Output = PreparedRuntime;

    async fn run(&mut self, input: PreparationInput) -> orchestrator::Result<PreparedRuntime> {
        self.prepare(input)
            .await
            .map_err(orchestrator::Error::message)
    }
}
impl PrepareRuntime {
    async fn prepare(&self, input: PreparationInput) -> Result<PreparedRuntime> {
        let run = input.run.ok_or("run missing")?;
        let plan = input.plan.ok_or("plan missing")?;
        let scope = &self.scope;
        let blocked = workload_lifecycle::plan::validate_host_shm_access(run.host_shm_access)?
            == workload_lifecycle::launch::HostShmAccess::Blocked;
        let names = link_inputs(&self.manager, scope, &plan).await?;
        let resolved =
            crate::filesystem::resolve(Some(&self.manager), scope, &run, &plan, false).await?;
        if blocked {
            for name in names {
                let unit = self.manager.watch(&name).await?;
                let service = &unit.service;
                let mut binds = service.bind_paths().await.map_err(error)?;
                binds.extend(service.bind_read_only_paths().await.map_err(error)?);
                for (source, target, optional, _) in &binds {
                    validate_native_bind(&name, source, target, *optional)?;
                }
                let root = std::path::Path::new(host_filesystem::SHM_PATH);
                let private_mount = binds
                    .iter()
                    .any(|(_, target, _, _)| root.starts_with(target))
                    || service
                        .temporary_file_system()
                        .await
                        .map_err(error)?
                        .iter()
                        .any(|(target, _)| root.starts_with(target))
                    || service.private_devices().await.map_err(error)?;
                if !private_mount {
                    let drop_in = std::path::Path::new(crate::SYSTEMD_RUNTIME_UNITS)
                        .join(format!("{name}.d"));
                    tokio::fs::create_dir_all(&drop_in).await.map_err(error)?;
                    tokio::fs::write(
                        drop_in.join(SHM_DROP_IN),
                        format!(
                            "[Service]\nBindPaths={}:{}\n",
                            shm_directory(scope).display(),
                            host_filesystem::SHM_PATH,
                        ),
                    )
                    .await
                    .map_err(error)?;
                }
            }
            self.manager.manager.reload().await.map_err(error)?;
        }
        let filesystem =
            crate::filesystem::prepare(&self.manager, scope, &run, &plan, resolved).await?;
        if blocked {
            create_shm(scope)?;
        }
        Ok(PreparedRuntime {
            scope: scope.clone(),
            filesystem,
        })
    }
}

/// Link captured user app units before inspecting any effective settings; never start them.
pub(crate) async fn link_inputs(
    manager: &Manager,
    scope: &Scope,
    plan: &SystemdPlan,
) -> Result<Vec<String>> {
    let units = scope.directory.join("units");
    tokio::fs::create_dir_all(&units).await.map_err(error)?;
    let mut copies = Vec::new();
    let mut names = Vec::new();
    for file in &plan.unit_files {
        let id = crate::unit_stem(file)?;
        let name = scope.unit(id);
        let copy = units.join(&name);
        tokio::fs::copy(file, &copy).await.map_err(error)?;
        copies.push(copy.to_string_lossy().into_owned());
        names.push(name);
    }
    if !copies.is_empty() {
        manager
            .manager
            .link_unit_files(copies, true, true)
            .await
            .map_err(error)?;
        manager.manager.reload().await.map_err(error)?;
    }
    Ok(names)
}

/// Verify owned units have stopped, then remove runtime files and the run's shm mount.
/// Failures retain remaining resources for explicit cleanup; absent resources are accepted.
pub(crate) struct RemoveRuntime {
    manager: Rc<Manager>,
    executor: Weak<LocalExecutor>,
}
impl Step for RemoveRuntime {
    const ID: Option<&'static str> = Some("systemd.runtime.remove");
    const DISPLAY_NAME: Option<&'static str> = Some("Remove systemd runtime");
    type Input = Scope;
    type Output = ();

    async fn run(&mut self, scope: Scope) -> orchestrator::Result<()> {
        self.remove(&scope)
            .await
            .map_err(orchestrator::Error::message)
    }
}
impl RemoveRuntime {
    async fn remove(&self, scope: &Scope) -> Result<()> {
        let loaded = self.manager.units(&format!("{}*", scope.prefix)).await?;
        let mut units = Vec::new();
        for (name, load, _) in &loaded {
            if name.ends_with(".service") && load != "not-found" {
                units.push(self.manager.watch(name).await?);
            }
        }
        let journal_run = tokio::fs::read_link(scope.directory.join(crate::journal::POINTER))
            .await
            .ok();
        if let Some(artifacts) = &journal_run {
            crate::journal::remember(artifacts, scope, &units).await;
        }
        let results = futures_util::future::join_all(
            units
                .iter_mut()
                .map(|unit| self.manager.stop_and_wait(unit)),
        )
        .await;
        for result in results {
            result?;
        }
        if let Some(artifacts) = journal_run
            && let Some(executor) = self.executor.upgrade()
        {
            let _ = workload_lifecycle::journal::finish(&executor, &artifacts).await;
        }
        // Discard abandoned overlays after every owned service has stopped.
        host_filesystem::overlay::discard(&crate::filesystem::workspace(scope))
            .await
            .map_err(error)?;
        // Remove accounting only after every owned service has stopped.
        self.executor
            .upgrade()
            .ok_or("runtime executor unavailable")?
            .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(
                workload_lifecycle::launch::SystemAppsCgroupsInput {
                    action: workload_lifecycle::cgroup::Action::Remove.into(),
                    ..Default::default()
                },
            )
            .await
            .map_err(error)?;
        for (name, ..) in loaded {
            self.manager.release(&name).await;
        }
        for path in runtime_files(scope).await? {
            remove_path(&path).await?;
        }
        remove_path(&scope.directory.join("units")).await?;
        remove_path(&scope.directory.join(ROOTS)).await?;
        remove_shm(scope)?;
        self.manager.manager.reload().await.map_err(error)?;
        remove_path(&scope.directory.join(crate::ACTIVE_RUN)).await?;
        remove_path(&scope.directory.join(crate::journal::POINTER)).await?;
        let remaining = self.manager.remaining(scope).await?;
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(format!("units remain after removal: {remaining:?}"))
        }
    }
}

/// Run shm defaults, separate from user app declarations.
const SHM_DROP_IN: &str = "20-doramcap-shm.conf";

/// Scope entry holding the run's /dev/shm filesystem.
const SHM_DIRECTORY: &str = "shm";
/// The same access mode as the host's /dev/shm, within the run's private filesystem.
const SHM_OPTIONS: &std::ffi::CStr = c"mode=1777";

pub(crate) fn shm_directory(scope: &Scope) -> PathBuf {
    scope.directory.join(SHM_DIRECTORY)
}

/// Validate source/target values already parsed by systemd, with native optional-source behavior.
fn validate_native_bind(id: &str, source: &str, target: &str, optional: bool) -> Result<()> {
    let resolved = match std::fs::canonicalize(source) {
        Ok(path) => path,
        Err(e) if optional && e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{id}: bind {source}:{target}: {e}")),
    };
    if host_filesystem::exposes_host_shm_tree(&resolved).map_err(error)? {
        return Err(format!(
            "{id}: bind {source}:{target} exposes whole host shm; select PASSTHROUGH"
        ));
    }
    Ok(())
}

/// Reject full host-shm exposure through typed binds before creating run resources.
pub(crate) fn validate_binds(run: &RunPlan, plan: &SystemdPlan) -> Result<()> {
    if workload_lifecycle::plan::validate_host_shm_access(run.host_shm_access)?
        != workload_lifecycle::launch::HostShmAccess::Blocked
    {
        return Ok(());
    }
    let binds = plan.programs.iter().map(|app| (&app.id, &app.binds));
    for (id, binds) in binds {
        for bind in binds {
            let source = match std::fs::canonicalize(&bind.source) {
                Ok(source) => source,
                // Existing generated output directories may appear only at startup.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("{id}: {}: {e}", bind.source)),
            };
            if host_filesystem::exposes_host_shm_tree(&source).map_err(error)? {
                return Err(format!(
                    "{id}: bind {}:{} exposes whole host shm; select PASSTHROUGH",
                    bind.source, bind.target,
                ));
            }
        }
    }
    Ok(())
}

/// Mount the run's shared filesystem; partial creation remains visible to scope cleanup.
fn create_shm(scope: &Scope) -> Result<()> {
    let shm = shm_directory(scope);
    std::fs::create_dir(&shm).map_err(error)?;
    rustix::mount::mount(
        "tmpfs",
        &shm,
        "tmpfs",
        MountFlags::NOSUID | MountFlags::NODEV,
        Some(SHM_OPTIONS),
    )
    .map_err(|e| format!("{}: {e}", shm.display()))
}

/// Unmount the owned filesystem after all units stop; a busy mount remains for explicit cleanup.
fn remove_shm(scope: &Scope) -> Result<()> {
    let path = shm_directory(scope);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    }
    match rustix::mount::unmount(&path, UnmountFlags::empty()) {
        Ok(()) | Err(rustix::io::Errno::INVAL) => {}
        Err(e) => return Err(format!("{}: {e}", path.display())),
    }
    std::fs::remove_dir(&path).map_err(|e| format!("{}: {e}", path.display()))
}
