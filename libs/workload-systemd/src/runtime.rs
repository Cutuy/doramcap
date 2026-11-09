//! Observed setup and verified removal of the run's native runtime resources.
use crate::{PreparationInput, Scope, manager::Manager};
use orchestrator::{LocalExecutor, Step};
use std::rc::Rc;

pub(crate) fn register(executor: &mut LocalExecutor, manager: Rc<Manager>, scope: Scope) {
    let setup_manager = manager.clone();
    executor.register::<PrepareRuntime>(move || PrepareRuntime {
        manager: setup_manager.clone(),
        scope: scope.clone(),
    });
    executor.register::<RemoveRuntime>(move || RemoveRuntime {
        manager: manager.clone(),
    });
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
    type Output = Scope;

    async fn run(&mut self, input: PreparationInput) -> orchestrator::Result<Scope> {
        let run = input
            .run
            .ok_or_else(|| orchestrator::Error::message("run missing"))?;
        let plan = input
            .plan
            .ok_or_else(|| orchestrator::Error::message("plan missing"))?;
        self.manager
            .prepare_native_shm(&self.scope, &run, &plan)
            .await
            .map_err(orchestrator::Error::message)?;
        if workload_lifecycle::plan::validate_host_shm_access(run.host_shm_access)
            .map_err(orchestrator::Error::message)?
            == workload_lifecycle::launch::HostShmAccess::Blocked
        {
            crate::shm::create(&self.scope).map_err(orchestrator::Error::message)?;
        }
        Ok(self.scope.clone())
    }
}

/// Verify owned units have stopped, then remove runtime files and the run's shm mount.
/// Failures retain remaining resources for explicit cleanup; absent resources are accepted.
pub(crate) struct RemoveRuntime {
    manager: Rc<Manager>,
}
impl Step for RemoveRuntime {
    const ID: Option<&'static str> = Some("systemd.runtime.remove");
    const DISPLAY_NAME: Option<&'static str> = Some("Remove systemd runtime");
    type Input = Scope;
    type Output = ();

    async fn run(&mut self, scope: Scope) -> orchestrator::Result<()> {
        let remaining = self
            .manager
            .reset(&scope)
            .await
            .map_err(orchestrator::Error::message)?;
        if remaining.is_empty() {
            Ok(())
        } else {
            Err(orchestrator::Error::message(format!(
                "units remain after removal: {remaining:?}"
            )))
        }
    }
}
