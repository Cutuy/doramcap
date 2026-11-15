//! Run-owned Docker lifecycle facts, bounded by the frozen Compose replica slots.
use bollard::models::{EventMessage, EventMessageTypeEnum};
use compose_apps::compose::{
    ContainerRuntime, ResolvedComposeProject, RuntimeObservation, adapter::RUN_LABEL,
    container_runtime::State,
};
use std::collections::BTreeMap;
use tokio::sync::watch;

const PROJECT_LABEL: &str = "com.docker.compose.project";
const SERVICE_LABEL: &str = "com.docker.compose.service";
const REPLICA_LABEL: &str = "com.docker.compose.container-number";

pub(super) struct RuntimeTracker {
    pub plan: ResolvedComposeProject,
    slots: BTreeMap<(String, u64), ContainerRuntime>,
    error: String,
    sender: watch::Sender<Option<RuntimeObservation>>,
}
impl RuntimeTracker {
    /// Begin an empty observation for the frozen run; only its declared replica slots are retained.
    pub fn new(
        plan: ResolvedComposeProject,
        sender: watch::Sender<Option<RuntimeObservation>>,
    ) -> Self {
        let tracker = Self {
            plan,
            slots: BTreeMap::new(),
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
        let Some((service, expected)) = attributes
            .get(SERVICE_LABEL)
            .and_then(|service| self.plan.services.get_key_value(service))
        else {
            self.fail("Docker lifecycle event has an unknown service");
            return;
        };
        let Some(replica) = attributes
            .get(REPLICA_LABEL)
            .and_then(|replica| replica.parse::<u64>().ok())
            .filter(|replica| *replica > 0 && *replica <= expected.replicas)
        else {
            self.fail("Docker lifecycle event has an invalid replica number");
            return;
        };
        let Some(id) = actor.id.as_ref().filter(|id| !id.is_empty()) else {
            self.fail("Docker lifecycle event has no container identity");
            return;
        };
        let key = (service.clone(), replica);
        let previous = self.slots.get(&key);
        // Late stop/removal of a replaced container cannot change its replacement.
        if !matches!(action, "create" | "start")
            && previous.is_some_and(|previous| previous.id != *id)
        {
            return;
        }
        let state = match action {
            "start" | "unpause" => Some(State::Running(true)),
            "die" => attributes
                .get("exitCode")
                .and_then(|code| code.parse::<i32>().ok())
                .filter(|code| *code >= 0)
                .map(State::ExitCode),
            "stop" | "destroy" => previous.and_then(|previous| match previous.state {
                Some(State::ExitCode(code)) => Some(State::ExitCode(code)),
                _ => None,
            }),
            _ => None,
        };
        let incomplete_exit = matches!(action, "die" | "stop" | "destroy") && state.is_none();
        self.slots.insert(
            key,
            ContainerRuntime {
                id: id.clone(),
                service: service.clone(),
                state,
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
            containers: self.slots.values().cloned().collect(),
            error: self.error.clone(),
        }));
    }
}

#[cfg(test)]
mod tests;
