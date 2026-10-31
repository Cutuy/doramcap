//! Application startup, monitoring and cooperative cancellation for one prepared run.
use super::*;
use orchestrator::{LocalExecutor, Output};
use std::time::Duration;

/// Stop requests win ties; callers always perform teardown after this future returns.
pub(super) async fn until_stop(
    stop: impl Future<Output = ()>,
    event_failure: impl Future<Output = String>,
    execution: impl Future<Output = orchestrator::Result<()>>,
) -> Option<String> {
    tokio::select! {
        biased;
        _ = stop => None,
        result = execution => result.err().map(|error| error.to_string()),
        error = event_failure => Some(error),
    }
}

pub(super) async fn launch_apps(
    executor: &LocalExecutor,
    spec: &ComposeRun,
    prepared: &Output<preparation::PreparedRun>,
    plan: &ResolvedComposeProject,
    overlays: Option<&Output<overlay::OverlaySet>>,
    last_launch: &mut Option<Output<ComposeProjectState>>,
) -> orchestrator::Result<()> {
    // Start communication and register the graph before any application containers.
    let daemon = executor
        .run::<super::super::experiment::StartDoraRuntime>(Input::new(()).after(prepared))
        .await?;
    let graph = executor
        .run::<super::super::experiment::RegisterDoraGraph>(Input::new(()).after(&daemon))
        .await?;
    let start_input = |services| StartInput {
        plan: Some(plan.clone()),
        services,
        readiness_seconds: spec.readiness_seconds,
    };
    let system = &prepared.value().system_services;
    if !system.is_empty() {
        *last_launch = Some(
            start_services(
                executor,
                start_input(system.clone()),
                &graph,
                overlays,
                last_launch.as_ref(),
            )
            .await?,
        );
        // System recordings and health must stabilize before adopted apps can start.
        executor
            .run::<workload_lifecycle::recording::WaitForRecordings>(
                workload_lifecycle::recording::ReadinessInput {
                    recordings: spec.recordings.clone(),
                    timeout_seconds: spec.readiness_seconds,
                },
            )
            .await?;
        if !prepared.value().user_services.is_empty() {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut state = executor
                .run::<InspectCompose>(plan.clone())
                .await?
                .into_value();
            state
                .plan
                .as_mut()
                .ok_or_else(|| orchestrator::Error::message("Compose state plan missing"))?
                .services
                .retain(|name, _| system.contains(name));
            if !state
                .ready()
                .map_err(|error| orchestrator::Error::message(error.to_string()))?
            {
                return Err(orchestrator::Error::message("system apps lost readiness"));
            }
        }
    }
    // Retain the last successful launch's producer identity across partial startup failure.
    let adopted = &prepared.value().user_services;
    if !adopted.is_empty() {
        *last_launch = Some(
            start_services(
                executor,
                start_input(adopted.clone()),
                &graph,
                overlays,
                last_launch.as_ref(),
            )
            .await?,
        );
    }
    Ok(())
}

async fn start_services(
    executor: &LocalExecutor,
    input: StartInput,
    graph: &Output<()>,
    overlays: Option<&Output<overlay::OverlaySet>>,
    previous: Option<&Output<ComposeProjectState>>,
) -> orchestrator::Result<Output<ComposeProjectState>> {
    let mut input = Input::new(input).after(graph);
    if let Some(overlays) = overlays {
        input = input.after(overlays);
    }
    if let Some(previous) = previous {
        input = input.after(previous);
    }
    executor.run::<StartCompose>(input).await
}

pub(super) async fn monitor(
    executor: &LocalExecutor,
    plan: &ResolvedComposeProject,
    resources: &super::super::experiment::SharedExperiment,
) -> orchestrator::Result<()> {
    loop {
        let state = executor.run::<InspectCompose>(plan.clone()).await?;
        if !state
            .value()
            .ready()
            .map_err(|error| orchestrator::Error::message(error.to_string()))?
        {
            return Err(orchestrator::Error::message("experiment lost readiness"));
        }
        resources
            .lock()
            .await
            .inspect()
            .await
            .map_err(|error| orchestrator::Error::message(error.to_string()))?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub(super) async fn quiesce_services(
    executor: &LocalExecutor,
    plan: &ResolvedComposeProject,
    services: &[String],
    stop_seconds: u64,
    diagnostics: &Diagnostics,
) -> bool {
    if services.is_empty() {
        return true;
    }
    let Ok(stopped) = executor
        .run::<QuiesceCompose>(StopInput {
            plan: Some(plan.clone()),
            services: services.to_vec(),
            allow_volume_deletion: false,
            stop_seconds,
        })
        .await
    else {
        return false;
    };
    let stopped = stopped.into_value();
    let clean = stopped.stopped && stopped.diagnostics.is_empty();
    diagnostics.borrow_mut().extend(stopped.diagnostics);
    clean
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

#[cfg(test)]
mod tests;
