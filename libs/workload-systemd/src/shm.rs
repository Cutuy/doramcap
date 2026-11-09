//! Kernel resources shared by a run's units and retained until verified shutdown.
use crate::{Result, RunPlan, Scope, SystemdPlan, error};
use rustix::mount::{MountFlags, UnmountFlags};
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt, path::PathBuf};
use std::{os::fd::AsFd, os::unix::fs::MetadataExt};

/// Scope entries holding the run's POSIX shm filesystem and pinned SysV IPC namespace.
const SHM_DIRECTORY: &str = "shm";
const IPC_NAMESPACE: &str = "ipc";
/// The same access mode as the host's /dev/shm, within the run's private filesystem.
const SHM_OPTIONS: &std::ffi::CStr = c"mode=1777";
/// Only the controller and service manager need to open the namespace pin.
const NAMESPACE_MODE: u32 = 0o600;

pub(crate) fn directory(scope: &Scope) -> PathBuf {
    scope.directory.join(SHM_DIRECTORY)
}

pub(crate) fn namespace(scope: &Scope) -> PathBuf {
    scope.directory.join(IPC_NAMESPACE)
}

/// Validate a native IPC namespace with the kernel and compare it to the host namespace.
pub(crate) fn is_host_namespace(path: &str) -> Result<bool> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
        .open(path)
        .map_err(|e| format!("IPCNamespacePath={path}: {e}"))?;
    let native = file.metadata().map_err(error)?;
    let host = std::fs::metadata("/proc/self/ns/ipc").map_err(error)?;
    if (native.dev(), native.ino()) == (host.dev(), host.ino()) {
        return Ok(true);
    }
    std::thread::spawn(move || {
        rustix::thread::move_into_link_name_space(
            file.as_fd(),
            Some(rustix::thread::LinkNameSpaceType::InterProcessCommunication),
        )
    })
    .join()
    .map_err(|_| "IPC namespace validation thread panicked".to_owned())?
    .map_err(|e| format!("IPCNamespacePath={path} is not a usable private IPC namespace: {e}"))?;
    Ok(false)
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
    let host = std::fs::canonicalize("/dev/shm").map_err(error)?;
    let host_metadata = std::fs::metadata(&host).map_err(error)?;
    let metadata = std::fs::metadata(source).map_err(error)?;
    Ok(host.starts_with(source)
        || (metadata.dev(), metadata.ino()) == (host_metadata.dev(), host_metadata.ino()))
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

/// Create kernel-owned run resources; partial creation remains visible to scope cleanup.
/// The temporary thread exits after pinning IPC, so no app or helper owns its lifetime.
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
    .map_err(|e| format!("{}: {e}", shm.display()))?;
    let ipc = namespace(scope);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(NAMESPACE_MODE)
        .open(&ipc)
        .map_err(error)?;
    std::thread::spawn(move || -> Result<()> {
        // SAFETY: NEWIPC leaves the shared descriptor table and other thread state unchanged.
        unsafe { rustix::thread::unshare_unsafe(rustix::thread::UnshareFlags::NEWIPC) }
            .map_err(error)?;
        rustix::mount::mount_bind("/proc/thread-self/ns/ipc", &ipc)
            .map_err(|e| format!("{}: {e}", ipc.display()))
    })
    .join()
    .map_err(|_| "IPC namespace creation thread panicked".to_owned())?
}

/// Unmount owned resources after all units stop; busy mounts remain for explicit cleanup.
pub(crate) fn remove(scope: &Scope) -> Result<()> {
    for path in [namespace(scope), directory(scope)] {
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        match rustix::mount::unmount(&path, UnmountFlags::empty()) {
            Ok(()) | Err(rustix::io::Errno::INVAL) => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
        let removed = if metadata.is_dir() {
            std::fs::remove_dir(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}
