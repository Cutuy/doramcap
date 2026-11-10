//! Application startup, monitoring and cooperative cancellation for one prepared run.
use super::*;
use crate::compose::FORCED_STOP_OPERATION;
use orchestrator::{LocalExecutor, Output};
use std::time::Duration;

// Approved polling exception: detect a stopped graph while containers remain running.
const DORA_GRAPH_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const SYSTEM_APPS_STABILITY_INTERVAL: Duration = Duration::from_millis(500);

#[allow(clippy::too_many_arguments)]
pub(super) async fn launch_apps(
    executor: &LocalExecutor,
    spec: &ComposeRun,
    prepared: &Output<preparation::PreparedRun>,
    plan: &ResolvedComposeProject,
    overlays: Option<&Output<overlay::OverlaySet>>,
    last_launch: &mut Option<Output<ComposeProjectState>>,
    system_apps: &workload_systemd::cgroup::SystemAppSlice,
    adapter: &DockerAdapter,
) -> orchestrator::Result<()> {
    let mut input = Input::new(workload_systemd::SystemAppsCgroupsInput {
        action: workload_systemd::cgroup::Action::Create.into(),
        artifacts: prepared.value().artifacts.to_string_lossy().into_owned(),
        readiness_seconds: spec.readiness_seconds,
    })
    .after(prepared);
    if let Some(overlays) = overlays {
        input = input.after(overlays);
    }
    let cgroups = executor
        .run::<workload_systemd::cgroup::SystemAppsCgroups>(input)
        .await?;
    // Keep the run namespace alive before Dora or application containers can join it.
    if plan.services.contains_key(preparation::SHM_SERVICE) {
        let mut input = Input::new(StartInput {
            plan: Some(plan.clone()),
            services: vec![preparation::SHM_SERVICE.into()],
            readiness_seconds: spec.readiness_seconds,
        })
        .after(prepared)
        .after(&cgroups);
        if let Some(overlays) = overlays {
            input = input.after(overlays);
        }
        *last_launch = Some(executor.run::<StartCompose>(input).await?);
    }
    // Start communication and register the graph before any application containers.
    let daemon = executor
        .run::<super::super::experiment::StartDoraRuntime>(
            Input::new(()).after(prepared).after(&cgroups),
        )
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
            tokio::time::sleep(SYSTEM_APPS_STABILITY_INTERVAL).await;
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
    // Verify every infrastructure container before permitting adopted workload startup.
    let mut state = executor
        .run::<InspectCompose>(plan.clone())
        .await?
        .into_value();
    let infrastructure = |name: &String| {
        prepared.value().system_services.contains(name)
            || prepared.value().dora.services.contains_key(name)
    };
    state
        .plan
        .as_mut()
        .ok_or_else(|| orchestrator::Error::message("Compose state plan missing"))?
        .services
        .retain(|name, _| infrastructure(name));
    if !state
        .ready()
        .map_err(|e| orchestrator::Error::message(e.to_string()))?
    {
        return Err(orchestrator::Error::message(
            "system apps lost readiness before cgroup verification",
        ));
    }
    system_apps
        .identity()
        .map_err(orchestrator::Error::message)?;
    for container in &state.containers {
        if infrastructure(&container.service) {
            let inspected = adapter
                .docker
                .inspect_container(&container.id, None)
                .await
                .map_err(|e| orchestrator::Error::message(e.to_string()))?;
            if inspected
                .host_config
                .and_then(|config| config.cgroup_parent)
                .as_deref()
                != Some(system_apps.name().as_str())
            {
                return Err(orchestrator::Error::message(format!(
                    "{}: system-app cgroup assignment differs",
                    container.service
                )));
            }
            if plan.services[&container.service].completed && container.status == "exited" {
                continue;
            }
            let pid = u32::try_from(container.pid)
                .ok()
                .filter(|pid| *pid != 0)
                .ok_or_else(|| {
                    orchestrator::Error::message(format!(
                        "{}: system-app host PID unavailable",
                        container.service
                    ))
                })?;
            system_apps
                .verify_pid(pid)
                .map_err(orchestrator::Error::message)?;
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
    mut changes: tokio::sync::watch::Receiver<()>,
) -> orchestrator::Result<()> {
    let containers = async {
        loop {
            let state = executor.run::<InspectCompose>(plan.clone()).await?;
            if !state
                .value()
                .ready()
                .map_err(|error| orchestrator::Error::message(error.to_string()))?
            {
                return Err(orchestrator::Error::message("experiment lost readiness"));
            }
            changes.changed().await.map_err(|error| {
                orchestrator::Error::message(format!("Docker event notifications ended: {error}"))
            })?;
        }
    };
    let dora = async {
        loop {
            resources
                .lock()
                .await
                .inspect()
                .await
                .map_err(|error| orchestrator::Error::message(error.to_string()))?;
            tokio::time::sleep(DORA_GRAPH_CHECK_INTERVAL).await;
        }
    };
    tokio::select! {
        result = containers => result,
        result = dora => result,
    }
}

/// Result of stopping selected services.
pub(super) struct QuiescedServices {
    /// Every service stopped with no failure other than forced termination.
    pub stopped: bool,
    /// How each stopped service ended.
    pub exits: Vec<crate::compose::ServiceExit>,
}

pub(super) async fn quiesce_services(
    executor: &LocalExecutor,
    plan: &ResolvedComposeProject,
    services: &[String],
    diagnostics: &Diagnostics,
) -> QuiescedServices {
    let mut result = QuiescedServices {
        stopped: services.is_empty(),
        exits: Vec::new(),
    };
    if services.is_empty() {
        return result;
    }
    let Ok(stopped) = executor
        .run::<QuiesceCompose>(StopInput {
            plan: Some(plan.clone()),
            services: services.to_vec(),
            allow_volume_deletion: false,
        })
        .await
    else {
        return result;
    };
    let stopped = stopped.into_value();
    result.stopped = stopped.stopped
        && stopped
            .diagnostics
            .iter()
            .all(|d| d.operation == FORCED_STOP_OPERATION);
    result.exits = stopped.exits;
    diagnostics.borrow_mut().extend(stopped.diagnostics);
    result
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
