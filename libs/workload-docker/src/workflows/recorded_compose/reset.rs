//! Repository-scoped admission and strict, repeatable pre-run cleanup.
use crate::compose::{DockerAdapter, Result, error::invalid};
use docker_engine::overlay;
use orchestrator::Step;
use std::{
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    rc::Rc,
};
use workload_lifecycle::admission::Admission;

pub(super) struct Controller {
    pub repository: PathBuf,
    directory: PathBuf,
    _admission: Admission,
}
impl Controller {
    pub fn acquire(repository: &Path) -> Result<Self> {
        let admission = Admission::acquire(repository).map_err(|e| invalid(e.to_string()))?;
        // All callers for this canonical repository share admission and reset scope.
        let directory = directory(admission.repository());
        Self::in_directory(admission, directory)
    }
    fn in_directory(admission: Admission, directory: PathBuf) -> Result<Self> {
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
        // In-flight Compose/rsync children keep admission until they exit if the launcher dies.
        admission.keep_across_exec()?;
        Ok(Self {
            repository: admission.repository().to_owned(),
            directory,
            _admission: admission,
        })
    }
    /// Fixed temporary OverlayFS root, supplied automatically by the runner when overlays are enabled.
    pub fn workspace(&self) -> PathBuf {
        self.directory.join("overlays")
    }
}

pub(super) fn directory(repository: &Path) -> PathBuf {
    crate::compose::adapter::repository_scope(repository)
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
        let owner = super::super::shared_dora::owner(&self.controller.repository);
        // Stop and verify this repository's writers before touching its overlay workspace.
        let resources = self.adapter.engine.resources(scope, &owner);
        let stopped = resources.quiesce(self.stop_seconds).await?;
        let mut last = None;
        for container in stopped {
            if container.labels.is_some_and(|labels| {
                labels.contains_key(crate::compose::adapter::EXPERIMENT_OWNER_LABEL)
            }) && last.replace(container.id.unwrap()).is_some()
            {
                return Err(invalid("multiple experiment IPC owners"));
            }
        }
        overlay::discard(&self.controller.workspace()).await?;
        // Empty containers/networks and an absent workspace are the success conditions.
        resources.remove(last.as_deref()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
