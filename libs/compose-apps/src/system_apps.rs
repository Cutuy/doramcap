//! Shared system-app startup, observation and shutdown sequences.
use crate::{
    compose::{error::invalid, *},
    experiment::{RegisterDoraGraph, SharedExperiment, StartDoraRuntime, StopDoraRuntime},
    preparation::{self, PreparedRun},
};
use orchestrator::{Input, LocalExecutor, Output};
use std::time::Duration;

// Approved polling exception: detect a stopped graph while containers remain running.
const DORA_GRAPH_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const SYSTEM_APPS_STABILITY_INTERVAL: Duration = Duration::from_millis(500);

/// Start shared memory, Dora, its graph and system services, then verify readiness and placement.
/// Returns graph lineage for extra-service startup; updates `last_launch` after each successful start.
/// Errors or cancellation leave partial resources owned by the caller for stop and removal.
pub async fn start(
    executor: &LocalExecutor,
    prepared: &Output<PreparedRun>,
    plan: &ResolvedComposeProject,
    overlays: Option<&Output<PreparedOverlays>>,
    last_launch: &mut Option<Output<ComposeProjectState>>,
    adapter: &DockerAdapter,
) -> orchestrator::Result<Output<()>> {
    // Keep the run namespace alive before Dora or application containers can join it.
    if plan.services.contains_key(preparation::SHM_SERVICE) {
        let mut input = Input::new(StartInput {
            plan: Some(plan.clone()),
            services: vec![preparation::SHM_SERVICE.into()],
            readiness_seconds: prepared.value().run.readiness_seconds,
        })
        .after(prepared);
        if let Some(overlays) = overlays {
            input = input.after(overlays);
        }
        *last_launch = Some(executor.run::<StartCompose>(input).await?);
    }
    // Start communication and register the graph before any application containers.
    let daemon = executor
        .run::<StartDoraRuntime>(Input::new(()).after(prepared))
        .await?;
    let graph = executor
        .run::<RegisterDoraGraph>(Input::new(()).after(&daemon))
        .await?;
    let system = &prepared.value().system_services;
    if !system.is_empty() {
        *last_launch = Some(
            start_services(
                executor,
                StartInput {
                    plan: Some(plan.clone()),
                    services: system.clone(),
                    readiness_seconds: prepared.value().run.readiness_seconds,
                },
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
                    recordings: prepared
                        .value()
                        .run
                        .recordings
                        .iter()
                        .map(|recording| workload_lifecycle::recording::Recording {
                            stream: recording.stream.clone().into(),
                            recording: recording.recording.clone().into(),
                        })
                        .collect(),
                    timeout_seconds: prepared.value().run.readiness_seconds,
                },
            )
            .await?;
        if !prepared.value().user_services.is_empty() {
            tokio::time::sleep(SYSTEM_APPS_STABILITY_INTERVAL).await;
            let mut state = executor
                .run::<InspectCompose>(plan.clone())
                .await?
                .into_value();
            select_services(&mut state, system)?;
            if !state
                .ready()
                .map_err(|error| orchestrator::Error::message(error.to_string()))?
            {
                return Err(orchestrator::Error::message("system apps lost readiness"));
            }
        }
    }
    // Verify every infrastructure container before permitting adopted workload startup.
    let inspected = executor.run::<InspectCompose>(plan.clone()).await?;
    let mut state = inspected.value().clone();
    let infrastructure = |name: &String| {
        name == preparation::SHM_SERVICE
            || prepared.value().system_services.contains(name)
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
    let mut pids = Vec::new();
    for container in &state.containers {
        if infrastructure(&container.service) {
            let inspected = adapter
                .engine
                .client()
                .inspect_container(&container.id, None)
                .await
                .map_err(|e| orchestrator::Error::message(e.to_string()))?;
            if inspected
                .host_config
                .and_then(|config| config.cgroup_parent)
                .as_deref()
                != prepared
                    .value()
                    .system_cgroup
                    .as_ref()
                    .map(|placement| placement.parent.as_str())
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
            pids.push(pid);
        }
    }
    if prepared.value().system_cgroup.is_some() {
        executor
            .run::<workload_lifecycle::cgroup::SystemAppsCgroups>(
                Input::new(workload_lifecycle::launch::SystemAppsCgroupsInput {
                    action: workload_lifecycle::cgroup::Action::Create.into(),
                    artifacts: prepared.value().artifacts.to_string_lossy().into_owned(),
                    readiness_seconds: prepared.value().run.readiness_seconds,
                    pids,
                })
                .after(prepared)
                .after(&inspected),
            )
            .await?;
    }
    Ok(graph)
}

/// Start selected services after the graph, overlays and previous successful launch.
/// Errors or cancellation may leave containers requiring caller-owned cleanup.
pub async fn start_services(
    executor: &LocalExecutor,
    input: StartInput,
    graph: &Output<()>,
    overlays: Option<&Output<PreparedOverlays>>,
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

/// Watch readiness of the explicit service list and the shared Dora runtime/graph.
/// Uses subscribed Docker changes and the existing graph check interval; returns on failure.
/// Cancellation stops observation and leaves resource ownership with the caller.
pub async fn watch(
    executor: &LocalExecutor,
    plan: &ResolvedComposeProject,
    services: &[String],
    resources: &SharedExperiment,
    mut changes: tokio::sync::watch::Receiver<()>,
) -> orchestrator::Result<()> {
    if let Some(service) = services
        .iter()
        .find(|service| !plan.services.contains_key(*service))
    {
        return Err(orchestrator::Error::message(format!(
            "unknown watched Compose service: {service}"
        )));
    }
    let containers = async {
        loop {
            let state = executor.run::<InspectCompose>(plan.clone()).await?;
            let mut state = state.into_value();
            select_services(&mut state, services)?;
            if !state
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

fn select_services(
    state: &mut ComposeProjectState,
    services: &[String],
) -> orchestrator::Result<()> {
    state
        .plan
        .as_mut()
        .ok_or_else(|| orchestrator::Error::message("Compose state plan missing"))?
        .services
        .retain(|name, _| services.contains(name));
    Ok(())
}

/// Result of stopping selected services.
pub struct QuiescedServices {
    /// Every service stopped with no failure other than forced termination.
    pub stopped: bool,
    /// How each stopped service ended.
    pub exits: Vec<ServiceExit>,
}

/// Quiesce selected services, collecting exit evidence and adapter diagnostics.
/// Empty selections succeed without invoking a step; other step failures return unconfirmed stop.
pub async fn quiesce_services(
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

/// Evidence from stopping system services and the Dora runtime.
pub struct StoppedSystemApps {
    /// Dora node shutdown succeeded and no system service required forced termination.
    pub clean: bool,
    /// Every Dora runtime service is confirmed stopped.
    pub runtime_stopped: bool,
    /// Runtime shutdown produced no diagnostics.
    pub runtime_exit_clean: bool,
    /// System service exits followed by runtime exits.
    pub exits: Vec<ServiceExit>,
}

/// Request system-node shutdown, quiesce system services including the SHM holder, then stop Dora.
/// Records diagnostics and returns separate cleanup/exit evidence even after a failed stage.
/// Cancellation can leave partial shutdown; the caller retains all resources.
pub async fn stop(
    executor: &LocalExecutor,
    plan: &ResolvedComposeProject,
    services: &[String],
    resources: &SharedExperiment,
    diagnostics: &Diagnostics,
) -> StoppedSystemApps {
    let systems_stopped = resources.lock().await.shutdown(services).await;
    if let Err(error) = &systems_stopped {
        diagnostics
            .borrow_mut()
            .push(invalid(error.to_string()).diagnostic("system shutdown"));
    }
    // The shared-memory holder has no inspection phase; stop it with the other system apps.
    let services: Vec<_> = services
        .iter()
        .cloned()
        .chain(
            plan.services
                .contains_key(preparation::SHM_SERVICE)
                .then(|| preparation::SHM_SERVICE.to_owned()),
        )
        .collect();
    let systems = quiesce_services(executor, plan, &services, diagnostics).await;
    let clean =
        systems_stopped.is_ok() && systems.stopped && !systems.exits.iter().any(|exit| exit.killed);
    let mut exits = systems.exits;
    let runtime_shutdown = executor.run::<StopDoraRuntime>(()).await;
    let runtime_stopped = runtime_shutdown
        .as_ref()
        .is_ok_and(|output| output.value().stopped);
    let runtime_exit_clean = runtime_shutdown
        .as_ref()
        .is_ok_and(|output| output.value().diagnostics.is_empty());
    match runtime_shutdown {
        Ok(stopped) => {
            let stopped = stopped.into_value();
            exits.extend(stopped.exits);
            diagnostics.borrow_mut().extend(stopped.diagnostics);
        }
        Err(error) => diagnostics
            .borrow_mut()
            .push(invalid(error.to_string()).diagnostic("Dora shutdown")),
    }
    StoppedSystemApps {
        clean,
        runtime_stopped,
        runtime_exit_clean,
        exits,
    }
}

/// Select the Dora experiment reservation to remove after all other owned resources.
/// The caller supplies only owned containers; duplicate owners or missing owner IDs fail.
/// This selects a stopped container's removal order and imposes no SHM inspection lifetime.
pub fn removal_owner<'a>(
    containers: impl IntoIterator<Item = &'a bollard::models::ContainerSummary>,
) -> Result<Option<&'a str>> {
    let mut owner = None;
    for container in containers {
        if container.labels.as_ref().is_some_and(|labels| {
            labels.contains_key(crate::compose::adapter::EXPERIMENT_OWNER_LABEL)
        }) {
            if owner.is_some() {
                return Err(invalid("multiple experiment IPC owners"));
            }
            owner = Some(
                container
                    .id
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| invalid("Engine omitted experiment owner ID"))?,
            );
        }
    }
    Ok(owner)
}

#[cfg(test)]
mod tests;
