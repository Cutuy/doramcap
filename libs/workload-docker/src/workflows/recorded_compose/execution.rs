//! Compose-specific user-service startup around the shared system-app sequence.
use super::*;
use compose_apps::{preparation, system_apps};
use orchestrator::{LocalExecutor, Output};

pub(super) async fn launch_apps(
    executor: &LocalExecutor,
    prepared: &Output<preparation::PreparedRun>,
    plan: &ResolvedComposeProject,
    overlays: Option<&Output<PreparedOverlays>>,
    last_launch: &mut Option<Output<ComposeProjectState>>,
    adapter: &DockerAdapter,
) -> orchestrator::Result<()> {
    let graph =
        system_apps::start(executor, prepared, plan, overlays, last_launch, adapter).await?;
    // Retain the last successful launch's producer identity across partial startup failure.
    let adopted = &prepared.value().user_services;
    if !adopted.is_empty() {
        *last_launch = Some(
            system_apps::start_services(
                executor,
                StartInput {
                    plan: Some(plan.clone()),
                    services: adopted.clone(),
                    readiness_seconds: prepared.value().run.readiness_seconds,
                },
                &graph,
                overlays,
                last_launch.as_ref(),
            )
            .await?,
        );
    }
    Ok(())
}

pub(super) fn record_failure(
    diagnostics: &Diagnostics,
    operation: &str,
    error: impl std::fmt::Display,
) {
    diagnostics
        .borrow_mut()
        .push(invalid(error.to_string()).diagnostic(operation));
}
