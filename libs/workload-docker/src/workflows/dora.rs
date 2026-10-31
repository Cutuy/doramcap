//! Communication setup owns the coordinator and daemon; applications stay step-owned.
use super::experiment::ExperimentOwner;
use super::shared_dora::{COORDINATOR, DAEMON};
use crate::compose::{error::invalid, *};
use std::{collections::BTreeMap, time::Duration};

include!(concat!(env!("OUT_DIR"), "/doramcap.dora.v1.rs"));

/// Build typed Compose declarations for the owned coordinator and daemon.
pub(crate) fn services(runtime: &DoraRuntime) -> BTreeMap<String, RuntimeService> {
    let bind = |source: &str, read_only| BindMount {
        r#type: "bind".into(),
        source: source.into(),
        target: source.into(),
        read_only,
    };
    let service = |command| RuntimeService {
        image: runtime.image.clone(),
        network_mode: "host".into(),
        stop_signal: "SIGINT".into(),
        ulimits: BTreeMap::from([(
            "memlock".into(),
            ResourceLimit {
                soft: runtime.memlock_bytes,
                hard: runtime.memlock_bytes,
            },
        )]),
        user: format!(
            "{}:{}",
            runtime.participant_uid.unwrap(),
            runtime.participant_gid.unwrap()
        ),
        working_dir: runtime.runtime_directory.clone(),
        environment: BTreeMap::from([
            ("HOME".into(), runtime.runtime_directory.clone()),
            (
                "DORA_DAEMON_LOCAL_LISTEN_PORT".into(),
                runtime.daemon_port.to_string(),
            ),
        ]),
        entrypoint: vec!["/usr/local/bin/dora".into()],
        command,
        volumes: vec![
            bind(&runtime.runtime_directory, None),
            bind(&runtime.zenoh_overlay, Some(true)),
        ],
        ipc: "shareable".into(),
        shm_size: None,
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
    daemon.shm_size = Some(runtime.shm_bytes);
    let mut coordinator = service(vec![
        "coordinator".into(),
        "--interface".into(),
        "127.0.0.1".into(),
        "--port".into(),
        runtime.coordinator_port.to_string(),
        "--store".into(),
        "memory".into(),
    ]);
    coordinator.ipc = format!("service:{DAEMON}");
    if !runtime.graph.is_empty() {
        // Dora creates session output beside the graph, inside its writable runtime mount.
        coordinator.volumes.push(BindMount {
            target: std::path::Path::new(&runtime.runtime_directory)
                .join(super::shared_dora::GRAPH_FILE)
                .to_string_lossy()
                .into_owned(),
            ..bind(&runtime.graph, Some(true))
        });
    }
    BTreeMap::from([(COORDINATOR.into(), coordinator), (DAEMON.into(), daemon)])
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
