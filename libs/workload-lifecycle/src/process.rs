//! Persisted ownership of serial foreground helpers; backend resources stay backend-owned.
include!(concat!(env!("OUT_DIR"), "/doramcap.process.v1.rs"));
use crate::cgroup;
use prost::Message;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;

type Result<T> = std::result::Result<T, String>;
/// The sole current-helper record within the existing per-user repository controller directory.
pub const OWNERSHIP_FILE: &str = "holoscan-cli-process.pb";
const HELPER: &str = "holoscan-cli";

/// One attempt's helper leaf/scope; the caller retains admission and the actual Child.
/// Recovery uses the repository controller record, including failed launches without containers.
/// Dropping this handle preserves ownership; cancellation must settle/reap the Child then call quiesce.
#[derive(Clone)]
pub struct HelperProcess {
    ownership: HelperOwnership,
    directory: PathBuf,
    stop: Duration,
}
impl HelperProcess {
    /// Persist the backend-validated placement before creating a leaf or allowing spawn.
    /// Caller holds ordinary admission and serializes every command through final copies and removal.
    pub fn prepare_at(
        placement: CgroupPlacement,
        artifacts: &Path,
        repository: &Path,
        application_parent: String,
        stop: Duration,
    ) -> Result<Self> {
        let artifacts = std::fs::canonicalize(artifacts).map_err(error)?;
        let artifacts_text = artifacts
            .to_str()
            .ok_or("Holoscan artifacts identity must be UTF-8")?
            .to_owned();
        let repository = std::fs::canonicalize(repository).map_err(error)?;
        let repository_text = repository
            .to_str()
            .ok_or("Holoscan repository identity must be UTF-8")?
            .to_owned();
        let directory = repository_scope(&repository);
        let (helper_name, helper_path) = helper_location(&application_parent)?;
        let boot_id = boot_id()?;
        let description = format!(
            "Doramcap Holoscan CLI {} {}",
            boot_id,
            host_filesystem::files::digest(artifacts.as_os_str().as_encoded_bytes())
        );
        let mut helper = Self {
            ownership: HelperOwnership {
                repository_root: repository_text,
                artifacts_root: artifacts_text,
                placement: Some(placement),
                application_parent,
                helper_name,
                helper_path: helper_path.to_string_lossy().into_owned(),
                description,
                boot_id,
                ..Default::default()
            },
            directory,
            stop,
        };
        helper.validate()?;
        host_filesystem::files::ensure_private_directory(&helper.directory).map_err(error)?;
        helper.save(true)?;
        if !helper.systemd() {
            let path = Path::new(&helper.ownership.helper_path);
            std::fs::create_dir_all(path.parent().ok_or("helper parent missing")?)
                .map_err(error)?;
            // A pre-existing leaf has unknown ownership and must never be adopted.
            std::fs::create_dir(path).map_err(error)?;
            let metadata = std::fs::symlink_metadata(path).map_err(error)?;
            helper.ownership.cgroup_device = metadata.dev();
            helper.ownership.cgroup_inode = metadata.ino();
            helper.save(false)?;
        }
        Ok(helper)
    }

