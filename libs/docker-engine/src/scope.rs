//! Idempotent cleanup of a controller's Docker ownership scope.
use super::{
    CONTROLLER_LABEL, Engine, Result,
    error::{absent, engine, invalid},
};
use bollard::query_parameters::*;
use std::collections::HashMap;

/// Resets the resources labeled for one controller, preserving volumes.
pub struct OwnedResources<'a> {
    pub(super) engine: &'a Engine,
    pub(super) scope: &'a str,
    pub(super) owner: &'a str,
}
impl OwnedResources<'_> {
    /// Stop every writer and return summaries from the stopped-state check.
    /// Requires exclusive scope ownership and a timeout of 0 through `i32::MAX` seconds.
    /// Foreign reservations or Engine failures abort. Failure or cancellation can leave some writers stopped.
    pub async fn quiesce(&self, seconds: u64) -> Result<Vec<bollard::models::ContainerSummary>> {
        let seconds = i32::try_from(seconds).map_err(|_| {
            invalid(format!(
                "Docker stop timeout {seconds}s exceeds {}s",
                i32::MAX
            ))
        })?;
        match self.engine.docker.inspect_container(self.owner, None).await {
            Ok(ref container)
                if container
                    .config
                    .as_ref()
                    .and_then(|c| c.labels.as_ref())
                    .and_then(|labels| labels.get(CONTROLLER_LABEL))
                    .map(String::as_str)
                    != Some(self.scope) =>
            {
                return Err(invalid(
                    "repository experiment reservation belongs to another controller",
                ));
            }
            Ok(_) => {}
            Err(e) if absent(&e) => {}
            Err(e) => return Err(engine("inspect experiment reservation", e)),
        }
        for container in self.containers().await? {
            match self
                .engine
                .docker
                .stop_container(
                    container.id.as_deref().unwrap(),
                    Some(StopContainerOptions {
                        t: Some(seconds),
                        ..Default::default()
                    }),
                )
                .await
            {
                Ok(()) => {}
                Err(e)
                    if absent(&e)
                        || matches!(
                            &e,
                            bollard::errors::Error::DockerResponseServerError {
                                status_code: 304,
                                ..
                            }
                        ) => {}
                Err(e) => return Err(engine("stop experiment writer", e)),
            }
        }
        self.verify_stopped().await
    }
    async fn containers(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        let containers = self
            .engine
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: scope_filters(self.scope),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("list experiment containers", e))?;
        if containers.iter().any(|c| {
            c.id.as_deref().is_none_or(str::is_empty)
                || c.labels
                    .as_ref()
                    .and_then(|l| l.get(CONTROLLER_LABEL))
                    .map(String::as_str)
                    != Some(self.scope)
        }) {
            return Err(invalid("invalid experiment container ownership"));
        }
        Ok(containers)
    }
    async fn networks(&self) -> Result<Vec<bollard::models::Network>> {
        let networks = self
            .engine
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: scope_filters(self.scope),
            }))
            .await
            .map_err(|e| engine("list experiment networks", e))?;
        if networks.iter().any(|n| {
            n.id.as_deref().is_none_or(str::is_empty)
                || n.labels
                    .as_ref()
                    .and_then(|l| l.get(CONTROLLER_LABEL))
                    .map(String::as_str)
                    != Some(self.scope)
        }) {
            return Err(invalid("invalid experiment network ownership"));
        }
        Ok(networks)
    }
    /// Verify zero PIDs and stopped state for every owned container.
    /// Returns the inspected ownership snapshot; missing containers are tolerated.
    pub async fn verify_stopped(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        let containers = self.containers().await?;
        for container in &containers {
            match self
                .engine
                .docker
                .inspect_container(container.id.as_deref().unwrap(), None)
                .await
            {
                Ok(container) => {
                    let state = container
                        .state
                        .ok_or_else(|| invalid("experiment container state missing"))?;
                    if state.pid != Some(0) || state.running != Some(false) {
                        return Err(invalid("experiment still has live or unverified writers"));
                    }
                }
                Err(e) if absent(&e) => {}
                Err(e) => return Err(engine("inspect experiment writer", e)),
            }
        }
        Ok(containers)
    }
    /// Remove stopped containers and networks, keeping the supplied container until last.
    /// Requires exclusive scope ownership. Only an in-scope final container ID is honored.
    /// Failure or cancellation can leave partial cleanup. Retry with the same scope and final ID.
    pub async fn remove(&self, last: Option<&str>) -> Result<()> {
        self.verify_stopped().await?;
        let mut owner = None;
        for container in self.containers().await? {
            let id = container.id.unwrap();
            if Some(id.as_str()) == last {
                owner = Some(id);
            } else {
                self.remove_container(&id).await?;
            }
        }
        for network in self.networks().await? {
            if let Err(e) = self
                .engine
                .docker
                .remove_network(network.id.as_deref().unwrap())
                .await
                && !absent(&e)
            {
                return Err(engine("remove experiment network", e));
            }
        }
        if !self.networks().await?.is_empty()
            || self.containers().await?.iter().any(|c| c.id != owner)
        {
            return Err(invalid("experiment resources remain after reset"));
        }
        if let Some(id) = owner {
            self.remove_container(&id).await?;
        }
        if !self.containers().await?.is_empty() || !self.networks().await?.is_empty() {
            return Err(invalid("experiment resources remain after reset"));
        }
        Ok(())
    }
    async fn remove_container(&self, id: &str) -> Result<()> {
        match self
            .engine
            .docker
            .remove_container(id, None::<RemoveContainerOptions>)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) if absent(&e) => Ok(()),
            Err(e) => Err(engine("remove experiment container", e)),
        }
    }
}
fn scope_filters(scope: &str) -> Option<HashMap<String, Vec<String>>> {
    Some(HashMap::from([(
        "label".into(),
        vec![format!("{CONTROLLER_LABEL}={scope}")],
    )]))
}
