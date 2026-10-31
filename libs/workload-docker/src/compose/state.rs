//! Readiness rules for an inspected Compose project.
use super::{ComposeProjectState, Result, error::invalid};

impl ComposeProjectState {
    /// Evaluate expected replicas, image identity, process state and health.
    /// Returns `false` while readiness is incomplete; unexpected exits, image
    /// changes and restarts return errors with diagnostic context.
    pub fn ready(&self) -> Result<bool> {
        let plan = self
            .plan
            .as_ref()
            .ok_or_else(|| invalid("Compose state plan missing"))?;
        for (name, service) in &plan.services {
            let containers: Vec<_> = self
                .containers
                .iter()
                .filter(|c| c.service == *name)
                .collect();
            if containers.len() as u64 != service.replicas {
                return Ok(false);
            }
            for container in containers {
                if container.image_id != service.image_id {
                    return Err(invalid(format!("{name}: image changed")));
                }
                if container.restart_count != 0 {
                    return Err(invalid(format!("{name}: unexpected autonomous restart")));
                }
                if container.status == "exited" {
                    if service.completed && container.exit_code == Some(0) {
                        continue;
                    }
                    return Err(invalid(format!(
                        "{name} exited unexpectedly: {:?}",
                        container.exit_code
                    )));
                }
                if container.status == "dead" || container.health.as_deref() == Some("unhealthy") {
                    return Err(invalid(format!(
                        "{name}: {} / {:?}",
                        container.status, container.health
                    )));
                }
                if container.status != "running"
                    || container
                        .health
                        .as_deref()
                        .is_some_and(|h| h != "healthy" && h != "none")
                {
                    return Ok(false);
                }
                if service.completed {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
