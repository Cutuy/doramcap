//! The run's shared filesystem, retained until verified shutdown.
use crate::{Result, RunPlan, Scope, SystemdPlan, error};
use rustix::mount::{MountFlags, UnmountFlags};
use std::path::PathBuf;

/// Scope entry holding the run's /dev/shm filesystem.
const SHM_DIRECTORY: &str = "shm";
/// The same access mode as the host's /dev/shm, within the run's private filesystem.
const SHM_OPTIONS: &std::ffi::CStr = c"mode=1777";

pub(crate) fn directory(scope: &Scope) -> PathBuf {
    scope.directory.join(SHM_DIRECTORY)
}

/// Validate source/target values already parsed by systemd, with native optional-source behavior.
pub(crate) fn validate_native_bind(
    id: &str,
    source: &str,
    target: &str,
    optional: bool,
) -> Result<()> {
    let resolved = match std::fs::canonicalize(source) {
        Ok(path) => path,
        Err(e) if optional && e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{id}: bind {source}:{target}: {e}")),
    };
    if exposes_host_shm(&resolved)? {
        return Err(format!(
            "{id}: bind {source}:{target} exposes whole host shm; select PASSTHROUGH"
        ));
    }
    Ok(())
}

fn exposes_host_shm(source: &std::path::Path) -> Result<bool> {
    workload_lifecycle::files::exposes_host_shm_tree(source).map_err(error)
}

/// Reject full host-shm exposure through typed binds before creating run resources.
pub(crate) fn validate_binds(run: &RunPlan, plan: &SystemdPlan) -> Result<()> {
    if workload_lifecycle::plan::validate_host_shm_access(run.host_shm_access)?
        != workload_lifecycle::launch::HostShmAccess::Blocked
    {
        return Ok(());
    }
    let binds = plan
        .programs
        .iter()
        .map(|app| (&app.id, &app.binds))
        .chain(plan.app_units.iter().map(|app| (&app.id, &app.binds)));
    for (id, binds) in binds {
        for bind in binds {
            let source = match std::fs::canonicalize(&bind.source) {
                Ok(source) => source,
                // Existing generated output directories may appear only at startup.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("{id}: {}: {e}", bind.source)),
            };
            if exposes_host_shm(&source)? {
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
pub(crate) fn create(scope: &Scope) -> Result<()> {
    let shm = directory(scope);
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
pub(crate) fn remove(scope: &Scope) -> Result<()> {
    let path = directory(scope);
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
