//! Owned host CLI containment; Docker container ownership remains with Engine.
use crate::CliProcessOwnership;
use compose_apps::cgroup::{
    CgroupPlacement, DockerCgroup, Role, placement_for_run, service_parent,
};
use docker_engine::Engine;
use prost::Message;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;
use workload_lifecycle::{cgroup, launch::RunPlan};

type Result<T> = std::result::Result<T, String>;
/// The sole current-helper record within the existing per-user repository controller directory.
pub const OWNERSHIP_FILE: &str = "holoscan-cli-process.pb";
const HELPER: &str = "holoscan-cli";

/// One attempt's helper leaf/scope; the caller retains admission and the actual Child.
/// Recovery uses the repository controller record, including failed launches without containers.
/// Dropping this handle preserves ownership; cancellation must settle/reap the Child then call quiesce.
pub struct HelperProcess {
    ownership: CliProcessOwnership,
    directory: PathBuf,
    stop: Duration,
}
impl HelperProcess {
    /// Resolve placement and persist ownership before creating the cgroupfs leaf or allowing spawn.
    /// Use a fixed/generated component; hold ordinary admission, with fresh artifacts and no unfinished record.
    /// Partial failures retain the record; recover it before another launch.
    pub async fn prepare(
        engine: &Engine,
        run: &RunPlan,
        role: Role,
        component: &str,
    ) -> Result<Self> {
        let placement = placement_for_run(engine, run).await.map_err(error)?;
        Self::prepare_at(
            placement,
            Path::new(&run.artifacts),
            Path::new(&run.repository_root),
            role,
            component,
            Duration::from_secs(run.stop_seconds),
        )
    }

    fn prepare_at(
        placement: CgroupPlacement,
        artifacts: &Path,
        repository: &Path,
        role: Role,
        component: &str,
        stop: Duration,
    ) -> Result<Self> {
        if component.is_empty()
            || !component
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(
                "helper component must be a single alphanumeric, dash or underscore component"
                    .into(),
            );
        }
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
        let directory = compose_apps::compose::adapter::repository_scope(&repository);
        let application_parent = service_parent(&placement, role, component)?;
        let (helper_name, helper_path) = helper_location(&application_parent)?;
        let boot_id = boot_id()?;
        let description = format!(
            "Doramcap Holoscan CLI {} {}",
            boot_id,
            host_filesystem::files::digest(artifacts.as_os_str().as_encoded_bytes())
        );
        let mut helper = Self {
            ownership: CliProcessOwnership {
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
        let directory = compose_apps::compose::adapter::repository_scope(repository);
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
            ownership: CliProcessOwnership::decode(bytes.as_slice()).map_err(error)?,
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
        if !cli.is_absolute() {
            return Err("helper CLI must be absolute".into());
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
                    workload_lifecycle::files::watch_changes([path.join("cgroup.events")])
                        .map_err(error)?;
                let mut kill = OpenOptions::new()
                    .write(true)
                    .open(path.join("cgroup.kill"))
                    .map_err(error)?;
                self.verify_leaf()?;
                kill.write_all(b"1").map_err(error)?;
                loop {
                    self.verify_leaf()?;
                    if cgroup::verify_empty(path).is_ok() {
                        break;
                    }
                    if !helper_populated(path)? {
                        break;
                    }
                    workload_lifecycle::files::wait_for_change(&mut changes)
                        .await
                        .map_err(error)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| format!("Holoscan helper cleanup exceeded {:?}", self.stop))?;
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

    /// Remove the stopped helper and empty run/repository parents after Engine-owned containers are removed.
    /// Never captures files or applies write-back; live ancestors and uncertain identities are preserved.
    /// Clears the exact current-helper record only after full cleanup succeeds; failed cleanup retains it.
    pub async fn remove(&self) -> Result<()> {
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
        DockerCgroup::group(
            self.ownership
                .placement
                .clone()
                .ok_or("helper placement missing")?,
            self.stop,
        )
        .remove()
        .await?;
        self.clear()?;
        Ok(())
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
            || compose_apps::compose::adapter::repository_scope(self.repository_root())
                != self.directory
        {
            return Err("helper repository/artifact/controller identity differs".into());
        }
        // Validate the application parent against the generated run and one app component.
        let mut matches = false;
        for role in [Role::System, Role::User] {
            let prefix = service_parent(placement, role, "")?;
            let prefix = prefix.strip_suffix(".slice").unwrap_or(&prefix);
            let parent = &self.ownership.application_parent;
            let parent = parent.strip_suffix(".slice").unwrap_or(parent);
            matches |= parent
                .strip_prefix(prefix)
                .is_some_and(|app| !app.is_empty() && !app.contains('/') && !app.contains('.'));
        }
        let (name, path) = helper_location(&self.ownership.application_parent)?;
        let expected_description = format!(
            "Doramcap Holoscan CLI {} {}",
            self.ownership.boot_id,
            host_filesystem::files::digest(self.artifacts().as_os_str().as_encoded_bytes())
        );
        if !matches
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
    fn clear(&self) -> Result<()> {
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

// A later observation may already be empty after verify_empty saw live descendants.
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

#[cfg(test)]
mod tests;
