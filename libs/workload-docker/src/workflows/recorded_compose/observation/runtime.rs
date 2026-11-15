//! Latest lifecycle facts for each run-owned Docker container.
use bollard::models::{EventMessage, EventMessageTypeEnum};
use compose_apps::compose::{
    ContainerRuntime, ResolvedComposeProject, RuntimeObservation, adapter::RUN_LABEL,
    container_runtime::State,
};
use std::collections::BTreeMap;
use tokio::sync::watch;

const PROJECT_LABEL: &str = "com.docker.compose.project";
const SERVICE_LABEL: &str = "com.docker.compose.service";

struct TrackedContainer {
    observation: ContainerRuntime,
    awaiting_exit: bool,
}

pub(super) struct RuntimeTracker {
    pub plan: ResolvedComposeProject,
    containers: BTreeMap<String, TrackedContainer>,
    error: String,
    sender: watch::Sender<Option<RuntimeObservation>>,
}
impl RuntimeTracker {
    /// Begin an empty observation; retain each observed container ID until this run ends.
    pub fn new(
        plan: ResolvedComposeProject,
        sender: watch::Sender<Option<RuntimeObservation>>,
    ) -> Self {
        let tracker = Self {
            plan,
            containers: BTreeMap::new(),
            error: String::new(),
            sender,
        };
        tracker.publish();
        tracker
    }

    /// Track run-owned process events; foreign events remain solely in the audit stream.
    pub fn observe(&mut self, event: &EventMessage) {
        if event.typ != Some(EventMessageTypeEnum::CONTAINER) {
            return;
        }
        let Some(action) = event.action.as_deref().filter(|action| {
            matches!(
                *action,
                "create" | "start" | "die" | "stop" | "destroy" | "pause" | "unpause"
            )
        }) else {
            return;
        };
        let Some(actor) = &event.actor else {
            return;
        };
        let Some(attributes) = &actor.attributes else {
            return;
        };
        if attributes.get(PROJECT_LABEL) != Some(&self.plan.name)
            || attributes.get(RUN_LABEL) != Some(&self.plan.run)
        {
            return;
        }
        let Some(service) = attributes
            .get(SERVICE_LABEL)
            .filter(|service| self.plan.services.contains_key(*service))
        else {
            self.fail("Docker lifecycle event has an unknown service");
            return;
        };
        let Some(id) = actor.id.as_ref().filter(|id| !id.is_empty()) else {
            self.fail("Docker lifecycle event has no container identity");
            return;
        };
        let previous = self.containers.get(id);
        let state = match action {
            "start" | "unpause" => Some(State::Running(true)),
            "die" => attributes
                .get("exitCode")
                .and_then(|code| code.parse::<i32>().ok())
                .filter(|code| *code >= 0)
                .map(State::ExitCode),
            "stop" | "destroy" => previous.and_then(|previous| match previous.observation.state {
                Some(State::ExitCode(code)) => Some(State::ExitCode(code)),
                _ => None,
            }),
            _ => None,
        };
        let incomplete_exit = (action == "die" && state.is_none())
            || (action == "destroy" && previous.is_some_and(|container| container.awaiting_exit));
        let awaiting_exit = match action {
            "start" | "unpause" => true,
            "create" | "die" | "destroy" => false,
            _ => previous.is_some_and(|container| container.awaiting_exit),
        };
        self.containers.insert(
            id.clone(),
            TrackedContainer {
                observation: ContainerRuntime {
                    container_id: id.clone(),
                    service_name: service.clone(),
                    state,
                },
                awaiting_exit,
            },
        );
        if incomplete_exit {
            self.fail("Docker lifecycle event has no valid exit code");
        } else {
            self.publish();
        }
    }

    /// Preserve accepted facts and mark this run incomplete; only a new run clears errors.
    pub fn fail(&mut self, error: &str) {
        self.error = error.to_owned();
        self.publish();
    }

    fn publish(&self) {
        self.sender.send_replace(Some(RuntimeObservation {
            containers: self
                .containers
                .values()
                .map(|container| container.observation.clone())
                .collect(),
            error: self.error.clone(),
        }));
    }
}

#[cfg(test)]
mod tests;
