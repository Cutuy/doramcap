//! Idempotent repository reset shared by native Build and Run.
use crate::process::HelperProcess;
use compose_apps::{compose::adapter::repository_scope, shared_dora::owner};
use docker_engine::Engine;
use std::{path::Path, time::Duration};
use tokio::time::Instant;
use workload_lifecycle::launch::RunPlan;

/// Settle prior writers, discard stale overlays and remove only owned resources under admission.
/// One stop budget bounds all phases; partial errors retain the checkpoint and never write back.
pub(crate) async fn reset_project(
    engine: &Engine,
    run: &RunPlan,
) -> std::result::Result<(), String> {
    let repository = Path::new(&run.repository_root);
    let deadline = Instant::now() + Duration::from_secs(run.stop_seconds);
    let helper =
        HelperProcess::recover_repository(repository, Duration::from_secs(run.stop_seconds))?;
    let helper_stopped = match &helper {
        Some(helper) => within(deadline, "stop previous helper", helper.quiesce()).await,
        None => Ok(()),
    };
    let scope = repository_scope(repository);
    let scope_text = scope
        .to_str()
        .ok_or("Holoscan controller scope must be UTF-8")?;
    let reservation = owner(repository);
    let resources = engine.resources(scope_text, &reservation);
    let containers_stopped = within(deadline, "stop previous containers", async {
        resources
            .quiesce(run.stop_seconds)
            .await
            .map_err(|error| error.to_string())
    })
    .await;
    helper_stopped?;
    let stopped = containers_stopped?;
    let last =
        compose_apps::system_apps::removal_owner(&stopped).map_err(|error| error.to_string())?;
    // Reset discards interrupted work only after all helper/container writers are verified stopped.
    within(deadline, "discard previous overlays", async {
        host_filesystem::overlay::discard(&scope.join("overlays"))
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    within(deadline, "remove previous containers", async {
        resources
            .remove(last.as_deref())
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    if let Some(helper) = helper {
        within(deadline, "remove previous helper", helper.remove()).await?;
    }
    Ok(())
}

async fn within<T>(
    deadline: Instant,
    operation: &str,
    future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| format!("{operation}: deadline exceeded"))?
}
