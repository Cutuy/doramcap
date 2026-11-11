//! Readiness rules for an inspected Compose project.
use super::{ComposeProjectState, Container, Result, error::invalid};

/// Memory guard for output a workload controls; ordinary logs fit whole.
pub(crate) const MAX_OUTPUT_CHARS: usize = 256 * 1024;

/// Append container output as printable text, keeping the newest `MAX_OUTPUT_CHARS`.
pub(crate) fn append_output(output: &mut String, bytes: &[u8]) {
    output.extend(
        String::from_utf8_lossy(bytes)
            .chars()
            .filter(|c| !c.is_control() || *c == '\n'),
    );
    let excess = output.chars().count().saturating_sub(MAX_OUTPUT_CHARS);
    if let Some((start, _)) = output.char_indices().nth(excess) {
        output.drain(..start);
    }
}

impl Container {
    /// True once this container stopped unexpectedly or Engine reports it unhealthy.
    /// `completed` marks a service whose clean exit is expected.
    pub(crate) fn failed(&self, completed: bool) -> bool {
        (self.status == "exited" && !(completed && self.exit_code == Some(0)))
            || self.status == "dead"
            || self.health.as_deref() == Some("unhealthy")
    }
    /// Append recent output to a failure message when it is available.
    fn with_output(&self, message: String) -> String {
        if self.last_output.is_empty() {
            message
        } else {
            format!("{message}\n{}", self.last_output)
        }
    }
}

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
                    let status = container
                        .exit_code
                        .map_or("unknown".to_owned(), |code| code.to_string());
                    return Err(invalid(container.with_output(format!(
                        "{name} exited unexpectedly with status {status}"
                    ))));
                }
                if container.status == "dead" || container.health.as_deref() == Some("unhealthy") {
                    return Err(invalid(container.with_output(format!(
                        "{name}: {} / {:?}",
                        container.status, container.health
                    ))));
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
