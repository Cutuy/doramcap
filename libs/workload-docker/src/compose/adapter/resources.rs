//! Discover existing run-owned Docker resources for explicit recovery.
use super::project::PROJECT_LABEL;
use super::{ARTIFACTS_LABEL, DockerAdapter, RUN_LABEL, validate_name};
use crate::compose::{
    OwnedResource, Result,
    error::{engine, invalid},
};
use bollard::query_parameters::*;
use std::collections::HashMap;
/// Borrows the Engine connection for ownership discovery without changing resources.
pub struct OwnedResources<'a> {
    pub(super) adapter: &'a DockerAdapter,
}
impl OwnedResources<'_> {
    /// List containers, networks and volumes bearing a run label with `prefix`.
    ///
    /// Docker list failures or missing resource identities return an error. The
    /// three Engine calls are not an atomic snapshot: callers must prevent other
    /// managers from creating resources concurrently. No resources are modified.
    pub async fn discover_runs(&self, prefix: &str) -> Result<Vec<OwnedResource>> {
        validate_name(prefix)?;
        let filters = Some(HashMap::from([(
            "label".to_owned(),
            vec![RUN_LABEL.to_owned()],
        )]));
        let mut found = Vec::new();
        let containers = self
            .adapter
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: filters.clone(),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("discover containers", e))?;
        for item in containers {
            let labels = item.labels.unwrap_or_default();
            if let Some(run) = labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: labels.get(PROJECT_LABEL).cloned(),
                    kind: "container".into(),
                    id: item
                        .id
                        .ok_or_else(|| invalid("discovered container has no ID"))?,
                });
            }
        }
        let networks = self
            .adapter
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: filters.clone(),
            }))
            .await
            .map_err(|e| engine("discover networks", e))?;
        for item in networks {
            let labels = item.labels.unwrap_or_default();
            if let Some(run) = labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: labels.get(PROJECT_LABEL).cloned(),
                    kind: "network".into(),
                    id: item
                        .id
                        .ok_or_else(|| invalid("discovered network has no ID"))?,
                });
            }
        }
        let volumes = self
            .adapter
            .docker
            .list_volumes(Some(ListVolumesOptions { filters }))
            .await
            .map_err(|e| engine("discover volumes", e))?;
        for item in volumes.volumes.unwrap_or_default() {
            if let Some(run) = item.labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: item.labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: item.labels.get(PROJECT_LABEL).cloned(),
                    kind: "volume".into(),
                    id: item.name,
                });
            }
        }
        Ok(found)
    }
}
