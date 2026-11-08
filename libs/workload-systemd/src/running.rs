//! One run's units behind the shared run sequence's handle.
use crate::{
    Result, Scope, SystemdPlan,
    manager::{Manager, Unit},
    runner::UnitExit,
};
use orchestrator::LocalExecutor;
use std::{
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    rc::Rc,
    time::Duration,
};
use workload_lifecycle::{
    launch::{CleanupRecord, RemovalPermit, StopRecord},
    recording::{ReadinessInput, Recording, WaitForRecordings},
};

/// Sticky and world-writable, like the host shared-memory directory.
const SHARED_DIRECTORY_MODE: u32 = 0o1777;

/// Create a fresh shared directory and set permissions on its no-follow handle.
fn create_shm(path: &Path) -> Result<()> {
    std::fs::DirBuilder::new()
        .mode(SHARED_DIRECTORY_MODE)
        .create(path)
        .map_err(crate::error)?;
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::DIRECTORY).bits() as i32)
        .open(path)
        .map_err(crate::error)?;
    directory
        .set_permissions(std::fs::Permissions::from_mode(SHARED_DIRECTORY_MODE))
        .map_err(crate::error)
}

pub(crate) struct SystemdRunning {
    pub manager: Rc<Manager>,
    pub executor: Rc<LocalExecutor>,
    pub scope: Scope,
    pub run: workload_lifecycle::launch::RunPlan,
    pub plan: SystemdPlan,
    pub recordings: Vec<Recording>,
    pub ready: Option<tokio::sync::oneshot::Sender<()>>,
    /// Attempted units in fixed startup phases: Dora, system apps, user apps.
    pub phases: Vec<Vec<Unit>>,
    pub dora: Option<crate::dora::Dora>,
    pub diagnostics: Vec<String>,
}

/// Unit results and failures handed back by removal.
pub(crate) struct Removed {
    pub units: Vec<UnitExit>,
    pub diagnostics: Vec<String>,
}

impl SystemdRunning {
    /// The run directory, with selected driver-owned host entries mounted over it.
    fn shm_binds(&self) -> Vec<crate::Bind> {
        let shared = crate::Bind {
            source: self.scope.shm.to_string_lossy().into_owned(),
            target: crate::SHM_ROOT.into(),
            read_only: false,
        };
        let host = self.run.host_shm.iter().map(|name| {
            let path = format!("{}/{name}", crate::SHM_ROOT);
            crate::Bind {
                source: path.clone(),
                target: path,
                read_only: false,
            }
        });
        std::iter::once(shared).chain(host).collect()
    }
}

impl workload_lifecycle::run::Running for SystemdRunning {
    type Removed = Removed;

    async fn start(&mut self) -> std::result::Result<(), String> {
        let readiness = Duration::from_secs(self.run.readiness_seconds);
        create_shm(&self.scope.shm)?;
        let shm = self.shm_binds();
        use workload_lifecycle::plan::Phase;
        for role in [Phase::Dora, Phase::System, Phase::User] {
            let mut phase = crate::phase(&self.run, &self.plan, role);
            if role != Phase::Dora {
                crate::dora::attach(&self.run, &mut phase)?;
            }
            for program in &mut phase.programs {
                program.binds.extend(shm.iter().cloned());
            }
            let runtime = if role == Phase::Dora {
                None
            } else {
                self.run.dora_runtime.as_ref()
            };
            self.phases.push(Vec::new());
            let start = async {
                let names = self
                    .manager
                    .adopt(
                        &self.scope,
                        &phase,
                        &self.plan.binaries,
                        self.run.stop_seconds,
                        runtime,
                        &shm,
                    )
                    .await?;
                // Keep every attempted unit before submitting any cancellable start.
                let units = self.phases.last_mut().unwrap();
                for program in &phase.programs {
                    units.push(self.manager.watch(&self.scope.unit(&program.id)).await?);
                }
                for name in &names {
                    units.push(self.manager.watch(name).await?);
                }
                let mut jobs = Vec::new();
                for program in &phase.programs {
                    jobs.push(
                        self.manager
                            .start_program(
                                &self.scope,
                                &self.run.id,
                                program,
                                self.run.stop_seconds,
                            )
                            .await?,
                    );
                }
                for name in &names {
                    jobs.push(self.manager.start_adopted(name).await?);
                }
                futures_util::future::try_join_all(
                    jobs.into_iter().map(|job| self.manager.wait_started(job)),
                )
                .await?;
                Ok::<_, String>(())
            };
            tokio::time::timeout(readiness, start)
                .await
                .map_err(|_| format!("apps not active within {readiness:?}"))??;
            if role == Phase::Dora && !self.run.dora_apps.is_empty() {
                self.dora = Some(
                    crate::dora::Dora::from_units(
                        self.phases.last().unwrap(),
                        &mut self.run,
                        &self.plan,
                    )
                    .await?,
                );
                let dora = self.dora.as_mut().unwrap();
                tokio::time::timeout(readiness, async {
                    dora.await_daemon(&self.manager, &self.scope, &self.run)
                        .await?;
                    dora.register(&self.manager, &self.scope, &mut self.run)
                        .await
                })
                .await
                .map_err(|_| "Dora readiness deadline exceeded".to_string())??;
            }
            if role == Phase::System {
                self.executor
                    .run::<WaitForRecordings>(ReadinessInput {
                        recordings: self.recordings.clone(),
                        timeout_seconds: self.run.readiness_seconds,
                    })
                    .await
                    .map_err(crate::error)?;
            }
        }
        Ok(())
    }

