//! Dora coordinator and daemon as units; Dora's CLI as oneshot units.
use crate::{DoraUnits, Program, Result, Scope, error, manager::Manager};
use std::{path::PathBuf, time::Duration};

// Approved polling exception, as for Compose: Dora reports no daemon readiness event.
const STARTUP_CHECK_INTERVAL: Duration = Duration::from_millis(100);
/// Zenoh overlay enabling shared-memory transport between local participants.
const ZENOH_OVERLAY: &[u8] = b"{transport:{shared_memory:{enabled:true}}}\n";
/// File name of the registered graph; Dora writes session output beside it.
const GRAPH_FILE: &str = "graph.yml";
/// Signal number of SIGINT, which Dora handles as a graceful stop.
const SIGINT: i32 = 2;
pub(crate) const COORDINATOR: &str = "dora-coordinator";
pub(crate) const DAEMON: &str = "dora-daemon";

/// Reserved prefix for transient Dora control commands.
pub(crate) const CLI_PREFIX: &str = "dora-cli-";

/// One run's Dora runtime: its two units and its registered dataflow.
pub(crate) struct Dora {
    settings: DoraUnits,
    run: String,
    commands: u32,
    readiness_seconds: u64,
    stop_seconds: u64,
    pub registered: Option<String>,
}
impl Dora {
    pub fn new(settings: DoraUnits, run: &crate::RunPlan) -> Self {
        Self {
            settings,
            run: run.id.clone(),
            readiness_seconds: run.readiness_seconds,
            stop_seconds: run.stop_seconds,
            commands: 0,
            registered: None,
        }
    }
    fn directory(&self) -> PathBuf {
        PathBuf::from(&self.settings.runtime_directory)
    }
    fn overlay(&self) -> PathBuf {
        self.directory().join("zenoh.json5")
    }
    /// Coordinator and daemon programs; write the Zenoh overlay they read.
    pub async fn programs(&self) -> Result<Vec<Program>> {
        tokio::fs::write(self.overlay(), ZENOH_OVERLAY)
            .await
            .map_err(error)?;
        let dora = &self.settings;
        let program = |id: &str, args: Vec<String>| Program {
            id: id.into(),
            argv: [vec![dora.executable.clone()], args].concat(),
            user: dora.user.clone(),
            working_directory: dora.runtime_directory.clone(),
            environment: vec![crate::EnvironmentVariable {
                name: "HOME".into(),
                value: dora.runtime_directory.clone(),
            }],
            memlock_bytes: dora.memlock_bytes,
            kill_signal: SIGINT,
            ..Default::default()
        };
        Ok(vec![
            program(
                COORDINATOR,
                vec![
                    "coordinator".into(),
                    "--interface".into(),
                    "127.0.0.1".into(),
                    "--port".into(),
                    dora.coordinator_port.to_string(),
                    "--store".into(),
                    "memory".into(),
                ],
            ),
            program(
                DAEMON,
                vec![
                    "daemon".into(),
                    "--coordinator-port".into(),
                    dora.coordinator_port.to_string(),
                    "--local-listen-port".into(),
                    dora.daemon_port.to_string(),
                    "--zenoh-config-overlay".into(),
                    self.overlay().to_string_lossy().into_owned(),
                ],
            ),
        ])
    }

    /// Run one Dora CLI command as a oneshot unit and return its standard output.
    async fn cli(
        &mut self,
        manager: &Manager,
        scope: &Scope,
        args: &[&str],
        seconds: u64,
    ) -> Result<String> {
        self.commands += 1;
        let name = scope.unit(&format!("{CLI_PREFIX}{}", self.commands));
        let output = self.directory().join(format!("cli-{}.out", self.commands));
        let argv: Vec<String> = [self.settings.executable.as_str()]
            .into_iter()
            .chain(args.iter().copied())
            .chain([
                "--coordinator-port",
                &self.settings.coordinator_port.to_string(),
            ])
            .map(str::to_owned)
            .collect();
        manager
            .run_oneshot(scope, &name, &argv, &self.settings.user, &output, seconds)
            .await?;
        tokio::fs::read_to_string(output).await.map_err(error)
    }

    /// Wait until the daemon reports itself connected to the coordinator.
    pub async fn await_daemon(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        loop {
            if let Ok(status) = self
                .cli(
                    manager,
                    scope,
                    &["status", "--format", "json"],
                    self.readiness_seconds,
                )
                .await
                && serde_json::from_str::<serde_json::Value>(&status)
                    .is_ok_and(|status| status["daemon"]["status"] == "running")
            {
                return Ok(());
            }
            tokio::time::sleep(STARTUP_CHECK_INTERVAL).await;
        }
    }

    /// Register the graph as the run's dataflow and record its identity.
    pub async fn register(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        if self.settings.graph.is_empty() {
            return Ok(());
        }
        let graph = self.directory().join(GRAPH_FILE);
        tokio::fs::copy(&self.settings.graph, &graph)
            .await
            .map_err(error)?;
        let graph = graph.to_string_lossy().into_owned();
        let run = self.run.clone();
        self.cli(
            manager,
            scope,
            &["start", &graph, "--name", &run, "--detach"],
            self.readiness_seconds,
        )
        .await?;
        let listed = self
            .cli(
                manager,
                scope,
                &["list", "--format", "json", "--name", &run],
                self.readiness_seconds,
            )
            .await?;
        let entries: Vec<serde_json::Value> = listed
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .map_err(error)?;
        match entries.as_slice() {
            [entry] if entry["name"] == run.as_str() && entry["status"] == "Running" => {
                self.registered = entry["uuid"].as_str().map(str::to_owned);
                self.registered
                    .as_ref()
                    .map(|_| ())
                    .ok_or_else(|| "Dora graph UUID missing".into())
            }
            _ => Err(format!("Dora graph identity is ambiguous: {listed}")),
        }
    }

    /// Ask the coordinator to stop every dataflow before its units stop.
    pub async fn down(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        self.cli(manager, scope, &["down", "--force"], self.stop_seconds)
            .await
            .map(|_| ())
    }
}
