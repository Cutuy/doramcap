//! Runtime operations bound to one frozen Compose project.
use super::{
    DockerAdapter, EXPERIMENT_OWNER_LABEL, RUN_LABEL, digest, private_write, validate_name,
};
use crate::compose::{
    Cleanup, ComposeProjectState, Container, ResolvedComposeProject, Result, Shutdown,
    command::compose,
    error::{absent, engine, invalid},
};
use bollard::query_parameters::*;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Borrows one frozen plan and its shared Docker connection for inspection and routing.
/// Workflow steps control startup and teardown; dropping this view changes no resources.
pub struct ComposeExecution<'a> {
    pub(super) adapter: &'a DockerAdapter,
    pub(super) plan: &'a ResolvedComposeProject,
}
impl ComposeExecution<'_> {
    /// Validate a generated execution manifest with Compose before any launch.
    /// Checks its frozen digest and returns CLI diagnostics on schema failure.
    /// Creates no Engine resources; requires the artifact parent to exist.
    pub(crate) async fn validate_execution(&self) -> Result<()> {
        self.verify_manifest().await?;
        let mut args = self.args();
        args.extend(["config".into(), "--quiet".into()]);
        compose(
            &self.adapter.socket,
            Path::new(&self.plan.manifest)
                .parent()
                .ok_or_else(|| invalid("manifest parent missing"))?,
            &BTreeMap::new(),
            &args,
            Duration::from_secs(30),
        )
        .await?;
        Ok(())
    }
    async fn verify_manifest(&self) -> Result<()> {
        if digest(&tokio::fs::read(&self.plan.manifest).await?) != self.plan.sha256 {
            return Err(invalid("frozen Compose manifest changed"));
        }
        Ok(())
    }
    pub(super) fn args(&self) -> Vec<String> {
        vec![
            "--env-file".into(),
            "/dev/null".into(),
            "--project-name".into(),
            self.plan.name.clone(),
            "-f".into(),
            Path::new(&self.plan.manifest)
                .to_string_lossy()
                .into_owned(),
        ]
    }
    pub(super) async fn containers(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        self.adapter
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: project_filters(&self.plan.name),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("list project containers", e))
    }
    pub(super) async fn networks(&self) -> Result<Vec<bollard::models::Network>> {
        self.adapter
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: project_filters(&self.plan.name),
            }))
            .await
            .map_err(|e| engine("list project networks", e))
    }
    pub(super) async fn volumes(&self) -> Result<Vec<bollard::models::Volume>> {
        self.adapter
            .docker
            .list_volumes(Some(ListVolumesOptions {
                filters: Some(HashMap::from([(
                    "label".into(),
                    vec![
                        format!("{PROJECT_LABEL}={}", self.plan.name),
                        format!("{RUN_LABEL}={}", self.plan.run),
                    ],
                )])),
            }))
            .await
            .map(|volumes| volumes.volumes.unwrap_or_default())
            .map_err(|e| engine("list project volumes", e))
    }
    fn owned(&self, summary: &bollard::models::ContainerSummary) -> bool {
        summary.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&self.plan.run)
    }
    /// Inspect this project's containers, rejecting changed ownership or service membership.
    /// Returns identities, PIDs, health and exit state; concurrent changes can race inspection.
    pub async fn inspect(&self) -> Result<ComposeProjectState> {
        let mut containers = Vec::new();
        for summary in self.containers().await? {
            if !self.owned(&summary) {
                return Err(invalid(format!("unowned container in {}", self.plan.name)));
            }
            let id = summary
                .id
                .ok_or_else(|| invalid("Engine omitted container ID"))?;
            let info = self
                .adapter
                .docker
                .inspect_container(&id, None)
                .await
                .map_err(|e| engine(format!("inspect {id}"), e))?;
            let labels = info
                .config
                .as_ref()
                .and_then(|c| c.labels.as_ref())
                .ok_or_else(|| invalid("missing labels"))?;
            if labels.get(RUN_LABEL) != Some(&self.plan.run)
                || labels.get(PROJECT_LABEL) != Some(&self.plan.name)
            {
                return Err(invalid("ownership changed during inspection"));
            }
            let service = labels
                .get("com.docker.compose.service")
                .ok_or_else(|| invalid("missing Compose service label"))?
                .clone();
            if !self.plan.services.contains_key(&service) {
                return Err(invalid(format!("unexpected service {service}")));
            }
            let state = info
                .state
                .ok_or_else(|| invalid("Engine omitted container state"))?;
            containers.push(Container {
                id,
                service,
                image_id: info.image.unwrap_or_default(),
                pid: state.pid.unwrap_or(0),
                status: state.status.map(|s| s.to_string()).unwrap_or_default(),
                health: state.health.and_then(|h| h.status).map(|s| s.to_string()),
                exit_code: state.exit_code,
                restart_count: info.restart_count.unwrap_or(0),
            });
        }
        Ok(ComposeProjectState {
            plan: Some(self.plan.clone()),
            containers,
        })
    }
    /// Start selected services and wait for the requested readiness budget.
    /// Errors or cancellation can leave resources; quiesce joins pending CLI startup.
    pub(crate) async fn start(
        &self,
        services: &[String],
        seconds: u64,
    ) -> Result<ComposeProjectState> {
        self.verify_manifest().await?;
        let mut args = self.args();
        args.extend(["up", "--detach", "--no-build", "--pull", "never"].map(str::to_owned));
        if !services.is_empty() {
            if services
                .iter()
                .any(|service| !self.plan.services.contains_key(service))
            {
                return Err(invalid("unknown selected Compose service"));
            }
            args.push("--no-recreate".into());
            args.extend(services.iter().cloned());
        }
        self.command(args, seconds).await?;
        let deadline = Instant::now() + Duration::from_secs(seconds);
        loop {
            let state = self.inspect().await?;
            let mut selected = state.clone();
            if !services.is_empty() {
                selected
                    .plan
                    .as_mut()
                    .ok_or_else(|| invalid("Compose state plan missing"))?
                    .services
                    .retain(|name, _| services.contains(name));
            }
            if selected.ready()? {
                return Ok(state);
            }
            if Instant::now() >= deadline {
                return Err(invalid(format!("{} readiness timeout", self.plan.name)));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    /// Create the named IPC owner without starting it, before workload side effects.
    /// Native Compose creates its pinned configuration; Docker rejects another
    /// experiment claiming the machine-wide experiment container name.
    pub(crate) async fn reserve(&self, service: &str, seconds: u64) -> Result<()> {
        self.verify_manifest().await?;
        let mut args = self.args();
        args.extend(
            [
                "create",
                "--no-build",
                "--pull",
                "never",
                "--no-recreate",
                service,
            ]
            .map(str::to_owned),
        );
        self.command(args, seconds).await
    }
    pub(super) async fn command(&self, args: Vec<String>, seconds: u64) -> Result<()> {
        let log = Path::new(&self.plan.manifest).with_extension(format!(
            "launch-{}.log",
            &digest(&serde_json::to_vec(&args)?)[..16]
        ));
        let socket = self.adapter.socket.clone();
        let directory = Path::new(&self.plan.manifest)
            .parent()
            .unwrap()
            .to_path_buf();
        let budget = Duration::from_secs(seconds);
        let mut starting = self.adapter.starting.lock().await;
        if starting.contains_key(&self.plan.name) {
            return Err(invalid(
                "project has an unfinished startup; quiesce it first",
            ));
        }
        starting.insert(
            self.plan.name.clone(),
            Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
                compose(&socket, &directory, &BTreeMap::new(), &args, budget).await
            })))),
        );
        drop(starting);
        let (out, err) = self
            .finish_start()
            .await?
            .ok_or_else(|| invalid("startup command already consumed"))?;
        private_write(&log, format!("{out}\n{err}").as_bytes()).await
    }
    // Keep the native up request alive across cancellation; shutdown joins it
    // before inspecting resources, so delayed container creation cannot escape cleanup.
    pub(super) async fn finish_start(&self) -> Result<Option<(String, String)>> {
        let command = self
            .adapter
            .starting
            .lock()
            .await
            .get(&self.plan.name)
            .cloned();
        let Some(command) = command else {
            return Ok(None);
        };
        let mut command = command.lock().await;
        let Some(pending) = command.as_mut() else {
            return Ok(None);
        };
        let result = pending.await;
        command.take();
        self.adapter.starting.lock().await.remove(&self.plan.name);
        result
            .map_err(|e| invalid(format!("Compose startup task: {e}")))?
            .map(Some)
    }
    /// Execute a command in one owned service; cancellation terminates the CLI client.
    pub(crate) async fn exec(
        &self,
        service: &str,
        command: &[String],
        timeout: Duration,
    ) -> Result<String> {
        self.verify_manifest().await?;
        Self::running_service(self.inspect().await?, service)?;
        let mut args = self.args();
        args.extend(["exec".into(), "--no-TTY".into(), service.into()]);
        args.extend_from_slice(command);
        compose(
            &self.adapter.socket,
            Path::new(&self.plan.manifest).parent().unwrap(),
            &BTreeMap::new(),
            &args,
            timeout,
        )
        .await
        .map(|(out, _)| out)
    }
    // Share image, health and replica checks between endpoint routing and command execution.
    fn running_service(mut state: ComposeProjectState, service: &str) -> Result<Container> {
        let plan = state
            .plan
            .as_mut()
            .ok_or_else(|| invalid("Compose state plan missing"))?;
        plan.services.retain(|name, _| name == service);
        let empty = plan.services.is_empty();
        if empty || !state.ready()? {
            return Err(invalid("service is not ready"));
        }
        let mut containers = state
            .containers
            .into_iter()
            .filter(|c| c.service == service);
        match (containers.next(), containers.next()) {
            (Some(container), None) if container.pid > 0 && container.status == "running" => {
                Ok(container)
            }
            _ => Err(invalid("expected one running service container")),
        }
    }
    /// Resolve one ready service's endpoint on its Compose network or host listener.
    /// Missing, unhealthy or unowned resources fail; callers must stop routing after shutdown.
    pub async fn endpoint(
        &self,
        service: &str,
        network: &str,
        port: u16,
    ) -> Result<std::net::SocketAddr> {
        validate_name(network)?;
        if port == 0 {
            return Err(invalid("endpoint port must be positive"));
        }
        let container = Self::running_service(self.inspect().await?, service)?;
        let info = self
            .adapter
            .docker
            .inspect_container(&container.id, None)
            .await
            .map_err(|e| engine("inspect endpoint container", e))?;
        if info
            .host_config
            .as_ref()
            .and_then(|config| config.network_mode.as_deref())
            == Some("host")
        {
            crate::compose::command::verify_listener(
                container
                    .pid
                    .try_into()
                    .map_err(|_| invalid("invalid endpoint PID"))?,
                port,
            )
            .await?;
            let current = self
                .adapter
                .docker
                .inspect_container(&container.id, None)
                .await
                .map_err(|e| engine("recheck endpoint container", e))?;
            if !current.state.is_some_and(|state| {
                state.running == Some(true) && state.pid == Some(container.pid)
            }) {
                return Err(invalid(
                    "endpoint container changed while checking its listener",
                ));
            }
            return Ok((std::net::Ipv4Addr::LOCALHOST, port).into());
        }
        let network_name = format!("{}_{}", self.plan.name, network);
        let networks = info
            .network_settings
            .and_then(|n| n.networks)
            .ok_or_else(|| invalid("container has no networks"))?;
        let endpoint = networks
            .get(&network_name)
            .ok_or_else(|| invalid("owned endpoint network missing"))?;
        let network_id = endpoint
            .network_id
            .as_deref()
            .ok_or_else(|| invalid("endpoint has no network ID"))?;
        let network_info = self
            .adapter
            .docker
            .inspect_network(network_id, None::<InspectNetworkOptions>)
            .await
            .map_err(|e| engine("inspect endpoint network", e))?;
        let labels = network_info.labels.unwrap_or_default();
        if labels.get(RUN_LABEL) != Some(&self.plan.run)
            || labels.get(PROJECT_LABEL) != Some(&self.plan.name)
        {
            return Err(invalid("endpoint network is not owned by the project"));
        }
        let ip: std::net::Ipv4Addr = endpoint
            .ip_address
            .as_deref()
            .ok_or_else(|| invalid("endpoint has no IPv4 address"))?
            .parse()
            .map_err(|_| invalid("invalid endpoint IPv4 address"))?;
        if ip.is_unspecified() || ip.is_loopback() {
            return Err(invalid("invalid endpoint network address"));
        }
        Ok(std::net::SocketAddr::new(ip.into(), port))
    }
    /// Quiesce the whole project, then remove its owned resources.
    /// Retain diagnostics and inspect `verified`; cancellation requires a retry.
    pub(crate) async fn stop(&self, seconds: u64, allow_volume_deletion: bool) -> Result<Cleanup> {
        let stopped = self.quiesce(&[], seconds).await?;
        if !stopped.stopped {
            return Err(invalid("project failed to quiesce"));
        }
        let mut result = self.remove(allow_volume_deletion).await?;
        result.diagnostics.extend(stopped.diagnostics);
        Ok(result)
    }
    /// Stop selected writers, retaining containers and volumes for evidence collection.
    /// `stopped` confirms process termination; diagnostics retain unsuccessful exits.
    pub(crate) async fn quiesce(&self, services: &[String], seconds: u64) -> Result<Shutdown> {
        let mut result = Shutdown {
            project: self.plan.name.clone(),
            ..Default::default()
        };
        match self.finish_start().await {
            Ok(Some((out, err))) => {
                if let Err(error) = private_write(
                    &Path::new(&self.plan.manifest).with_extension("launch-interrupted.log"),
                    format!("{out}\n{err}").as_bytes(),
                )
                .await
                {
                    result
                        .diagnostics
                        .push(error.diagnostic("startup evidence"));
                }
            }
            Ok(None) => {}
            Err(error) => result.diagnostics.push(error.diagnostic("pending startup")),
        }
        let mut before = self.containers().await?;
        let all_owned = before.iter().all(|c| self.owned(c));
        // Preserve dependency-order stop semantics through Compose, with a bounded
        // grace period. Fall back to per-container Engine operations on CLI error.
        let mut args = self.args();
        args.extend(["stop".into(), "--timeout".into(), seconds.to_string()]);
        if !services.is_empty() {
            if services
                .iter()
                .any(|name| !self.plan.services.contains_key(name))
            {
                return Err(invalid("unknown selected Compose service"));
            }
            args.extend(services.iter().cloned());
            before.retain(|c| {
                c.labels
                    .as_ref()
                    .and_then(|labels| labels.get("com.docker.compose.service"))
                    .is_some_and(|name| services.contains(name))
            });
        }
        if all_owned && self.verify_manifest().await.is_ok() {
            if let Err(e) = compose(
                &self.adapter.socket,
                Path::new(&self.plan.manifest).parent().unwrap(),
                &BTreeMap::new(),
                &args,
                Duration::from_secs(seconds.saturating_mul(before.len() as u64 + 1) + 10),
            )
            .await
            {
                result.diagnostics.push(e.diagnostic("compose stop"));
            }
        } else {
            result.diagnostics.push(
                invalid(
                    "manifest or project ownership changed; restricted to owned Engine objects",
                )
                .diagnostic("stop"),
            );
        }
        for summary in before {
            if !self.owned(&summary) {
                continue;
            }
            let Some(id) = summary.id else {
                continue;
            };
            match self.adapter.docker.inspect_container(&id, None).await {
                Ok(info) => {
                    if info.state.as_ref().and_then(|s| s.running) == Some(true)
                        && let Err(e) = self
                            .adapter
                            .docker
                            .stop_container(
                                &id,
                                Some(StopContainerOptions {
                                    t: Some(seconds.min(i32::MAX as u64) as i32),
                                    ..Default::default()
                                }),
                            )
                            .await
                        && !absent(&e)
                    {
                        result
                            .diagnostics
                            .push(engine(format!("stop {id}"), e).diagnostic("stop"));
                    }
                    match self.adapter.docker.inspect_container(&id, None).await {
                        Ok(stopped) => {
                            if stopped.state.as_ref().is_some_and(|s| {
                                s.running == Some(true)
                                    || s.pid.unwrap_or(0) != 0
                                    || s.exit_code == Some(137)
                                    || s.oom_killed == Some(true)
                                    || (self.plan.key == "system" && s.exit_code != Some(0))
                            }) {
                                result.diagnostics.push(
                                    invalid(format!("{id}: forced/unclean shutdown"))
                                        .diagnostic("stop"),
                                );
                            }
                            if let Err(e) = save_stopped_inspection(
                                Path::new(&self.plan.manifest).parent().unwrap(),
                                &self.plan.key,
                                &id,
                                serde_json::to_vec_pretty(&stopped)?,
                            )
                            .await
                            {
                                result.diagnostics.push(e.diagnostic("save inspection"));
                            }
                        }
                        Err(e) if absent(&e) => {}
                        Err(e) => result
                            .diagnostics
                            .push(engine(format!("inspect stopped {id}"), e).diagnostic("stop")),
                    }
                }
                Err(e) if absent(&e) => {}
                Err(e) => result
                    .diagnostics
                    .push(engine(format!("inspect {id}"), e).diagnostic("stop")),
            }
        }
        result.stopped = self
            .inspect()
            .await?
            .containers
            .iter()
            .filter(|c| services.is_empty() || services.contains(&c.service))
            .all(|c| c.pid == 0 && matches!(c.status.as_str(), "exited" | "created"));
        Ok(result)
    }
    /// Remove stopped owned resources after the caller saves their file-change report.
    /// Volume deletion requires explicit permission and a matching retention policy.
    /// Inspect `verified` and diagnostics; cancellation requires reconciliation.
    pub(crate) async fn remove(&self, allow_volume_deletion: bool) -> Result<Cleanup> {
        self.finish_start().await?;
        let state = self.inspect().await?;
        if state
            .containers
            .iter()
            .any(|c| c.pid != 0 || !matches!(c.status.as_str(), "exited" | "created"))
        {
            return Err(invalid("refusing to remove a project with live containers"));
        }
        let mut result = Cleanup {
            project: self.plan.name.clone(),
            ..Default::default()
        };
        let mut experiment_owner = None;
        // Keep the stopped IPC owner until all other owned resources are removed.
        for summary in self.containers().await? {
            if summary
                .labels
                .as_ref()
                .and_then(|labels| labels.get(EXPERIMENT_OWNER_LABEL))
                .is_some()
            {
                if self.owned(&summary) {
                    experiment_owner = summary.id;
                }
                continue;
            }
            if self.owned(&summary)
                && let Some(id) = summary.id
                && let Err(e) = self
                    .adapter
                    .docker
                    .remove_container(
                        &id,
                        Some(RemoveContainerOptions {
                            force: false,
                            v: true,
                            ..Default::default()
                        }),
                    )
                    .await
                && !absent(&e)
            {
                result
                    .diagnostics
                    .push(engine(format!("remove {id}"), e).diagnostic("remove"));
            }
        }
        let networks = self.networks().await?;
        for network in networks {
            if let Some(id) = network.id {
                // Compose-scoped names were validated before launch; unknown names
                // require explicit reconciliation rather than destructive cleanup.
                if network.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&self.plan.run) {
                    if let Err(e) = self.adapter.docker.remove_network(&id).await
                        && !absent(&e)
                    {
                        result
                            .diagnostics
                            .push(engine("remove network", e).diagnostic("cleanup"));
                    }
                } else {
                    result.remaining.push(format!("network:{id}"));
                }
            }
        }
        for volume in self.volumes().await? {
            if volume.labels.get(RUN_LABEL) == Some(&self.plan.run) {
                let remove = allow_volume_deletion
                    && volume
                        .labels
                        .get("io.doramcap.remove-volume")
                        .map_or(self.plan.remove_volumes, |v| v == "true");
                if !remove {
                    result.retained_volumes.push(volume.name);
                    continue;
                }
                if let Err(e) = self
                    .adapter
                    .docker
                    .remove_volume(&volume.name, None::<RemoveVolumeOptions>)
                    .await
                    && !absent(&e)
                {
                    result
                        .diagnostics
                        .push(engine("remove volume", e).diagnostic("cleanup"));
                    result.remaining.push(format!("volume:{}", volume.name));
                }
            } else {
                result.remaining.push(format!("volume:{}", volume.name));
            }
        }
        result.remaining.extend(
            self.networks()
                .await?
                .into_iter()
                .filter_map(|n| n.id)
                .map(|id| format!("network:{id}")),
        );
        result.remaining.extend(
            self.containers()
                .await?
                .into_iter()
                .filter(|c| c.id != experiment_owner)
                .map(|c| format!("container:{}", c.id.unwrap_or_default())),
        );
        if result.remaining.is_empty()
            && result.diagnostics.is_empty()
            && let Some(id) = experiment_owner
            && let Err(error) = self
                .adapter
                .docker
                .remove_container(&id, None::<RemoveContainerOptions>)
                .await
            && !absent(&error)
        {
            result
                .diagnostics
                .push(engine("release experiment owner", error).diagnostic("cleanup"));
        }
        result.remaining.extend(
            self.containers()
                .await?
                .into_iter()
                .filter(|c| {
                    c.labels
                        .as_ref()
                        .is_some_and(|l| l.contains_key(EXPERIMENT_OWNER_LABEL))
                })
                .map(|c| format!("container:{}", c.id.unwrap_or_default())),
        );
        result.verified = result.remaining.is_empty();
        Ok(result)
    }
}

pub(super) const PROJECT_LABEL: &str = "com.docker.compose.project";
fn project_filters(project: &str) -> Option<HashMap<String, Vec<String>>> {
    Some(HashMap::from([(
        "label".into(),
        vec![format!("{PROJECT_LABEL}={project}")],
    )]))
}

/// Atomically replace one container's private diagnostic snapshot after a complete write.
/// Write or rename failures preserve the previous snapshot and remove the temporary file.
async fn save_stopped_inspection(
    directory: &Path,
    key: &str,
    id: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    let directory = directory.to_owned();
    let destination = directory.join(format!("{key}-{id}.stopped.json"));
    tokio::task::spawn_blocking(move || -> Result<()> {
        use std::io::Write;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist(destination)
            .map_err(|error| error.error)?;
        Ok(())
    })
    .await
    .map_err(|error| invalid(format!("stopped inspection writer: {error}")))?
}

#[cfg(test)]
mod tests;
