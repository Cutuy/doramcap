//! Idempotent cleanup of a controller's Docker ownership scope.
use super::{CONTROLLER_LABEL, DockerAdapter, EXPERIMENT_OWNER_LABEL};
use crate::compose::{
    Result,
    error::{absent, engine, invalid},
};
use bollard::query_parameters::*;
use std::collections::HashMap;

/// Resets the resources labeled for one controller, preserving volumes.
pub(crate) struct ExperimentResources<'a> {
    pub(super) adapter: &'a DockerAdapter,
    pub(super) scope: &'a str,
}
impl ExperimentResources<'_> {
    /// Stop and verify every writer; foreign reservations or Engine failures abort reset.
    pub(crate) async fn quiesce(&self, seconds: u64) -> Result<()> {
        match self
            .adapter
            .docker
            .inspect_container(crate::workflows::shared_dora::MACHINE_OWNER, None)
            .await
        {
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
                    "machine experiment reservation belongs to another controller",
                ));
            }
            Ok(_) => {}
            Err(e) if absent(&e) => {}
            Err(e) => return Err(engine("inspect experiment reservation", e)),
        }
        for container in self.containers().await? {
            match self
                .adapter
                .docker
                .stop_container(
                    container.id.as_deref().unwrap(),
                    Some(StopContainerOptions {
                        t: Some(seconds as i32),
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
            .adapter
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
            .adapter
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
    pub(super) async fn verify_stopped(&self) -> Result<()> {
        for container in self.containers().await? {
            match self
                .adapter
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
        Ok(())
    }
    /// Remove stopped containers and networks, releasing the IPC owner last.
    pub(crate) async fn remove(&self) -> Result<()> {
        self.verify_stopped().await?;
        let mut owner = None;
        // Keep the daemon reservation until all other disposable resources are gone.
        for container in self.containers().await? {
            let id = container.id.unwrap();
            if container
                .labels
                .is_some_and(|l| l.contains_key(EXPERIMENT_OWNER_LABEL))
            {
                if owner.replace(id).is_some() {
                    return Err(invalid("multiple experiment IPC owners"));
                }
            } else {
                self.remove_container(&id).await?;
            }
        }
        for network in self.networks().await? {
            if let Err(e) = self
                .adapter
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
            .adapter
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
