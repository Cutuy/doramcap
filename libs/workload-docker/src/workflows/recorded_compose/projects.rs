//! Cooperative project-group launch, observation and best-effort shutdown steps.
use super::super::{
    dora,
    system_apps::{ReadinessInput, Recording, WaitForRecordings},
};
use crate::compose::{error::invalid, *};
use futures_util::future::join_all;
use orchestrator::{Input, LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use std::{rc::Weak, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchInput {
    pub projects: Vec<ResolvedComposeProject>,
    pub recordings: Vec<Recording>,
    pub readiness_seconds: u64,
}

/// Launch independent Compose projects, then verify their output readiness.
/// All starts are awaited even after a sibling fails. Cancellation drops pending
/// CLI futures; the caller must retain every attempted plan for Docker cleanup.
/// Uses the same executor for child events and dependency/parent lineage.
pub struct LaunchProjects {
    pub executor: Weak<LocalExecutor>,
}
impl Step for LaunchProjects {
    const ID: Option<&'static str> = Some("compose.launch");
    const DISPLAY_NAME: Option<&'static str> = Some("Start project");
    type Input = LaunchInput;
    type Output = ();
    async fn run(&mut self, input: LaunchInput) -> orchestrator::Result<()> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("project executor dropped"))?;
        let started = join_all(input.projects.into_iter().map(|plan| {
            executor.run::<StartCompose>(StartInput {
                services: vec![],
                plan: Some(plan),
                readiness_seconds: input.readiness_seconds,
            })
        }))
        .await;
        let mut readiness = orchestrator::Input::new(ReadinessInput {
            recordings: input.recordings,
            timeout_seconds: input.readiness_seconds,
        });
        for result in started {
            readiness = readiness.after(&result?);
        }
        executor.run::<WaitForRecordings>(readiness).await?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchFlowsInput {
    pub flows: Vec<dora::Flow>,
    pub readiness_seconds: u64,
}
/// Launch independent Dora flows after their Compose infrastructure is ready.
/// Await all siblings before propagating failure. On cancellation or partial
/// failure, shutdown can rediscover each attempted flow through its pinned plan.
pub struct LaunchDoraFlows {
    pub executor: Weak<LocalExecutor>,
}
impl Step for LaunchDoraFlows {
    type Input = LaunchFlowsInput;
    type Output = Vec<dora::Flow>;
    async fn run(&mut self, input: LaunchFlowsInput) -> orchestrator::Result<Vec<dora::Flow>> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("project executor dropped"))?;
        join_all(input.flows.into_iter().map(|flow| {
            executor.run::<dora::StartDoraFlow>(dora::StartInput {
                flow,
                timeout_seconds: input.readiness_seconds,
            })
        }))
        .await
        .into_iter()
        .map(|result| result.map(|started| started.into_value()))
        .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObserveInput {
    pub projects: Vec<ResolvedComposeProject>,
    pub flows: Vec<dora::Flow>,
}

/// Verify Compose replicas and Dora nodes concurrently. Snapshot synchronization
/// is a startup condition; runtime inspection checks process and HTTP health.
pub struct ObserveProjects {
    pub executor: Weak<LocalExecutor>,
}
impl Step for ObserveProjects {
    const ID: Option<&'static str> = Some("compose.observe");
    const DISPLAY_NAME: Option<&'static str> = Some("Observe projects");
    type Input = ObserveInput;
    type Output = ();
    async fn run(&mut self, input: ObserveInput) -> orchestrator::Result<()> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("project executor dropped"))?;
        let (projects, flows) = tokio::join!(
            join_all(input.projects.into_iter().map(|plan| async {
                let name = plan.key.clone();
                let state = executor.run::<InspectCompose>(plan).await?;
                if !state.value().ready().map_err(step_error)? {
                    return Err(orchestrator::Error::message(format!(
                        "{name} lost readiness or containers"
                    )));
                }
                Ok(())
            })),
            join_all(
                input
                    .flows
                    .into_iter()
                    .map(|flow| executor.run::<dora::InspectDoraFlow>(flow))
            ),
        );
        for result in projects {
            result?;
        }
        for result in flows {
            result?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuiesceInput {
    /// Launch groups in startup order, including each possibly partial launch.
    pub groups: Vec<Vec<ResolvedComposeProject>>,
    pub flows: Vec<dora::Flow>,
    pub stop_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuiescedProject {
    pub plan: ResolvedComposeProject,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuiescedProjects {
    pub projects: Vec<QuiescedProject>,
    pub writers_stopped: bool,
    pub dora_finalized: bool,
}

/// Stop Dora roles, then quiesce project groups in reverse startup order.
/// Siblings run concurrently and all outcomes are retained. Partial failure is
/// returned as evidence so the caller can still finalize and reconcile resources.
pub struct QuiesceProjects {
    pub executor: Weak<LocalExecutor>,
    pub diagnostics: Diagnostics,
}
impl Step for QuiesceProjects {
    const ID: Option<&'static str> = Some("compose.quiesce");
    const DISPLAY_NAME: Option<&'static str> = Some("Stop project");
    type Input = QuiesceInput;
    type Output = QuiescedProjects;
    async fn run(&mut self, input: QuiesceInput) -> orchestrator::Result<QuiescedProjects> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("project executor dropped"))?;
        let stops = join_all(input.flows.into_iter().map(|flow| async {
            tokio::time::timeout(
                Duration::from_secs(input.stop_seconds),
                executor.run::<dora::StopDoraFlow>(flow),
            )
            .await
            .map_err(|_| orchestrator::Error::message("Dora shutdown deadline exceeded"))?
        }))
        .await;
        let mut output = QuiescedProjects {
            projects: Vec::new(),
            writers_stopped: true,
            dora_finalized: stops.iter().all(|s| s.is_ok()),
        };
        let mut dependencies = Vec::new();
        for stopped in stops {
            match stopped {
                Ok(stopped) => dependencies.push(stopped),
                Err(error) => self
                    .diagnostics
                    .borrow_mut()
                    .push(invalid(error.to_string()).diagnostic("Dora shutdown")),
            }
        }
        for group in input.groups.into_iter().rev() {
            let stopped = join_all(group.iter().map(|plan| {
                let mut stop = Input::new(StopInput {
                    services: vec![],
                    allow_volume_deletion: true,
                    plan: Some(plan.clone()),
                    stop_seconds: input.stop_seconds,
                });
                for dependency in &dependencies {
                    stop = stop.after(dependency);
                }
                executor.run::<QuiesceCompose>(stop)
            }))
            .await;
            for (plan, stopped) in group.into_iter().zip(stopped) {
                let diagnostics = match stopped {
                    Ok(stopped) => {
                        output.writers_stopped &= stopped.value().stopped;
                        stopped.into_value().diagnostics
                    }
                    Err(error) => {
                        output.writers_stopped = false;
                        vec![invalid(error.to_string()).diagnostic("quiesce")]
                    }
                };
                output.projects.push(QuiescedProject { plan, diagnostics });
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoveInput {
    pub projects: Vec<QuiescedProject>,
    pub stop_seconds: u64,
    /// Finalization permits removal only after durable evidence and overlay disposition.
    pub can_remove: bool,
    pub preserve_volumes: bool,
}
/// Remove independent projects after the caller's finalization barrier.
/// Failed removal and deliberately retained resources remain explicit in cleanup.
pub struct RemoveProjects {
    pub executor: Weak<LocalExecutor>,
}
impl Step for RemoveProjects {
    const ID: Option<&'static str> = Some("compose.remove");
    const DISPLAY_NAME: Option<&'static str> = Some("Remove resources");
    type Input = RemoveInput;
    type Output = Vec<Cleanup>;
    async fn run(&mut self, input: RemoveInput) -> orchestrator::Result<Vec<Cleanup>> {
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| orchestrator::Error::message("project executor dropped"))?;
        Ok(join_all(input.projects.into_iter().map(|project| async {
            let mut plan = project.plan;
            let mut result = if input.can_remove {
                if input.preserve_volumes {
                    plan.remove_volumes = false;
                }
                match executor
                    .run::<RemoveCompose>(StopInput {
                        services: vec![],
                        allow_volume_deletion: !input.preserve_volumes,
                        plan: Some(plan.clone()),
                        stop_seconds: input.stop_seconds,
                    })
                    .await
                {
                    Ok(cleanup) => cleanup.into_value(),
                    Err(error) => Cleanup {
                        project: plan.name,
                        diagnostics: vec![invalid(error.to_string()).diagnostic("remove")],
                        remaining: vec!["unknown".into()],
                        retained_volumes: vec![],
                        verified: false,
                    },
                }
            } else {
                Cleanup {
                    project: plan.name,
                    diagnostics: vec![],
                    remaining: vec!["preserved: shutdown or filesystem evidence incomplete".into()],
                    retained_volumes: vec![],
                    verified: false,
                }
            };
            result.diagnostics.extend(project.diagnostics);
            result
        }))
        .await)
    }
}

fn step_error(error: impl std::fmt::Display) -> orchestrator::Error {
    orchestrator::Error::message(error.to_string())
}