    fn ready(&mut self) {
        if let Some(ready) = self.ready.take() {
            let _ = ready.send(());
        }
    }

    /// A failed unit is recorded at stop; only a stop request ends a systemd run.
    async fn ended(&mut self) -> String {
        std::future::pending().await
    }

    async fn stop(&mut self) -> StopRecord {
        let mut stopped = true;
        for (index, phase) in self.phases.iter_mut().enumerate().rev() {
            // Dataflows stop before the coordinator and daemon they run on.
            if index == 0
                && let Some(dora) = &mut self.dora
                && let Err(error) = dora.down(&self.manager, &self.scope, &self.run).await
            {
                self.diagnostics.push(format!("Dora down: {error}"));
            }
            let results = futures_util::future::join_all(
                phase
                    .iter_mut()
                    .map(|unit| self.manager.stop_and_wait(unit)),
            )
            .await;
            for result in results {
                if let Err(error) = result {
                    stopped = false;
                    self.diagnostics.push(error);
                }
            }
        }
        StopRecord {
            writers_stopped: stopped,
            evidence_durable: true,
        }
    }

    async fn remove(mut self, permit: &RemovalPermit) -> (CleanupRecord, Removed) {
        let mut units = Vec::new();
        for unit in self.phases.iter().flatten() {
            units.push(UnitExit::read(unit).await);
            if permit.remove {
                self.manager.release(&unit.name).await;
            }
        }
        let verified = if permit.remove {
            match self.reset().await {
                Ok(()) => true,
                Err(error) => {
                    self.diagnostics.push(error);
                    false
                }
            }
        } else {
            self.diagnostics
                .push("preserved: writers did not stop".into());
            false
        };
        (
            CleanupRecord { verified },
            Removed {
                units,
                diagnostics: self.diagnostics,
            },
        )
    }
}

impl SystemdRunning {
    async fn reset(&self) -> Result<()> {
        let remaining = self.manager.reset(&self.scope).await?;
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(format!("units remain after removal: {remaining:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_memory_creation_preserves_existing_directories_and_symlinks() {
        // Arrange
        let root = std::env::temp_dir().join(format!("systemd-shm-create-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let existing = root.join("existing");
        std::fs::create_dir(&existing).unwrap();
        std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o750)).unwrap();
        std::fs::write(existing.join("retained"), "keep").unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&existing, &link).unwrap();
        // Act / Assert
        assert!(create_shm(&existing).is_err());
        assert!(create_shm(&link).is_err());
        assert_eq!(
            std::fs::metadata(&existing).unwrap().permissions().mode() & 0o7777,
            0o750
        );
        assert_eq!(
            std::fs::read_to_string(existing.join("retained")).unwrap(),
            "keep"
        );
        // Act / Assert: a newly owned directory gets the shared-memory permissions.
        let fresh = root.join("fresh");
        create_shm(&fresh).unwrap();
        assert_eq!(
            std::fs::metadata(&fresh).unwrap().permissions().mode() & 0o7777,
            SHARED_DIRECTORY_MODE
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
