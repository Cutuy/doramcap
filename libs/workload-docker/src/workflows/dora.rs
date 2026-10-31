//! Communication setup owns the coordinator and daemon; applications stay step-owned.
use super::experiment::ExperimentOwner;
use super::shared_dora::{COORDINATOR, DAEMON};
use crate::compose::{error::invalid, *};
use std::time::Duration;

pub use crate::proto::doramcap::dora::v1::*;

impl ExperimentOwner {
    pub(crate) async fn cli(&self, args: &[&str]) -> Result<String> {
        let mut command: Vec<_> = ["/usr/local/bin/dora"]
            .into_iter()
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();
        command.extend([
            "--coordinator-port".into(),
            self.settings.coordinator_port.to_string(),
        ]);
        self.adapter
            .project(&self.plan)
            .exec(
                COORDINATOR,
                &command,
                Duration::from_secs(self.startup_seconds),
            )
            .await
    }
    pub(super) async fn start_runtime(&self) -> Result<()> {
        self.adapter
            .project(&self.plan)
            .start(&[DAEMON.into(), COORDINATOR.into()], self.startup_seconds)
            .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.startup_seconds);
        loop {
            if self.inspect_runtime().await.is_ok()
                && let Ok(status) = self.cli(&["status", "--format", "json"]).await
            {
                let status: serde_json::Value = serde_json::from_str(&status)?;
                if status["daemon"]["status"] == "running" {
                    return Ok(());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(invalid("Dora daemon did not connect to coordinator"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub(super) async fn inspect_runtime(&self) -> Result<()> {
        for (name, port) in [
            (COORDINATOR, self.settings.coordinator_port),
            (DAEMON, self.settings.daemon_port),
        ] {
            self.adapter
                .project(&self.plan)
                .endpoint(name, "host", port as u16)
                .await?;
        }
        Ok(())
    }
    /// Workloads must have drained and stopped before native graph/runtime teardown.
    pub(super) async fn stop_runtime(&self) -> Result<Shutdown> {
        if self
            .adapter
            .project(&self.plan)
            .endpoint(COORDINATOR, "host", self.settings.coordinator_port as u16)
            .await
            .is_ok()
            && let Err(error) = self.cli(&["down", "--force"]).await
        {
            eprintln!(
                "warning: Dora control shutdown failed; verifying container shutdown: {error}"
            );
        }
        let stopped = self
            .adapter
            .project(&self.plan)
            .quiesce(&[COORDINATOR.into(), DAEMON.into()], self.stop_seconds)
            .await?;
        if !stopped.stopped {
            return Err(invalid(format!(
                "Dora shutdown incomplete: {:?}",
                stopped.diagnostics
            )));
        }
        Ok(stopped)
    }
}
