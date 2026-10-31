//! Communication setup owns the coordinator and daemon; applications stay step-owned.
use super::experiment::ExperimentOwner;
use super::shared_dora::{COORDINATOR, DAEMON};
use crate::compose::{error::invalid, *};
use std::time::Duration;

include!(concat!(env!("OUT_DIR"), "/doramcap.dora.v1.rs"));

pub(crate) fn services(runtime: &DoraRuntime) -> serde_json::Value {
    let service = |args: Vec<String>| {
        serde_json::json!({
            "image":runtime.image, "network_mode":"host", "stop_signal":"SIGINT",
            "ulimits":{"memlock":{"soft":runtime.memlock_bytes,"hard":runtime.memlock_bytes}},
            "user":format!("{}:{}",runtime.participant_uid.unwrap(),runtime.participant_gid.unwrap()),
            "working_dir":runtime.runtime_directory, "environment":{"HOME":runtime.runtime_directory,"DORA_DAEMON_LOCAL_LISTEN_PORT":runtime.daemon_port.to_string()},
            "entrypoint":["/usr/local/bin/dora"], "command":args,
            "volumes":[{"type":"bind","source":runtime.runtime_directory,"target":runtime.runtime_directory},
                {"type":"bind","source":runtime.zenoh_overlay,"target":runtime.zenoh_overlay,"read_only":true}]
        })
    };
    let mut daemon = service(vec![
        "daemon".into(),
        "--coordinator-port".into(),
        runtime.coordinator_port.to_string(),
        "--local-listen-port".into(),
        runtime.daemon_port.to_string(),
        "--zenoh-config-overlay".into(),
        runtime.zenoh_overlay.clone(),
    ]);
    daemon["ipc"] = "shareable".into();
    daemon["shm_size"] = runtime.shm_bytes.into();
    let mut coordinator = service(vec![
        "coordinator".into(),
        "--interface".into(),
        "127.0.0.1".into(),
        "--port".into(),
        runtime.coordinator_port.to_string(),
        "--store".into(),
        "memory".into(),
    ]);
    coordinator["ipc"] = format!("service:{DAEMON}").into();
    if !runtime.graph.is_empty() {
        coordinator["volumes"].as_array_mut().unwrap().push(serde_json::json!({"type":"bind","source":runtime.graph,"target":runtime.graph,"read_only":true}));
    }
    serde_json::json!({COORDINATOR:coordinator,DAEMON:daemon})
}

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
            .exec(
                &self.plan,
                COORDINATOR,
                &command,
                Duration::from_secs(self.startup_seconds),
            )
            .await
    }
    pub(super) async fn start_runtime(&self) -> Result<()> {
        self.adapter
            .start(StartInput {
                plan: Some(self.plan.clone()),
                services: vec![DAEMON.into(), COORDINATOR.into()],
                readiness_seconds: self.startup_seconds,
            })
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
                .endpoint(&self.plan, name, "host", port as u16)
                .await?;
        }
        Ok(())
    }
    /// Workloads must have drained and stopped before native graph/runtime teardown.
    pub(super) async fn stop_runtime(&self) -> Result<Shutdown> {
        if self
            .adapter
            .endpoint(
                &self.plan,
                COORDINATOR,
                "host",
                self.settings.coordinator_port as u16,
            )
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
            .quiesce(StopInput {
                allow_volume_deletion: false,
                services: vec![COORDINATOR.into(), DAEMON.into()],
                plan: Some(self.plan.clone()),
                stop_seconds: self.stop_seconds,
            })
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
