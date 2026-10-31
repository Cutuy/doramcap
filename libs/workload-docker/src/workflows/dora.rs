//! Workflow-owned Dora lifecycle through its supported CLI and owned Compose services.
//! Commands address Dora node IDs; Compose retains ownership of foreground infrastructure.
use super::system_apps::DoraFlow;
use crate::compose::{
    DockerAdapter, ResolvedComposeProject, Result,
    adapter::{private_write, validate_name},
    error::invalid,
};
use crate::evidence::read_regular;
use futures_util::future::join_all;
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

/// Prepared binding and the coordinator's UUID, populated by `StartDoraFlow`.
/// Checkpoints live beside the private resolved Compose manifest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flow {
    pub plan: ResolvedComposeProject,
    pub config: DoraFlow,
    pub id: String,
}
impl Flow {
    fn checkpoint(&self, suffix: &str) -> std::path::PathBuf {
        self.plan
            .manifest
            .parent()
            .unwrap()
            .join(format!("{}.dora-{suffix}.json", self.plan.key))
    }
    async fn command(&self, adapter: &DockerAdapter, args: &[&str]) -> Result<String> {
        let mut command = vec!["/usr/local/bin/dora".into()];
        command.extend(args.iter().map(|s| (*s).to_owned()));
        adapter
            .exec(
                &self.plan,
                &self.config.service,
                &command,
                Duration::from_secs(35),
            )
            .await
    }
    async fn nodes(&self, adapter: &DockerAdapter) -> Result<Vec<Value>> {
        json_lines(
            &self
                .command(
                    adapter,
                    &["node", "list", "--dataflow", &self.id, "--format", "json"],
                )
                .await?,
        )
    }
    async fn lookup(&self, adapter: &DockerAdapter) -> Result<Option<Value>> {
        Ok(
            json_lines(&self.command(adapter, &["list", "--format", "json"]).await?)?
                .into_iter()
                .find(|v| {
                    if self.id.is_empty() {
                        v["name"] == self.config.name
                    } else {
                        v["uuid"] == self.id
                    }
                }),
        )
    }
    async fn fetch(&self, adapter: &DockerAdapter, url: &str) -> Result<String> {
        adapter
            .exec(
                &self.plan,
                &self.config.service,
                &[
                    "curl".into(),
                    "--fail".into(),
                    "--silent".into(),
                    "--max-time".into(),
                    "3".into(),
                    url.into(),
                ],
                Duration::from_secs(5),
            )
            .await
    }
}
fn json_lines(text: &str) -> Result<Vec<Value>> {
    text.lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| Ok(serde_json::from_str(s)?))
        .collect()
}
/// Validate operator configuration before launch. Results paths are filled by the
/// run owner; readiness/snapshot URLs stay on the container's loopback interface.
pub fn validate(config: &DoraFlow) -> Result<()> {
    for name in [&config.project, &config.service, &config.name] {
        validate_name(name)?;
    }
    if !Path::new(&config.descriptor).is_absolute()
        || (config.shutdown.is_empty() || config.shutdown.len() > 16)
    {
        return Err(invalid(
            "Dora requires an absolute descriptor and ordered node groups",
        ));
    }
    let mut nodes = HashSet::new();
    for stage in &config.shutdown {
        if stage.nodes.is_empty() {
            return Err(invalid("empty Dora shutdown stage"));
        }
        for node in &stage.nodes {
            validate_name(&node.replace('_', "-"))?;
            if !nodes.insert(node) {
                return Err(invalid("duplicate Dora node in shutdown order"));
            }
        }
    }
    for url in [&config.readiness_url, &config.snapshot_url]
        .into_iter()
        .filter(|s| !s.is_empty())
    {
        let loopback = url
            .strip_prefix("http://127.0.0.1:")
            .and_then(|tail| tail.split_once('/'))
            .is_some_and(|(port, _)| port.parse::<u16>().is_ok_and(|port| port != 0));
        if !loopback {
            return Err(invalid("Dora HTTP probes must use loopback"));
        }
    }
    for file in [
        &config.snapshot_file,
        &config.count_file,
        &config.completion_file,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    {
        if matches!(file.as_str(), "." | "..") {
            return Err(invalid("invalid Dora result filename"));
        }
        validate_name(&file.replace('.', "-"))?;
    }
    if config.snapshot_url.is_empty() != config.snapshot_file.is_empty()
        || config.count_node.is_empty() != config.count_file.is_empty()
        || (!config.count_node.is_empty() && !nodes.contains(&config.count_node))
    {
        return Err(invalid(
            "inconsistent Dora snapshot or collector count policy",
        ));
    }
    if !config.readiness_field.is_empty() && config.readiness_url.is_empty() {
        return Err(invalid("Dora readiness_field requires readiness_url"));
    }
    Ok(())
}

/// Validate a complete run binding before any container or flow is started.
/// Registration loading may omit the results path; the run owner must fill it.
pub(crate) fn validate_runtime(config: &DoraFlow) -> Result<()> {
    validate(config)?;
    if !Path::new(&config.results_directory).is_absolute() {
        return Err(invalid("Dora results_directory must be absolute"));
    }
    Ok(())
}

/// Start a declared flow and verify its nodes and application readiness.
/// Retains a private UUID checkpoint before readiness; callers quiesce the owned
/// Compose project on failure or cancellation.
pub struct StartDoraFlow {
    pub adapter: Rc<DockerAdapter>,
}
/// Verify the expected running node set and optional application HTTP probe.
pub struct InspectDoraFlow {
    pub adapter: Rc<DockerAdapter>,
}
/// Stop node groups in order, await draining consumers, then finalize declared
/// snapshots/counts and completion markers. Compose teardown remains the caller’s job.
pub struct StopDoraFlow {
    pub adapter: Rc<DockerAdapter>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartInput {
    pub flow: Flow,
    pub timeout_seconds: u64,
}

pub(crate) fn register(executor: &mut LocalExecutor, adapter: Rc<DockerAdapter>) {
    let a = adapter.clone();
    executor.register::<StartDoraFlow>(move || StartDoraFlow { adapter: a.clone() });
    let a = adapter.clone();
    executor.register::<InspectDoraFlow>(move || InspectDoraFlow { adapter: a.clone() });
    executor.register::<StopDoraFlow>(move || StopDoraFlow {
        adapter: adapter.clone(),
    });
}
impl Step for StartDoraFlow {
    type Input = StartInput;
    type Output = Flow;
    async fn run(&mut self, input: StartInput) -> orchestrator::Result<Flow> {
        start(&self.adapter, input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
/// Bind each declared flow to exactly one resolved project and service before launch.
pub(super) fn bind(
    configs: &[super::system_apps::DoraFlow],
    plans: &[ResolvedComposeProject],
) -> Result<Vec<Flow>> {
    let mut projects = HashSet::new();
    configs
        .iter()
        .map(|config| {
            validate_runtime(config)?;
            let plan = plans
                .iter()
                .find(|p| p.key == config.project && p.services.contains_key(&config.service))
                .filter(|_| projects.insert(&config.project))
                .ok_or_else(|| {
                    invalid("Dora requires one binding to an existing project and service")
                })?;
            Ok(Flow {
                plan: plan.clone(),
                config: config.clone(),
                id: String::new(),
            })
        })
        .collect()
}

async fn start(adapter: &DockerAdapter, input: StartInput) -> Result<Flow> {
    let mut flow = input.flow;
    validate_runtime(&flow.config)?;
    let deadline = Instant::now() + Duration::from_secs(input.timeout_seconds);
    loop {
        if let Some(state) = flow.lookup(adapter).await.ok().flatten() {
            flow.id = state["uuid"]
                .as_str()
                .ok_or_else(|| invalid("Dora omitted flow UUID"))?
                .into();
            break;
        }
        let launch = flow
            .command(
                adapter,
                &[
                    "start",
                    &flow.config.descriptor,
                    "--name",
                    &flow.config.name,
                    "--detach",
                ],
            )
            .await;
        if Instant::now() >= deadline {
            return Err(launch
                .err()
                .unwrap_or_else(|| invalid("Dora launch timed out")));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    private_write(&flow.checkpoint("started"), &serde_json::to_vec(&flow)?).await?;
    loop {
        if inspect(adapter, &flow).await.is_ok() {
            let synchronized = if flow.config.readiness_field.is_empty() {
                true
            } else {
                let body = flow.fetch(adapter, &flow.config.readiness_url).await?;
                serde_json::from_str::<Value>(&body)?[&flow.config.readiness_field] == true
            };
            if synchronized {
                return Ok(flow);
            }
        }
        if flow
            .lookup(adapter)
            .await?
            .is_some_and(|s| matches!(s["status"].as_str(), Some("Failed" | "Finished")))
        {
            return Err(invalid("Dora flow exited before application readiness"));
        }
        if Instant::now() >= deadline {
            return Err(invalid("Dora nodes or application did not become ready"));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
impl Step for InspectDoraFlow {
    type Input = Flow;
    type Output = ();
    async fn run(&mut self, flow: Flow) -> orchestrator::Result<()> {
        inspect(&self.adapter, &flow)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
async fn inspect(adapter: &DockerAdapter, flow: &Flow) -> Result<()> {
    let nodes = flow.nodes(adapter).await?;
    let expected: HashSet<_> = flow.config.shutdown.iter().flat_map(|s| &s.nodes).collect();
    if nodes.len() != expected.len()
        || nodes.iter().any(|n| {
            n["status"] != "Running"
                || n["node"]
                    .as_str()
                    .is_none_or(|id| !expected.iter().any(|name| name.as_str() == id))
        })
    {
        return Err(invalid("Dora node exited or failed"));
    }
    if !flow.config.readiness_url.is_empty() {
        // Synchronization can temporarily become incomplete while a healthy
        // dashboard reconciles data; the JSON readiness field gates startup only.
        flow.fetch(adapter, &flow.config.readiness_url).await?;
    }
    Ok(())
}
impl Step for StopDoraFlow {
    type Input = Flow;
    type Output = ();
    async fn run(&mut self, flow: Flow) -> orchestrator::Result<()> {
        stop(&self.adapter, flow)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
async fn write_result(path: &Path, bytes: &[u8]) -> Result<()> {
    match private_write(path, bytes).await {
        Err(crate::compose::Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_regular(path, 16 * 1024 * 1024).await? == bytes {
                Ok(())
            } else {
                Err(invalid("Dora result changed during finalization"))
            }
        }
        result => result,
    }
}
async fn stop(adapter: &DockerAdapter, mut flow: Flow) -> Result<()> {
    validate(&flow.config)?;
    if flow.id.is_empty() {
        if let Some(state) = flow.lookup(adapter).await? {
            flow.id = state["uuid"]
                .as_str()
                .ok_or_else(|| invalid("Dora omitted UUID"))?
                .into();
        } else {
            return Ok(());
        } // No flow launched before startup was cancelled.
    }
    match read_regular(&flow.checkpoint("complete"), 1024).await {
        Ok(bytes) if bytes == flow.id.as_bytes() => return Ok(()),
        Ok(_) => return Err(invalid("Dora completion identity changed")),
        Err(crate::compose::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let results = Path::new(&flow.config.results_directory);
    if !results.is_absolute() {
        return Err(invalid("Dora results directory must be absolute"));
    }
    if !flow.config.snapshot_url.is_empty() {
        let snapshot = flow.fetch(adapter, &flow.config.snapshot_url).await?;
        write_result(
            &results.join(&flow.config.snapshot_file),
            snapshot.as_bytes(),
        )
        .await?;
    }
    for stage in &flow.config.shutdown {
        let nodes = flow.nodes(adapter).await?;
        if !stage.drain {
            let stops = join_all(
                stage
                    .nodes
                    .iter()
                    .filter(|id| {
                        nodes
                            .iter()
                            .any(|n| n["node"] == id.as_str() && n["status"] == "Running")
                    })
                    .map(|node| async {
                        flow.command(
                            adapter,
                            &[
                                "node",
                                "stop",
                                node,
                                "--dataflow",
                                &flow.id,
                                "--grace",
                                "12s",
                            ],
                        )
                        .await
                    }),
            )
            .await;
            for result in stops {
                result?;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let nodes = flow.nodes(adapter).await?;
            if !nodes.iter().any(|n| {
                stage.nodes.iter().any(|id| n["node"] == id.as_str()) && n["status"] == "Running"
            }) {
                break;
            }
            if Instant::now() >= deadline {
                return Err(invalid("Dora node did not drain before deadline"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = flow
            .lookup(adapter)
            .await?
            .ok_or_else(|| invalid("Dora flow status disappeared"))?;
        if state["status"] == "Finished" {
            break;
        }
        if state["status"] == "Failed" || Instant::now() >= deadline {
            return Err(invalid(format!(
                "Dora flow did not finish cleanly: {}",
                state["status"]
            )));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if !flow.config.count_node.is_empty() {
        let logs = flow
            .command(
                adapter,
                &[
                    "logs",
                    &flow.id,
                    "--node",
                    &flow.config.count_node,
                    "--tail",
                    "20",
                    "--log-format",
                    "json",
                ],
            )
            .await?;
        let count = published_count(&logs)?;
        write_result(
            &results.join(&flow.config.count_file),
            format!("{count}\n").as_bytes(),
        )
        .await?;
    }
    if !flow.config.completion_file.is_empty() {
        write_result(&results.join(&flow.config.completion_file), b"complete\n").await?;
    }
    private_write(&flow.checkpoint("complete"), flow.id.as_bytes()).await
}
fn published_count(logs: &str) -> Result<u64> {
    let messages = json_lines(logs)?;
    let message = messages
        .iter()
        .rev()
        .filter_map(|v| v["message"].as_str())
        .find_map(|s| s.strip_prefix("session complete: "))
        .ok_or_else(|| invalid("collector did not report complete counts"))?;
    let counts: Vec<u64> = message
        .split_whitespace()
        .zip(["collected=", "published=", "dropped="])
        .map(|(s, key)| {
            s.strip_prefix(key)
                .ok_or_else(|| invalid("invalid collector count"))?
                .parse()
                .map_err(|_| invalid("invalid collector count"))
        })
        .collect::<Result<_>>()?;
    if counts.len() != 3 || counts[0] != counts[1] || counts[2] != 0 {
        return Err(invalid("collector lost samples"));
    }
    Ok(counts[1])
}
/// Recover workflow-owned Dora shutdown before the surrounding Compose cleanup.
/// Missing checkpoints mean no managed flow started; incomplete shutdown is an
/// error retained by the caller while Docker teardown still proceeds.
pub(crate) async fn recover(adapter: &DockerAdapter, plan: &ResolvedComposeProject) -> Result<()> {
    let path = plan
        .manifest
        .parent()
        .unwrap()
        .join(format!("{}.dora-started.json", plan.key));
    let bytes = match read_regular(&path, 2 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(crate::compose::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let mut flow: Flow = serde_json::from_slice(&bytes)?;
    if flow.plan.run != plan.run || flow.plan.key != plan.key || flow.plan.sha256 != plan.sha256 {
        return Err(invalid("Dora checkpoint ownership changed"));
    }
    flow.plan = plan.clone();
    tokio::time::timeout(Duration::from_secs(60), stop(adapter, flow))
        .await
        .map_err(|_| invalid("Dora recovery shutdown deadline exceeded"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::system_apps::DoraStopStage;
    #[test]
    fn shutdown_policy_rejects_duplicate_nodes_and_result_escapes() {
        // Arrange
        let mut config = DoraFlow {
            project: "app".into(),
            service: "daemon".into(),
            name: "flow".into(),
            descriptor: "/opt/flow.yml".into(),
            completion_file: "shutdown.complete".into(),
            shutdown: vec![DoraStopStage {
                nodes: vec!["source_a".into()],
                drain: false,
            }],
            ..Default::default()
        };
        // Act / Assert
        assert!(validate(&config).is_ok());
        config.shutdown.push(config.shutdown[0].clone());
        assert!(validate(&config).is_err());
        config.shutdown.pop();
        config.completion_file = "../escape".into();
        assert!(validate(&config).is_err());
        config.completion_file = "..".into();
        assert!(validate(&config).is_err());
        config.completion_file = "shutdown.complete".into();
        config.readiness_url = "http://127.0.0.1:80@outside.example/state".into();
        assert!(validate(&config).is_err());

        // Act: a JSON readiness field has no meaning without a probe URL.
        config.readiness_url.clear();
        config.readiness_field = "ready".into();

        // Assert
        assert!(validate(&config).is_err());
    }
    #[test]
    fn final_collector_summary_requires_complete_lossless_delivery() {
        // Arrange
        let log = |summary| serde_json::json!({"message":summary}).to_string();
        // Act / Assert
        assert_eq!(
            published_count(&log(
                "session complete: collected=42 published=42 dropped=0"
            ))
            .unwrap(),
            42
        );
        assert!(
            published_count(&log(
                "session complete: collected=42 published=41 dropped=1"
            ))
            .is_err()
        );
        assert!(published_count(&log("session complete: collected=42 published=42")).is_err());
        assert!(published_count(&log("unrelated log")).is_err());
    }
}
