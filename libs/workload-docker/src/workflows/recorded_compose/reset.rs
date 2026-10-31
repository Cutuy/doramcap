//! Exclusive experiment admission and strict, repeatable pre-run cleanup.
use crate::compose::{DockerAdapter, Result, error::invalid, overlay};
use orchestrator::Step;
use std::{
    fs::File,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    rc::Rc,
};

pub(super) struct Controller {
    pub repository: PathBuf,
    directory: PathBuf,
    _lock: File,
}
impl Controller {
    pub fn acquire(repository: &Path) -> Result<Self> {
        if !repository.is_absolute() {
            return Err(invalid("repository root must be absolute"));
        }
        let repository = std::fs::canonicalize(repository)?;
        if !repository.is_dir() {
            return Err(invalid("repository root must be a directory"));
        }
        // Each host user shares one controller across repositories and Console workers.
        let directory = directory();
        Self::in_directory(repository, directory)
    }
    fn in_directory(repository: PathBuf, directory: PathBuf) -> Result<Self> {
        match std::fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        let metadata = std::fs::symlink_metadata(&directory)?;
        if !metadata.is_dir()
            || metadata.permissions().mode() & 0o077 != 0
            || metadata.uid() != unsafe { libc::geteuid() }
            || std::fs::canonicalize(&directory)? != directory
        {
            return Err(invalid(
                "experiment controller directory must be private and canonical",
            ));
        }
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join("controller.lock"))?;
        if !lock.metadata()?.is_file() {
            return Err(invalid("controller lock is not a regular file"));
        }
        lock.try_lock()
            .map_err(|e| invalid(format!("another experiment controller is active: {e}")))?;
        // In-flight Compose/rsync children keep admission until they exit if the launcher dies.
        if unsafe { libc::fcntl(lock.as_raw_fd(), libc::F_SETFD, 0) } == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(Self {
            repository,
            directory,
            _lock: lock,
        })
    }
    pub fn workspace(&self) -> PathBuf {
        self.directory.join("overlays")
    }
}

pub(super) fn directory() -> PathBuf {
    PathBuf::from(format!("/tmp/doramcap-experiments-{}", unsafe {
        libc::geteuid()
    }))
}

pub struct ResetExperiment {
    pub(super) adapter: Rc<DockerAdapter>,
    pub(super) controller: Rc<Controller>,
    pub(super) stop_seconds: u64,
}
impl Step for ResetExperiment {
    const ID: Option<&'static str> = Some("compose.reset");
    const DISPLAY_NAME: Option<&'static str> = Some("Reset experiment");
    type Input = ();
    type Output = ();
    async fn run(&mut self, (): ()) -> orchestrator::Result<()> {
        self.reset()
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
impl ResetExperiment {
    async fn reset(&self) -> Result<()> {
        super::validate_budgets(1, self.stop_seconds)?;
        let scope = self
            .controller
            .directory
            .to_str()
            .ok_or_else(|| invalid("controller path is not UTF-8"))?;
        // Stop and verify every writer before touching the fixed overlay workspace.
        self.adapter.quiesce_scope(scope, self.stop_seconds).await?;
        overlay::discard(&self.controller.workspace()).await?;
        // Empty containers/networks and an absent workspace are the success conditions.
        self.adapter.remove_scope(scope).await
    }
}

#[cfg(test)]
mod tests;
