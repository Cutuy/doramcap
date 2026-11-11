//! Workload-owned graph registration and system application shutdown.
use super::dora::DoraApps;
use crate::compose::{error::invalid, *};
use dora_control::DoraCoordinatorClient;
use orchestrator::Step;
use std::{path::Path, rc::Rc, time::Duration};
use tokio::time::Instant;

/// Owns one experiment's Dora runtime control and registered graph identity.
/// Starts and monitors the runtime, registers the graph, and stops system services.
pub struct ExperimentOwner {
    pub(super) client: Option<DoraCoordinatorClient>,
    pub(super) startup_deadline: Option<Instant>,
    pub(super) stop_deadline: Option<Instant>,
    pub(crate) stop_seconds: u64,
    pub(crate) startup_seconds: u64,
    pub(crate) apps: DoraApps,
    pub(crate) run: workload_lifecycle::launch::RunPlan,
    pub(crate) adapter: Rc<DockerAdapter>,
    pub(crate) plan: ResolvedComposeProject,
}
impl ExperimentOwner {
    pub(crate) fn new(
        apps: DoraApps,
        run: workload_lifecycle::launch::RunPlan,
        adapter: Rc<DockerAdapter>,
        plan: ResolvedComposeProject,
        seconds: u64,
        stop_seconds: u64,
    ) -> Self {
        Self {
            client: None,
            startup_deadline: None,
            stop_deadline: None,
            stop_seconds,
            apps,
            run,
            adapter,
            plan,
            startup_seconds: seconds,
        }
    }
    pub(crate) fn dora_runtime(&self) -> Result<&workload_lifecycle::launch::DoraRuntime> {
        self.run
            .dora_runtime
            .as_ref()
            .ok_or_else(|| invalid("prepared Dora runtime missing"))
    }
    pub async fn register_graph(&mut self) -> Result<()> {
        if self
            .run
            .dora_runtime
            .as_ref()
            .and_then(|runtime| runtime.dataflow.as_ref())
            .is_none()
        {
            return Ok(());
        }
        let deadline = self
            .startup_deadline
            .ok_or_else(|| invalid("Dora runtime has not started"))?;
        let registration = async {
            let Some(dataflow) = self
                .run
                .dora_runtime
                .as_ref()
                .and_then(|runtime| runtime.dataflow.as_ref())
            else {
                return Ok(());
            };
            if self
                .client
                .as_ref()
                .is_some_and(|client| client.graph().is_some())
            {
                return Err(invalid("Dora graph start already attempted"));
            }
            let bytes = crate::evidence::read_regular(
                std::path::Path::new(&dataflow.source),
                super::shared_dora::MAX_GRAPH_BYTES,
            )
            .await?;
            if crate::compose::adapter::digest(&bytes) != self.dora_runtime()?.dataflow_sha256 {
                return Err(invalid("frozen Dora graph changed"));
            }
            self.inspect_runtime().await?;
            let graph =
                workload_lifecycle::dora::graph(&dataflow.source, &bytes).map_err(invalid)?;
            let app = self.apps.coordinator()?.1;
            let directory = app["working_dir"]
                .as_str()
                .ok_or_else(|| invalid("Dora working_dir missing"))?;
            let source_directory = DoraApps::host_path(app, directory)?;
            let session = dora_control::protocol::SessionId::generate();
            let events = app["environment"]["DORA_WRITE_EVENTS_TO"]
                .as_str()
                .map(Into::into);
            let start = workload_lifecycle::dora::start(
                graph,
                self.plan.run.clone(),
                directory.into(),
                session,
                events,
            )
            .map_err(invalid)?;
            let out = Path::new(&source_directory).join("out");
            tokio::fs::create_dir_all(&out).await?;
            crate::compose::adapter::private_write(
                &Path::new(&source_directory).join(workload_lifecycle::dora::SESSION_FILE),
                &serde_json::to_vec_pretty(&workload_lifecycle::dora::session_artifact(session))?,
            )
            .await?;
            let ignore = out.join(".gitignore");
            let mut entries = match tokio::fs::read_to_string(&ignore).await {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(e.into()),
            };
            if !entries
                .lines()
                .any(|line| line == "/dataflow.dora-session.yaml")
            {
                entries.push_str("\n/dataflow.dora-session.yaml\n");
                crate::compose::adapter::private_write(&ignore, entries.as_bytes()).await?;
            }
            let deadline = self
                .startup_deadline
                .ok_or_else(|| invalid("Dora runtime has not started"))?;
            let client = self
                .client
                .as_mut()
                .ok_or_else(|| invalid("Dora client missing"))?;
            client
                .start_graph(start, deadline)
                .await
                .map_err(|e| invalid(e.to_string()))?;
            client
                .inspect_graph(deadline)
                .await
                .map_err(|e| invalid(e.to_string()))?;
            Ok(())
        };
        tokio::time::timeout_at(deadline, registration)
            .await
            .map_err(|_| invalid("Dora registration deadline expired"))?
    }
    pub async fn inspect(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(self.startup_seconds);
        let inspection = async {
            self.inspect_runtime().await?;
            if let Some(client) = &mut self.client {
                client
                    .inspect_graph(deadline)
                    .await
                    .map_err(|e| invalid(e.to_string()))?;
            }
            Ok(())
        };
        tokio::time::timeout_at(deadline, inspection)
            .await
            .map_err(|_| invalid("Dora observation deadline expired"))?
    }
    pub(super) fn cleanup_deadline(&mut self) -> Instant {
        *self
            .stop_deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_secs(self.stop_seconds))
    }
    /// Request Dora stop for every system node; Compose then verifies shutdown.
    pub async fn shutdown(&mut self, system: &[String]) -> Result<()> {
        let deadline = self.cleanup_deadline();
        if self
            .client
            .as_ref()
            .is_none_or(|client| client.graph().is_none())
        {
            return Ok(());
        }
        let dataflow = self
            .dora_runtime()?
            .dataflow
            .as_ref()
            .ok_or_else(|| invalid("registered Dora graph missing"))?;
        let bytes = crate::evidence::read_regular(
            std::path::Path::new(&dataflow.source),
            super::shared_dora::MAX_GRAPH_BYTES,
        )
        .await?;
        let graph = workload_lifecycle::dora::graph(&dataflow.source, &bytes).map_err(invalid)?;
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
                let grace = Duration::from_secs(self.stop_seconds.saturating_sub(1).max(1));
                self.client
                    .as_mut()
                    .expect("retained graph client")
                    .stop_node(node.id.clone(), grace, deadline)
                    .await
                    .map_err(|e| invalid(e.to_string()))?;
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
