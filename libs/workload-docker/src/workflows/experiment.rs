//! Workload-owned graph registration and system application shutdown.
use super::dora::DoraRuntime;
use crate::compose::{error::invalid, *};
use orchestrator::Step;
use std::rc::Rc;

/// Owns one experiment's Dora runtime control and registered graph identity.
/// Starts and monitors the runtime, registers the graph, and stops system services.
pub struct ExperimentOwner {
    identity: Option<String>,
    attempted: bool,
    pub(crate) stop_seconds: u64,
    pub(crate) startup_seconds: u64,
    pub(crate) settings: DoraRuntime,
    pub(crate) adapter: Rc<DockerAdapter>,
    pub(crate) plan: ResolvedComposeProject,
}
impl ExperimentOwner {
    pub(crate) fn new(
        settings: DoraRuntime,
        adapter: Rc<DockerAdapter>,
        plan: ResolvedComposeProject,
        seconds: u64,
        stop_seconds: u64,
    ) -> Self {
        Self {
            identity: None,
            attempted: false,
            stop_seconds,
            settings,
            adapter,
            plan,
            startup_seconds: seconds,
        }
    }
    async fn entries(&self) -> Result<Vec<serde_json::Value>> {
        let output = self
            .cli(&["list", "--format", "json", "--name", &self.plan.run])
            .await?;
        output
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(Into::into))
            .collect()
    }
    pub async fn register_graph(&mut self) -> Result<()> {
        if self.settings.graph.is_empty() {
            return Ok(());
        }
        let bytes = crate::evidence::read_regular(
            std::path::Path::new(&self.settings.graph),
            2 * 1024 * 1024,
        )
        .await?;
        if crate::compose::adapter::digest(&bytes) != self.settings.graph_sha256 {
            return Err(invalid("frozen Dora graph changed"));
        }
        self.attempted = true;
        self.cli(&[
            "start",
            super::shared_dora::GRAPH_FILE,
            "--name",
            &self.plan.run,
            "--detach",
        ])
        .await?;
        let entries = self.entries().await?;
        if entries.len() != 1
            || entries[0]["name"] != self.plan.run
            || entries[0]["status"] != "Running"
        {
            return Err(invalid("Dora graph identity is ambiguous"));
        }
        self.identity = Some(
            entries[0]["uuid"]
                .as_str()
                .ok_or_else(|| invalid("Dora graph UUID missing"))?
                .into(),
        );
        Ok(())
    }
    pub async fn inspect(&self) -> Result<()> {
        self.inspect_runtime().await?;
        if self.attempted {
            let entries = self.entries().await?;
            if entries.len() != 1
                || entries[0]["uuid"].as_str() != self.identity.as_deref()
                || entries[0]["status"] != "Running"
            {
                return Err(invalid("Dora graph stopped unexpectedly"));
            }
        }
        Ok(())
    }
    /// Request Dora stop for every system node; Compose then verifies shutdown.
    pub async fn shutdown(&self, system: &[String]) -> Result<()> {
        let Some(uuid) = &self.identity else {
            return if self.attempted {
                Err(invalid("Dora graph registration incomplete"))
            } else {
                Ok(())
            };
        };
        let bytes = crate::evidence::read_regular(
            std::path::Path::new(&self.settings.graph),
            2 * 1024 * 1024,
        )
        .await?;
        let graph = super::shared_dora::graph(&self.settings, &bytes)?;
        let state = self.adapter.project(&self.plan).inspect().await?;
        let nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| system.contains(&n.id.to_string()))
            .collect();
        for node in nodes {
            if state
                .containers
                .iter()
                .any(|container| container.service == node.id.as_ref() && container.pid != 0)
            {
                self.cli(&[
                    "node",
                    "stop",
                    node.id.as_ref(),
                    "--dataflow",
                    uuid,
                    "--grace",
                    &format!("{}s", self.stop_seconds.saturating_sub(1).max(1)),
                ])
                .await?;
            }
        }
        Ok(())
    }
}

pub type SharedExperiment = Rc<tokio::sync::Mutex<ExperimentOwner>>;
macro_rules! step {
    ($name:ident,$method:ident) => {
        step!($name, $method, ());
    };
    ($name:ident,$method:ident,$output:ty) => {
        pub struct $name {
            pub owner: SharedExperiment,
        }
        impl Step for $name {
            type Input = ();
            type Output = $output;
            async fn run(&mut self, (): ()) -> orchestrator::Result<$output> {
                self.owner
                    .lock()
                    .await
                    .$method()
                    .await
                    .map_err(|e| orchestrator::Error::message(e.to_string()))
            }
        }
    };
}
step!(RegisterDoraGraph, register_graph);
step!(StartDoraRuntime, start_runtime);
step!(StopDoraRuntime, stop_runtime, Shutdown);
