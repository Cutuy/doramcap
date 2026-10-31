//! Exclusive experiment admission and strict, repeatable pre-run cleanup.
use crate::compose::{
    DockerAdapter, ResolvedComposeProject, Result, StopInput,
    adapter::{private_write, project_name, validate_name},
    error::invalid,
    overlay,
};
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
        let directory = PathBuf::from(format!("/tmp/doramcap-experiments-{}", unsafe {
            libc::geteuid()
        }));
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
    pub fn project(&self) -> String {
        project_name("experiment", &self.repository)
    }
    pub async fn previous(&self) -> Result<Option<ResolvedComposeProject>> {
        let path = self.directory.join("current.json");
        let bytes = match crate::evidence::read_regular(&path, 2 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(crate::compose::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let plan: ResolvedComposeProject = serde_json::from_slice(&bytes)?;
        validate_name(&plan.run)?;
        if !plan.name.starts_with("dc-") || plan.key != "experiment" || !plan.manifest.is_absolute()
        {
            return Err(invalid("invalid saved experiment identity"));
        }
        Ok(Some(plan))
    }
    pub async fn clear(&self) -> Result<()> {
        for name in ["current.json", "pending.json"] {
            match tokio::fs::remove_file(self.directory.join(name)).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        tokio::fs::File::open(&self.directory)
            .await?
            .sync_all()
            .await?;
        Ok(())
    }
    pub async fn save(&self, plan: &ResolvedComposeProject) -> Result<()> {
        let pending = self.directory.join("pending.json");
        private_write(&pending, &serde_json::to_vec(plan)?).await?;
        tokio::fs::File::open(&pending).await?.sync_all().await?;
        tokio::fs::rename(pending, self.directory.join("current.json")).await?;
        tokio::fs::File::open(&self.directory)
            .await?
            .sync_all()
            .await?;
        Ok(())
    }
}

pub struct ResetPreviousProject {
    pub(super) adapter: Rc<DockerAdapter>,
    pub(super) controller: Rc<Controller>,
    pub(super) stop_seconds: u64,
}
impl Step for ResetPreviousProject {
    const ID: Option<&'static str> = Some("compose.reset");
    const DISPLAY_NAME: Option<&'static str> = Some("Reset previous experiment");
    type Input = ();
    type Output = ();
    async fn run(&mut self, (): ()) -> orchestrator::Result<()> {
        self.reset()
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
impl ResetPreviousProject {
    async fn reset(&self) -> Result<()> {
        super::validate_budgets(1, self.stop_seconds)?;
        let previous = self.controller.previous().await?;
        let project = self.controller.project();
        let owner = previous
            .as_ref()
            .map_or(project.as_str(), |plan| plan.name.as_str());
        self.adapter
            .check_machine_owner(owner, previous.is_some())
            .await?;
        if let Some(plan) = previous {
            // Stop every writer before unmounting or deleting any upper data.
            self.adapter
                .stop_for_reset(&plan, self.stop_seconds)
                .await?;
            let artifacts = plan
                .manifest
                .parent()
                .ok_or_else(|| invalid("saved artifact directory missing"))?;
            overlay::discard(&artifacts.join("overlays")).await?;
            let cleanup = self
                .adapter
                .remove(StopInput {
                    plan,
                    services: vec![],
                    allow_volume_deletion: false,
                    stop_seconds: self.stop_seconds,
                })
                .await?;
            if !cleanup.verified || !cleanup.diagnostics.is_empty() {
                return Err(invalid(format!(
                    "previous experiment cleanup failed: {cleanup:?}"
                )));
            }
        }
        self.adapter.verify_project_empty(&project).await?;
        self.controller.clear().await
    }
}

#[cfg(test)]
mod tests;