    /// Recover the repository's current helper while the caller holds ordinary admission.
    /// Only a missing record returns None; old attempt artifacts need not exist.
    /// Invalid/private-directory/ownership failures preserve the record and block another launch.
    pub fn recover_repository(repository: &Path, stop: Duration) -> Result<Option<Self>> {
        if !repository.is_absolute() {
            return Err("admitted repository identity must be absolute".into());
        }
        let directory = repository_scope(repository);
        host_filesystem::files::ensure_private_directory(&directory).map_err(error)?;
        let bytes = match std::fs::read(directory.join(OWNERSHIP_FILE)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                sync_directory(&directory)?;
                return Ok(None);
            }
            Err(e) => return Err(error(e)),
        };
        let helper = Self {
            ownership: HelperOwnership::decode(bytes.as_slice()).map_err(error)?,
            directory,
            stop,
        };
        helper.validate()?;
        if helper.repository_root() != repository {
            return Err("current helper belongs to another repository".into());
        }
        Ok(Some(helper))
    }

    fn artifacts(&self) -> &Path {
        Path::new(&self.ownership.artifacts_root)
    }

    /// Canonical repository identity recorded before side effects; the admitted caller filters recovery by it.
    pub fn repository_root(&self) -> &Path {
        Path::new(&self.ownership.repository_root)
    }

    /// Native Docker cgroup_parent for this application's containers, beside the helper leaf.
    pub fn cgroup_parent(&self) -> &str {
        &self.ownership.application_parent
    }

    /// Create the guarded foreground command; caller adds argv/cwd/env/stdio and retains/reaps Child.
    /// Spawn on the owner's long-lived runtime thread and preserve executable credentials.
    /// Systemd uses literal argv and inherited descriptors; no shell, pipe proxy or admission descriptor is added.
    pub fn command(&self, cli: &Path) -> Result<Command> {
        self.validate()?;
        if !self.current()? {
            return Err("current helper ownership record is missing".into());
        }
        let mut command = if self.systemd() {
            let mut command = Command::new("systemd-run");
            command
                .args([
                    "--system",
                    "--scope",
                    "--quiet",
                    "--no-ask-password",
                    "--expand-environment=no",
                ])
                .arg(format!("--slice={}", self.ownership.application_parent))
                .arg(format!("--unit={}", self.ownership.helper_name))
                .arg(format!("--description={}", self.ownership.description))
                .arg("--")
                .arg(cli);
            command
        } else {
            Command::new(cli)
        };
        let procs = if self.systemd() {
            None
        } else {
            self.verify_leaf()?;
            Some(
                OpenOptions::new()
                    .write(true)
                    .open(Path::new(&self.ownership.helper_path).join("cgroup.procs"))
                    .map_err(error)?,
            )
        };
        host_filesystem::files::guard_parent(&mut command, procs);
        Ok(command)
    }

    /// Stop only this owned helper tree and verify it has no writers; containers are unaffected.
    /// The caller first kills/reaps a bootstrap Child that may still be registering its scope.
    /// Failed/uncertain ownership is retained; repeated absence succeeds. Bound by the run stop budget.
    pub async fn quiesce(&self) -> Result<()> {
        self.validate()?;
        self.current()?;
        let result = tokio::time::timeout(self.stop, async {
            let path = Path::new(&self.ownership.helper_path);
            if self.systemd() {
                let manager = cgroup::slice::connect().await?;
                cgroup::scope::stop(
                    &manager,
                    &self.ownership.helper_name,
                    &self.ownership.application_parent,
                    path,
                    &self.ownership.description,
                )
                .await?;
                if path.try_exists().map_err(error)? {
                    return Err(format!(
                        "{}: helper scope remains after stop",
                        path.display()
                    ));
                }
            } else if path.try_exists().map_err(error)? {
                self.verify_leaf()?;
                let (_watcher, mut changes) =
                    crate::files::watch_changes([path.join("cgroup.events")]).map_err(error)?;
                let mut kill = OpenOptions::new()
                    .write(true)
                    .open(path.join("cgroup.kill"))
                    .map_err(error)?;
                self.verify_leaf()?;
                kill.write_all(b"1").map_err(error)?;
                loop {
                    self.verify_leaf()?;
                    if !helper_populated(path)? {
                        break;
                    }
                    crate::files::wait_for_change(&mut changes)
                        .await
                        .map_err(error)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| format!("host helper cleanup exceeded {:?}", self.stop))?;
        if result.is_err()
            && !self.systemd()
            && !Path::new(&self.ownership.helper_path)
                .try_exists()
                .map_err(error)?
        {
            return Ok(());
        }
        result
    }

    /// Remove the stopped helper leaf; backend parents and the current record remain owned.
    /// Call `clear` only after backend resources and parents have also been removed.
    pub async fn remove_leaf(&self) -> Result<()> {
        self.quiesce().await?;
        if !self.systemd()
            && Path::new(&self.ownership.helper_path)
                .try_exists()
                .map_err(error)?
        {
            self.verify_leaf()?;
            cgroup::verify_empty(Path::new(&self.ownership.helper_path))?;
            std::fs::remove_dir(&self.ownership.helper_path).map_err(error)?;
        }
        Ok(())
    }

    /// Persisted backend placement for parent cleanup after helper and workload writers stop.
    pub fn placement(&self) -> &CgroupPlacement {
        self.ownership
            .placement
            .as_ref()
            .expect("validated placement")
    }

    fn systemd(&self) -> bool {
        self.ownership.application_parent.ends_with(".slice")
    }
    fn validate(&self) -> Result<()> {
        let placement = self
            .ownership
            .placement
            .as_ref()
            .ok_or("helper placement missing")?;
        let expected_path = parent_path(&placement.parent)?;
        if expected_path != Path::new(&placement.path) {
            return Err("helper placement host path differs".into());
        }
        if !self.repository_root().is_absolute()
            || !self.artifacts().is_absolute()
            || repository_scope(self.repository_root()) != self.directory
        {
            return Err("helper repository/artifact/controller identity differs".into());
        }
        let application = parent_path(&self.ownership.application_parent)?;
        let root = expected_path
            .parent()
            .ok_or("helper placement root missing")?;
        let relationship = if let Some(run) = placement.parent.strip_suffix("-system.slice") {
            ["user", "system"].into_iter().any(|role| {
                self.ownership
                    .application_parent
                    .strip_prefix(&format!("{run}-{role}-"))
                    .and_then(|app| app.strip_suffix(".slice"))
                    .is_some_and(|app| !app.is_empty() && !app.contains(['/', '.']))
            })
        } else if let Some(run) = placement.parent.strip_suffix("/system") {
            ["user", "system"].into_iter().any(|role| {
                self.ownership
                    .application_parent
                    .strip_prefix(&format!("{run}/{role}/"))
                    .is_some_and(|app| !app.is_empty() && !app.contains(['/', '.']))
            })
        } else {
            false
        };
        let contained = root != Path::new(cgroup::CGROUP_ROOT)
            && application != root
            && application.starts_with(root)
            && relationship;
        let (name, path) = helper_location(&self.ownership.application_parent)?;
        let expected_description = format!(
            "Doramcap Holoscan CLI {} {}",
            self.ownership.boot_id,
            host_filesystem::files::digest(self.artifacts().as_os_str().as_encoded_bytes())
        );
        if !contained
            || name != self.ownership.helper_name
            || path != Path::new(&self.ownership.helper_path)
            || self.ownership.description != expected_description
            || self.ownership.boot_id.is_empty()
        {
            return Err("helper ownership location or description differs".into());
        }
        Ok(())
    }
    fn verify_leaf(&self) -> Result<()> {
        if self.ownership.cgroup_inode == 0 || self.ownership.boot_id != boot_id()? {
            return Err(
                "helper cgroup ownership is incomplete or from another boot; preserve resources"
                    .into(),
            );
        }
        let metadata = std::fs::symlink_metadata(&self.ownership.helper_path).map_err(error)?;
        if !metadata.is_dir()
            || metadata.dev() != self.ownership.cgroup_device
            || metadata.ino() != self.ownership.cgroup_inode
        {
            return Err("helper cgroup identity changed; refusing signals".into());
        }
        Ok(())
    }
    // Admission serializes record writers. Refuse a newer current record before acting or clearing.
    fn current(&self) -> Result<bool> {
        match std::fs::read(self.directory.join(OWNERSHIP_FILE)) {
            Ok(bytes) if bytes == self.ownership.encode_to_vec() => Ok(true),
            Ok(_) => Err("current helper ownership changed; preserve newer resources".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(error(e)),
        }
    }
    /// Acknowledge the exact current record only after helper and backend cleanup succeeded.
    pub fn clear(&self) -> Result<()> {
        self.clear_with_sync(sync_directory)
    }
    fn clear_with_sync(&self, sync: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
        if self.current()? {
            std::fs::remove_file(self.directory.join(OWNERSHIP_FILE)).map_err(error)?;
        }
        // An earlier unlink may have succeeded before its sync failed; acknowledge absence durably on retry.
        sync(&self.directory)
    }
    fn save(&self, initial: bool) -> Result<()> {
        let destination = self.directory.join(OWNERSHIP_FILE);
        let path = if initial {
            destination.clone()
        } else {
            self.directory.join(format!("{OWNERSHIP_FILE}.next"))
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(error)?;
        let result = (|| {
            file.write_all(&self.ownership.encode_to_vec())
                .map_err(error)?;
            file.sync_all().map_err(error)?;
            if !initial {
                std::fs::rename(&path, &destination).map_err(error)?;
            }
            File::open(&self.directory)
                .map_err(error)?
                .sync_all()
                .map_err(error)
        })();
        if result.is_err() && !initial {
            let _ = std::fs::remove_file(path);
        }
        result
    }
}

// One population observation per iteration; unknown/read failures cannot establish emptiness.
fn helper_populated(path: &Path) -> Result<bool> {
    let events = std::fs::read_to_string(path.join("cgroup.events")).map_err(error)?;
    if events.lines().any(|line| line == "populated 0") {
        Ok(false)
    } else if events.lines().any(|line| line == "populated 1") {
        Ok(true)
    } else {
        Err("helper cgroup population is unknown".into())
    }
}
fn parent_path(parent: &str) -> Result<PathBuf> {
    if parent.ends_with(".slice") {
        Ok(cgroup::slice::path(parent))
    } else {
        cgroup::host_path(parent)
    }
}
fn helper_location(parent: &str) -> Result<(String, PathBuf)> {
    let name = if let Some(stem) = parent.strip_suffix(".slice") {
        format!("{stem}-{HELPER}.scope")
    } else {
        HELPER.into()
    };
    Ok((name.clone(), parent_path(parent)?.join(name)))
}
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path).map_err(error)?.sync_all().map_err(error)
}
fn boot_id() -> Result<String> {
    Ok(std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(error)?
        .trim()
        .into())
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Existing per-user repository controller identity, shared across admitted backend runs.
pub fn repository_scope(repository: &Path) -> PathBuf {
    PathBuf::from(format!(
        "/tmp/doramcap-experiments-{}-dc-{}-experiment",
        unsafe { libc::geteuid() },
        &host_filesystem::files::digest(repository.as_os_str().as_encoded_bytes())[..16]
    ))
}
impl host_filesystem::files::CommandScope for HelperProcess {
    fn command(&self, program: &Path) -> std::io::Result<Command> {
        HelperProcess::command(self, program).map_err(std::io::Error::other)
    }
    fn quiesce(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + '_>> {
        Box::pin(async {
            HelperProcess::quiesce(self)
                .await
                .map_err(std::io::Error::other)
        })
    }
}

#[cfg(test)]
mod tests;
