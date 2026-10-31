use super::{
    Cleanup, ComposeProject, ComposeProjectState, ComposeService, Container, OwnedResource,
    ResolveInput, ResolvedComposeProject, Result, Shutdown, StartInput, StopInput,
    command::compose,
    error::{absent, engine, invalid},
};
use bollard::{Docker, query_parameters::*};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;

pub(crate) const RUN_LABEL: &str = "io.doramcap.run";
pub(crate) const ARTIFACTS_LABEL: &str = "io.doramcap.artifacts";
pub(crate) const DIRECTORY_OWNER_LABEL: &str = "io.doramcap.directory-owner";
const PROJECT_LABEL: &str = "com.docker.compose.project";
type StartupCommand = tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(String, String)>>>>;

/// Rendered Compose configuration and the inputs used to produce it.
pub(crate) struct NormalizedCompose {
    pub(crate) model: Value,
    pub(crate) compose_version: String,
    pub(crate) input_hashes: BTreeMap<String, String>,
    pub(crate) stderr: String,
}

/// Local Engine and Compose delegate, explicitly injected into project steps.
/// Both interfaces use the same absolute Unix socket. Calls retain Engine error
/// status/body; caller-owned artifacts retain bounded Compose diagnostics.
///
/// # Ownership and concurrency
///
/// Clones share Engine connectivity. Methods borrow the adapter and own their
/// request values; artifact files and Docker resources survive those calls.
/// There is no per-project lock. The caller must serialize conflicting lifecycle
/// operations and arrange cleanup after cancellation or partial failure.
#[derive(Clone)]
pub struct DockerAdapter {
    pub(crate) docker: Docker,
    socket: String,
    starting: Arc<tokio::sync::Mutex<BTreeMap<String, Arc<StartupCommand>>>>,
}
impl DockerAdapter {
    /// Return the configured Unix socket path for trusted deployment wiring.
    pub fn socket_path(&self) -> &Path {
        Path::new(&self.socket)
    }

    /// Connect to `socket` and negotiate the Engine API version.
    ///
    /// Returns an adapter using the absolute Unix socket for both Engine and CLI
    /// operations. Engine requests use a 120-second timeout. Relative paths return
    /// a validation error; connection/negotiation failures retain Engine details.
    pub async fn connect(socket: impl Into<String>) -> Result<Self> {
        let socket = socket.into();
        if !Path::new(&socket).is_absolute() {
            return Err(invalid("Docker socket must be absolute"));
        }
        let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| engine("connect", e))?
            .negotiate_version()
            .await
            .map_err(|e| engine("negotiate", e))?;
        Ok(Self {
            docker,
            socket,
            starting: Default::default(),
        })
    }

    /// Require a local socket peer sharing the caller's mount namespace.
    /// Overlay binds must resolve in the daemon's namespace. Different namespaces
    /// fail here. This requires a direct local socket; peer namespace comparison
    /// does not establish the destination behind a proxy.
    pub async fn verify_overlay_namespace(&self) -> Result<()> {
        use std::os::fd::AsRawFd;
        let stream = tokio::net::UnixStream::connect(&self.socket).await?;
        let mut cred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: destination and length point to correctly sized live storage.
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut len,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let ours = tokio::fs::read_link("/proc/self/ns/mnt").await?;
        let theirs = tokio::fs::read_link(format!("/proc/{}/ns/mnt", cred.pid)).await?;
        if ours != theirs {
            return Err(invalid(
                "Docker peer and manager mount namespaces differ; host overlay visibility is unqualified",
            ));
        }
        Ok(())
    }

    /// List containers, networks and volumes bearing a run label with `prefix`.
    ///
    /// Docker list failures or missing resource identities return an error. The
    /// three Engine calls are not an atomic snapshot: callers must prevent other
    /// managers from creating resources concurrently. No resources are modified.
    pub async fn discover_runs(&self, prefix: &str) -> Result<Vec<OwnedResource>> {
        validate_name(prefix)?;
        let filters = Some(HashMap::from([(
            "label".to_owned(),
            vec![RUN_LABEL.to_owned()],
        )]));
        let mut found = Vec::new();
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: filters.clone(),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("discover containers", e))?;
        for item in containers {
            let labels = item.labels.unwrap_or_default();
            if let Some(run) = labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: labels.get(PROJECT_LABEL).cloned(),
                    kind: "container",
                    id: item
                        .id
                        .ok_or_else(|| invalid("discovered container has no ID"))?,
                });
            }
        }
        let networks = self
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: filters.clone(),
            }))
            .await
            .map_err(|e| engine("discover networks", e))?;
        for item in networks {
            let labels = item.labels.unwrap_or_default();
            if let Some(run) = labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: labels.get(PROJECT_LABEL).cloned(),
                    kind: "network",
                    id: item
                        .id
                        .ok_or_else(|| invalid("discovered network has no ID"))?,
                });
            }
        }
        let volumes = self
            .docker
            .list_volumes(Some(ListVolumesOptions { filters }))
            .await
            .map_err(|e| engine("discover volumes", e))?;
        for item in volumes.volumes.unwrap_or_default() {
            if let Some(run) = item.labels.get(RUN_LABEL).filter(|s| s.starts_with(prefix)) {
                found.push(OwnedResource {
                    run: run.clone(),
                    artifacts: item.labels.get(ARTIFACTS_LABEL).map(Into::into),
                    project: item.labels.get(PROJECT_LABEL).cloned(),
                    kind: "volume",
                    id: item.name,
                });
            }
        }
        Ok(found)
    }

    /// Discover profiles for explicit `run` and project identities using resolution
    /// inputs. Invalid identities fail before file or process access; diagnostics stay private.
    pub async fn profiles(&self, run: &str, project: &ComposeProject) -> Result<Vec<String>> {
        validate_name(run)?;
        validate_name(&project.key)?;
        let name = format!("dc-{run}-{}", project.key);
        let mut discovery = project.clone();
        discovery.profiles.clear();
        discovery.services.clear();
        let (directory, mut args, _) = config_arguments(&discovery, &name).await?;
        args.extend(["config".into(), "--profiles".into()]);
        let (stdout, stderr) = compose(
            &self.socket,
            &directory,
            &discovery.environment,
            &args,
            Duration::from_secs(60),
        )
        .await?;
        if stderr.contains("variable is not set") {
            return Err(invalid(format!("unresolved Compose environment: {stderr}")));
        }
        let mut profiles: Vec<_> = stdout
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        profiles.sort();
        profiles.dedup();
        Ok(profiles)
    }

    /// Render Compose inputs and collect the CLI version, source-file hashes and stderr.
    /// No runtime resources or manifests are created. Relative paths use the input directory.
    /// Invalid settings, missing files or Compose failures return errors.
    pub(crate) async fn normalize(&self, input: &ResolveInput) -> Result<NormalizedCompose> {
        validate_name(&input.run)?;
        validate_name(&input.project.key)?;
        let project = &input.project;
        let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
        let name = project_name(&input.run, &project.key, &artifacts);
        let (directory, mut args, sources) = config_arguments(project, &name).await?;
        args.extend(["config".into(), "--format".into(), "json".into()]);
        args.extend(project.services.clone());
        let (stdout, stderr) = compose(
            &self.socket,
            &directory,
            &project.environment,
            &args,
            Duration::from_secs(60),
        )
        .await?;
        if stderr.contains("variable is not set") {
            return Err(invalid(format!("unresolved Compose environment: {stderr}")));
        }
        let model: Value = serde_json::from_str(&stdout)?;
        let (version, _) = compose(
            &self.socket,
            &directory,
            &BTreeMap::new(),
            &["version".into(), "--short".into()],
            Duration::from_secs(10),
        )
        .await?;
        Ok(NormalizedCompose {
            model,
            compose_version: version,
            input_hashes: sources,
            stderr,
        })
    }

    /// Freeze supplied Compose inputs and pinned image IDs into private launch artifacts.
    /// Reject invalid settings, ownership collisions and missing images; partial artifacts may remain.
    /// Host mounts stay live; callers explicitly own container startup and cleanup.
    pub async fn resolve(&self, input: ResolveInput) -> Result<ResolvedComposeProject> {
        let normalized = self.normalize(&input).await?;
        let mut model = normalized.model;
        let project = &input.project;
        let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
        let name = project_name(&input.run, &project.key, &artifacts);
        let manifest = artifacts.join(format!("{}.compose.json", project.key));
        let mut diagnostics = Vec::new();
        let mut policy_changes = Vec::new();
        if !normalized.stderr.trim().is_empty() {
            diagnostics.push(normalized.stderr);
        }
        // Explicit resource names escape Compose project namespacing. External
        // resources are deliberately retained; internal resources must be scoped.
        for kind in ["networks", "volumes"] {
            if let Some(resources) = model.get_mut(kind).and_then(Value::as_object_mut) {
                for (key, resource) in resources {
                    if resource["external"] == true {
                        diagnostics.push(format!(
                            "external {kind}/{key} retained; reset not verified"
                        ));
                    } else if resource["name"]
                        .as_str()
                        .is_some_and(|n| n != format!("{name}_{key}"))
                    {
                        return Err(invalid(format!(
                            "unscoped {kind}/{key}: explicit resource name requires external ownership"
                        )));
                    }
                    if resource["external"] != true {
                        let labels = resource
                            .as_object_mut()
                            .ok_or_else(|| invalid("invalid resource"))?
                            .entry("labels")
                            .or_insert_with(|| json!({}))
                            .as_object_mut()
                            .ok_or_else(|| invalid("invalid resource labels"))?;
                        if labels.contains_key(RUN_LABEL) || labels.contains_key(ARTIFACTS_LABEL) {
                            return Err(invalid("reserved resource ownership label"));
                        }
                        labels.insert(RUN_LABEL.into(), json!(input.run));
                        labels.insert(ARTIFACTS_LABEL.into(), json!(artifacts));
                    }
                }
            }
        }
        let model_services = model
            .get_mut("services")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| invalid("resolved Compose has no services"))?;
        if model_services.is_empty() {
            return Err(super::Error::EmptySelection);
        }
        let mut completed = project.completed_services.clone();
        for service in model_services.values() {
            if let Some(deps) = service["depends_on"].as_object() {
                for (name, dep) in deps {
                    if dep["condition"] == "service_completed_successfully" {
                        completed.push(name.clone());
                    }
                }
            }
        }
        if completed.iter().any(|s| !model_services.contains_key(s)) {
            return Err(invalid(
                "completed_services refers to an unselected service",
            ));
        }
        let mut services = BTreeMap::new();
        for (service_name, service) in model_services.iter_mut() {
            let image = service["image"].as_str().ok_or_else(|| {
                invalid(format!(
                    "{service_name}: prepare a named local image before the experiment"
                ))
            })?;
            let image = self
                .docker
                .inspect_image(image)
                .await
                .map_err(|e| engine(format!("inspect image for {service_name}"), e))?;
            let mounts = service["volumes"].as_array().cloned().unwrap_or_default();
            if mounts
                .iter()
                .any(|m| m["type"] == "volume" && m.get("source").is_none())
            {
                return Err(invalid(format!(
                    "{service_name}: anonymous volumes need explicit named-volume retention settings"
                )));
            }
            if let Some(declared) = image.config.as_ref().and_then(|c| c.volumes.as_ref()) {
                for target in declared {
                    if !mounts.iter().any(|m| m["target"].as_str() == Some(target)) {
                        return Err(invalid(format!(
                            "{service_name}: image volume {target} needs an explicit mount for ownership/retention"
                        )));
                    }
                }
            }
            let image_id = image.id.ok_or_else(|| invalid("Engine omitted image ID"))?;
            let replicas = service["scale"]
                .as_u64()
                .or_else(|| service["deploy"]["replicas"].as_u64())
                .unwrap_or(1) as usize;
            if replicas == 0 {
                return Err(invalid(format!(
                    "{service_name}: zero replicas is unsupported; deselect via profiles"
                )));
            }
            if let Some(fixed) = service["container_name"].as_str() {
                match self.docker.inspect_container(fixed, None).await {
                    Ok(_) => {
                        return Err(invalid(format!("container name already occupied: {fixed}")));
                    }
                    Err(e) if absent(&e) => {}
                    Err(e) => return Err(engine("check fixed container name", e)),
                }
            }
            if service["network_mode"] == "host" {
                diagnostics.push(format!("{service_name}: host networking; ports are shared"));
            }
            if service["privileged"] == true {
                diagnostics.push(format!(
                    "{service_name}: privileged; containment is cooperative"
                ));
            }
            if service.get("volumes").is_some() {
                diagnostics.push(format!("{service_name}: bind/external input contents are live; container cleanup does not reset them"));
            }
            if service.get("healthcheck").is_none() {
                diagnostics.push(format!("{service_name}: process-level readiness only"));
            }
            let object = service
                .as_object_mut()
                .ok_or_else(|| invalid("invalid resolved service"))?;
            object.remove("build");
            object.remove("profiles");
            object.remove("pull_policy");
            if object
                .get("restart")
                .and_then(Value::as_str)
                .is_some_and(|r| r != "no")
            {
                diagnostics.push(format!(
                    "{service_name}: restart policy overridden to no for experiment ownership"
                ));
            }
            policy_changes.push(json!({"service":service_name,"field":"restart",
                "original":object.get("restart"),"effective":"no",
                "reason":"experiment owns restart and termination"}));
            object.insert("restart".into(), json!("no"));
            if let Some(deploy) = object.get_mut("deploy").and_then(Value::as_object_mut)
                && let Some(original) = deploy.remove("restart_policy")
            {
                policy_changes.push(json!({"service":service_name,"field":"deploy.restart_policy",
                    "original":original,"effective":null,"reason":"experiment owns restart and termination"}));
                diagnostics.push(format!(
                    "{service_name}: deploy restart policy removed for experiment ownership"
                ));
            }
            object.insert("image".into(), json!(image_id));
            let labels = object
                .entry("labels")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| invalid("labels did not normalize to an object"))?;
            if labels.contains_key(RUN_LABEL) || labels.contains_key(ARTIFACTS_LABEL) {
                return Err(invalid(format!("reserved label {RUN_LABEL}")));
            }
            labels.insert(RUN_LABEL.into(), json!(input.run));
            labels.insert(ARTIFACTS_LABEL.into(), json!(artifacts));
            services.insert(
                service_name.clone(),
                ComposeService {
                    image_id,
                    replicas,
                    completed: completed.contains(service_name),
                },
            );
        }
        let plan = ResolvedComposeProject {
            run: input.run,
            key: project.key.clone(),
            name,
            manifest,
            sha256: String::new(),
            services,
            diagnostics,
            remove_volumes: project.remove_volumes,
        };
        if !self.project_containers(&plan).await?.is_empty() {
            return Err(invalid("runtime project name already exists"));
        }
        if !self.project_networks(&plan).await?.is_empty()
            || !self.project_volumes(&plan).await?.is_empty()
        {
            return Err(invalid("runtime project resources already exist"));
        }
        // Compose's config renderer already escapes literal dollars for re-import.
        // Preserve that representation; escaping again changes commands/environment.
        let bytes = serde_json::to_vec_pretty(&model)?;
        private_write(&plan.manifest, &bytes).await?;
        let plan = ResolvedComposeProject {
            sha256: digest(&bytes),
            ..plan
        };
        let mut check = Self::args(&plan);
        check.extend(["config".into(), "--quiet".into()]);
        compose(
            &self.socket,
            &artifacts,
            &BTreeMap::new(),
            &check,
            Duration::from_secs(30),
        )
        .await?;
        private_write(
            &artifacts.join(format!("{}.resolution.json", project.key)),
            &serde_json::to_vec_pretty(
                &json!({"compose_version":normalized.compose_version.trim(),"sources":normalized.input_hashes,"request":project,"plan":plan,"policy_changes":policy_changes}),
            )?,
        )
        .await?;
        Ok(plan)
    }

    /// Validate a generated execution manifest with Compose before any launch.
    /// Checks its frozen digest and returns CLI diagnostics on schema failure.
    /// Creates no Engine resources; requires the artifact parent to exist.
    pub(crate) async fn validate_execution(&self, plan: &ResolvedComposeProject) -> Result<()> {
        self.verify_manifest(plan).await?;
        let mut args = Self::args(plan);
        args.extend(["config".into(), "--quiet".into()]);
        compose(
            &self.socket,
            plan.manifest
                .parent()
                .ok_or_else(|| invalid("manifest parent missing"))?,
            &BTreeMap::new(),
            &args,
            Duration::from_secs(30),
        )
        .await?;
        Ok(())
    }

    async fn verify_manifest(&self, plan: &ResolvedComposeProject) -> Result<()> {
        if digest(&tokio::fs::read(&plan.manifest).await?) != plan.sha256 {
            return Err(invalid("frozen Compose manifest changed"));
        }
        Ok(())
    }
    fn args(plan: &ResolvedComposeProject) -> Vec<String> {
        vec![
            "--env-file".into(),
            "/dev/null".into(),
            "--project-name".into(),
            plan.name.clone(),
            "-f".into(),
            plan.manifest.to_string_lossy().into_owned(),
        ]
    }
    pub(crate) async fn project_containers(
        &self,
        plan: &ResolvedComposeProject,
    ) -> Result<Vec<bollard::models::ContainerSummary>> {
        self.docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: project_filters(plan),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("list project containers", e))
    }
    async fn project_networks(
        &self,
        plan: &ResolvedComposeProject,
    ) -> Result<Vec<bollard::models::Network>> {
        self.docker
            .list_networks(Some(ListNetworksOptions {
                filters: project_filters(plan),
            }))
            .await
            .map_err(|e| engine("list project networks", e))
    }
    async fn project_volumes(
        &self,
        plan: &ResolvedComposeProject,
    ) -> Result<Vec<bollard::models::Volume>> {
        self.docker
            .list_volumes(Some(ListVolumesOptions {
                filters: project_filters(plan),
            }))
            .await
            .map(|volumes| volumes.volumes.unwrap_or_default())
            .map_err(|e| engine("list project volumes", e))
    }
    fn owned(summary: &bollard::models::ContainerSummary, plan: &ResolvedComposeProject) -> bool {
        summary.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&plan.run)
    }
    /// Inspect every project container through the Engine. Unknown ownership or
    /// membership fails closed; exact IDs and PIDs are returned for attribution.
    ///
    /// Returns a [`ComposeProjectState`] for `plan`. Engine failures or inconsistent
    /// ownership/service membership return errors. This read is not an atomic
    /// snapshot; concurrent Engine clients may change state between requests.
    pub async fn inspect(&self, plan: &ResolvedComposeProject) -> Result<ComposeProjectState> {
        let mut containers = Vec::new();
        for summary in self.project_containers(plan).await? {
            if !Self::owned(&summary, plan) {
                return Err(invalid(format!("unowned container in {}", plan.name)));
            }
            let id = summary
                .id
                .ok_or_else(|| invalid("Engine omitted container ID"))?;
            let info = self
                .docker
                .inspect_container(&id, None)
                .await
                .map_err(|e| engine(format!("inspect {id}"), e))?;
            let labels = info
                .config
                .as_ref()
                .and_then(|c| c.labels.as_ref())
                .ok_or_else(|| invalid("missing labels"))?;
            if labels.get(RUN_LABEL) != Some(&plan.run)
                || labels.get(PROJECT_LABEL) != Some(&plan.name)
            {
                return Err(invalid("ownership changed during inspection"));
            }
            let service = labels
                .get("com.docker.compose.service")
                .ok_or_else(|| invalid("missing Compose service label"))?
                .clone();
            if !plan.services.contains_key(&service) {
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
            plan: plan.clone(),
            containers,
        })
    }
    /// Evaluate expected replicas, image identity, process state and health.
    /// Returns `false` while readiness is incomplete; unexpected exits, image
    /// changes and restarts return errors with diagnostic context.
    pub fn ready(state: &ComposeProjectState) -> Result<bool> {
        for (name, service) in &state.plan.services {
            let containers: Vec<_> = state
                .containers
                .iter()
                .filter(|c| c.service == *name)
                .collect();
            if containers.len() != service.replicas {
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
    /// Start one frozen project and wait for Engine-confirmed running/healthy or
    /// declared successful completion. Caller must clean up this plan on any error.
    ///
    /// # Parameters and return value
    ///
    /// `input` supplies a frozen plan and readiness budget. Returns the observed
    /// [`ComposeProjectState`] once expected containers satisfy readiness/completion rules.
    ///
    /// # Errors and cancellation
    ///
    /// Manifest changes, CLI/Engine errors, timeout and invalid container state
    /// return errors. Resources may already exist. Dropping the future does not
    /// stop Docker workloads. Pending native startup is joined by quiescence/removal;
    /// retain the plan and await [`Self::stop`] to establish the cleanup boundary.
    pub async fn start(&self, input: StartInput) -> Result<ComposeProjectState> {
        self.verify_manifest(&input.plan).await?;
        let mut args = Self::args(&input.plan);
        args.extend(["up", "--detach", "--no-build", "--pull", "never"].map(str::to_owned));
        if !input.services.is_empty() {
            if input
                .services
                .iter()
                .any(|service| !input.plan.services.contains_key(service))
            {
                return Err(invalid("unknown selected Compose service"));
            }
            args.push("--no-recreate".into());
            args.extend(input.services.clone());
        }
        self.project_command(&input.plan, args, input.readiness_seconds)
            .await?;
        let deadline = Instant::now() + Duration::from_secs(input.readiness_seconds);
        loop {
            let state = self.inspect(&input.plan).await?;
            let mut selected = state.clone();
            if !input.services.is_empty() {
                selected
                    .plan
                    .services
                    .retain(|name, _| input.services.contains(name));
            }
            if Self::ready(&selected)? {
                return Ok(state);
            }
            if Instant::now() >= deadline {
                return Err(invalid(format!("{} readiness timeout", input.plan.name)));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// Create the named IPC owner without starting it, before workload side effects.
    /// Native Compose creates its pinned configuration; Docker rejects another
    /// experiment claiming the same directory-derived container name.
    pub(crate) async fn reserve(
        &self,
        plan: &ResolvedComposeProject,
        service: &str,
        seconds: u64,
    ) -> Result<()> {
        self.verify_manifest(plan).await?;
        let mut args = Self::args(plan);
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
        self.project_command(plan, args, seconds).await
    }

    async fn project_command(
        &self,
        plan: &ResolvedComposeProject,
        args: Vec<String>,
        seconds: u64,
    ) -> Result<()> {
        let log = plan.manifest.with_extension(format!(
            "launch-{}.log",
            &digest(&serde_json::to_vec(&args)?)[..16]
        ));
        let socket = self.socket.clone();
        let directory = plan.manifest.parent().unwrap().to_path_buf();
        let budget = Duration::from_secs(seconds);
        let mut starting = self.starting.lock().await;
        if starting.contains_key(&plan.name) {
            return Err(invalid(
                "project has an unfinished startup; quiesce it first",
            ));
        }
        starting.insert(
            plan.name.clone(),
            Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
                compose(&socket, &directory, &BTreeMap::new(), &args, budget).await
            })))),
        );
        drop(starting);
        let (out, err) = self
            .finish_start(&plan.name)
            .await?
            .ok_or_else(|| invalid("startup command already consumed"))?;
        private_write(&log, format!("{out}\n{err}").as_bytes()).await
    }

    // Keep the native up request alive across cancellation; shutdown joins it
    // before inspecting resources, so delayed container creation cannot escape cleanup.
    async fn finish_start(&self, project: &str) -> Result<Option<(String, String)>> {
        let command = self.starting.lock().await.get(project).cloned();
        let Some(command) = command else {
            return Ok(None);
        };
        let mut command = command.lock().await;
        let Some(pending) = command.as_mut() else {
            return Ok(None);
        };
        let result = pending.await;
        command.take();
        self.starting.lock().await.remove(project);
        result
            .map_err(|e| invalid(format!("Compose startup task: {e}")))?
            .map(Some)
    }

    /// Execute a command in one owned service; cancellation terminates the CLI client.
    pub(crate) async fn exec(
        &self,
        plan: &ResolvedComposeProject,
        service: &str,
        command: &[String],
        timeout: Duration,
    ) -> Result<String> {
        self.verify_manifest(plan).await?;
        Self::running_service(self.inspect(plan).await?, service)?;
        let mut args = Self::args(plan);
        args.extend(["exec".into(), "--no-TTY".into(), service.into()]);
        args.extend_from_slice(command);
        compose(
            &self.socket,
            plan.manifest.parent().unwrap(),
            &BTreeMap::new(),
            &args,
            timeout,
        )
        .await
        .map(|(out, _)| out)
    }

    pub(crate) async fn resources_owned(&self, plan: &ResolvedComposeProject) -> Result<bool> {
        let networks = self.project_networks(plan).await?;
        let volumes = self.project_volumes(plan).await?;
        Ok(networks
            .iter()
            .all(|n| n.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&plan.run))
            && volumes
                .iter()
                .all(|v| v.labels.get(RUN_LABEL) == Some(&plan.run)))
    }

    // Share image, health and replica checks between endpoint routing and command execution.
    fn running_service(mut state: ComposeProjectState, service: &str) -> Result<Container> {
        state.plan.services.retain(|name, _| name == service);
        if state.plan.services.is_empty() || !Self::ready(&state)? {
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

    /// Resolve the single ready service's IPv4 endpoint on one project-owned
    /// network. `network` is the Compose network key and `port` is server-owned
    /// registration data. Missing, ambiguous, unhealthy or unowned resources fail
    /// closed. The returned address is only valid while this generation remains
    /// running; callers must prevent routing after its state changes.
    pub async fn endpoint(
        &self,
        plan: &ResolvedComposeProject,
        service: &str,
        network: &str,
        port: u16,
    ) -> Result<std::net::SocketAddr> {
        validate_name(network)?;
        if port == 0 {
            return Err(invalid("endpoint port must be positive"));
        }
        let container = Self::running_service(self.inspect(plan).await?, service)?;
        let info = self
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
            super::command::verify_listener(
                container
                    .pid
                    .try_into()
                    .map_err(|_| invalid("invalid endpoint PID"))?,
                port,
            )
            .await?;
            let current = self
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
        let network_name = format!("{}_{}", plan.name, network);
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
            .docker
            .inspect_network(network_id, None::<InspectNetworkOptions>)
            .await
            .map_err(|e| engine("inspect endpoint network", e))?;
        let labels = network_info.labels.unwrap_or_default();
        if labels.get(RUN_LABEL) != Some(&plan.run) || labels.get(PROJECT_LABEL) != Some(&plan.name)
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

    /// Attempt graceful project shutdown, retain exit evidence, then remove owned
    /// resources. Continues independent cleanup after errors. Verifies Engine state
    /// after Compose exit; a successful CLI status alone never proves teardown.
    ///
    /// # Parameters and return value
    ///
    /// `input` supplies the owned plan and stop grace period. Returns [`Cleanup`]
    /// with retained diagnostics, remaining resources and verification status.
    /// Inspect that status even when the outer result is `Ok`.
    ///
    /// # Ownership and failure semantics
    ///
    /// External/unowned resources are preserved. Named volumes are removed only
    /// when the plan requests it. Repeated calls tolerate already absent resources.
    /// Cancellation can leave partial cleanup; retain the plan and retry teardown.
    /// Final evidence files remain in the caller-owned artifact directory.
    pub async fn stop(&self, input: StopInput) -> Result<Cleanup> {
        if !input.services.is_empty() {
            return Err(invalid(
                "selected services use quiesce; removal owns the whole experiment",
            ));
        }
        let stopped = self.quiesce(input.clone()).await?;
        if !stopped.stopped {
            return Err(invalid("project failed to quiesce"));
        }
        let mut result = self.remove(input).await?;
        result.diagnostics.extend(stopped.diagnostics);
        Ok(result)
    }

    /// Stop project writers and preserve containers and volumes for inspection.
    ///
    /// Returns shutdown diagnostics and a fresh stopped-state check. `stopped`
    /// establishes process termination, not successful exit or resource removal.
    /// No filesystem contents are exported. Caller must collect evidence and remove
    /// resources explicitly; cancellation cannot complete that sequence.
    pub async fn quiesce(&self, input: StopInput) -> Result<Shutdown> {
        let services = input.services.clone();
        let plan = &input.plan;
        let mut result = Shutdown {
            project: plan.name.clone(),
            diagnostics: Vec::new(),
            stopped: false,
        };
        match self.finish_start(&plan.name).await {
            Ok(Some((out, err))) => {
                if let Err(error) = private_write(
                    &plan.manifest.with_extension("launch-interrupted.log"),
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
        let mut before = self.project_containers(plan).await?;
        let all_owned = before.iter().all(|c| Self::owned(c, plan));
        // Preserve dependency-order stop semantics through Compose, with a bounded
        // grace period. Fall back to per-container Engine operations on CLI error.
        let mut args = Self::args(plan);
        args.extend([
            "stop".into(),
            "--timeout".into(),
            input.stop_seconds.to_string(),
        ]);
        if !services.is_empty() {
            if services
                .iter()
                .any(|name| !plan.services.contains_key(name))
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
        if all_owned && self.verify_manifest(plan).await.is_ok() {
            if let Err(e) = compose(
                &self.socket,
                plan.manifest.parent().unwrap(),
                &BTreeMap::new(),
                &args,
                Duration::from_secs(
                    input.stop_seconds.saturating_mul(before.len() as u64 + 1) + 10,
                ),
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
            if !Self::owned(&summary, plan) {
                continue;
            }
            let Some(id) = summary.id else {
                continue;
            };
            match self.docker.inspect_container(&id, None).await {
                Ok(info) => {
                    if info.state.as_ref().and_then(|s| s.running) == Some(true)
                        && let Err(e) = self
                            .docker
                            .stop_container(
                                &id,
                                Some(StopContainerOptions {
                                    t: Some(input.stop_seconds.min(i32::MAX as u64) as i32),
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
                    match self.docker.inspect_container(&id, None).await {
                        Ok(stopped) => {
                            if stopped.state.as_ref().is_some_and(|s| {
                                s.running == Some(true)
                                    || s.pid.unwrap_or(0) != 0
                                    || s.exit_code == Some(137)
                                    || s.oom_killed == Some(true)
                                    || (plan.key == "system" && s.exit_code != Some(0))
                            }) {
                                result.diagnostics.push(
                                    invalid(format!("{id}: forced/unclean shutdown"))
                                        .diagnostic("stop"),
                                );
                            }
                            if let Err(e) = save_stopped_inspection(
                                plan.manifest.parent().unwrap(),
                                &plan.key,
                                &id,
                                &serde_json::to_vec_pretty(&stopped)?,
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
            .inspect(plan)
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
    pub async fn remove(&self, input: StopInput) -> Result<Cleanup> {
        let plan = &input.plan;
        if !input.services.is_empty() {
            return Err(invalid("removal requires the whole experiment"));
        }
        self.finish_start(&plan.name).await?;
        let state = self.inspect(plan).await?;
        if state
            .containers
            .iter()
            .any(|c| c.pid != 0 || !matches!(c.status.as_str(), "exited" | "created"))
        {
            return Err(invalid("refusing to remove a project with live containers"));
        }
        let mut result = Cleanup {
            project: plan.name.clone(),
            diagnostics: vec![],
            remaining: vec![],
            retained_volumes: vec![],
            verified: false,
        };
        let mut directory_owner = None;
        // Keep the stopped IPC owner until all other owned resources are removed.
        for summary in self.project_containers(plan).await? {
            if summary
                .labels
                .as_ref()
                .and_then(|labels| labels.get(DIRECTORY_OWNER_LABEL))
                .is_some()
            {
                if Self::owned(&summary, plan) {
                    directory_owner = summary.id;
                }
                continue;
            }
            if Self::owned(&summary, plan)
                && let Some(id) = summary.id
                && let Err(e) = self
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
        let networks = self.project_networks(plan).await?;
        for network in networks {
            if let Some(id) = network.id {
                // Compose-scoped names were validated before launch; unknown names
                // require explicit reconciliation rather than destructive cleanup.
                if network.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&plan.run) {
                    if let Err(e) = self.docker.remove_network(&id).await
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
        for volume in self.project_volumes(plan).await? {
            if volume.labels.get(RUN_LABEL) == Some(&plan.run) {
                let remove = input.allow_volume_deletion
                    && volume
                        .labels
                        .get("io.doramcap.remove-volume")
                        .map_or(plan.remove_volumes, |v| v == "true");
                if !remove {
                    result.retained_volumes.push(volume.name);
                    continue;
                }
                if let Err(e) = self
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
            self.project_networks(plan)
                .await?
                .into_iter()
                .filter_map(|n| n.id)
                .map(|id| format!("network:{id}")),
        );
        result.remaining.extend(
            self.project_containers(plan)
                .await?
                .into_iter()
                .filter(|c| c.id != directory_owner)
                .map(|c| format!("container:{}", c.id.unwrap_or_default())),
        );
        if result.remaining.is_empty()
            && result.diagnostics.is_empty()
            && (input.allow_volume_deletion || result.retained_volumes.is_empty())
            && let Some(id) = directory_owner
            && let Err(error) = self
                .docker
                .remove_container(&id, None::<RemoveContainerOptions>)
                .await
            && !absent(&error)
        {
            result
                .diagnostics
                .push(engine("release directory owner", error).diagnostic("cleanup"));
        }
        result.remaining.extend(
            self.project_containers(plan)
                .await?
                .into_iter()
                .filter(|c| {
                    c.labels
                        .as_ref()
                        .is_some_and(|l| l.contains_key(DIRECTORY_OWNER_LABEL))
                })
                .map(|c| format!("container:{}", c.id.unwrap_or_default())),
        );
        result.verified = result.remaining.is_empty();
        Ok(result)
    }
}
// Shared Compose arguments preserve the explicit environment and input order.
async fn config_arguments(
    project: &ComposeProject,
    name: &str,
) -> Result<(std::path::PathBuf, Vec<String>, BTreeMap<String, String>)> {
    if project.files.is_empty() {
        return Err(invalid("at least one Compose file is required"));
    }
    if project
        .environment
        .keys()
        .any(|k| k.starts_with("DOCKER_") || k.starts_with("COMPOSE_"))
    {
        return Err(invalid(
            "Docker/Compose control variables must be supplied as explicit launch settings",
        ));
    }
    let directory = tokio::fs::canonicalize(&project.directory).await?;
    let mut args = vec![
        "--project-directory".into(),
        directory.to_string_lossy().into_owned(),
        "--project-name".into(),
        name.into(),
        "--env-file".into(),
        "/dev/null".into(),
    ];
    let mut sources = BTreeMap::new();
    for (flag, files) in [("-f", &project.files), ("--env-file", &project.env_files)] {
        for file in files {
            let path = tokio::fs::canonicalize(directory.join(file)).await?;
            sources.insert(
                path.to_string_lossy().into_owned(),
                digest(&tokio::fs::read(&path).await?),
            );
            args.extend([flag.into(), path.to_string_lossy().into_owned()]);
        }
    }
    for profile in &project.profiles {
        args.extend(["--profile".into(), profile.clone()]);
    }
    Ok((directory, args, sources))
}

fn project_filters(plan: &ResolvedComposeProject) -> Option<HashMap<String, Vec<String>>> {
    Some(HashMap::from([(
        "label".into(),
        vec![format!("{PROJECT_LABEL}={}", plan.name)],
    )]))
}

pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 40
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(invalid(
            "identity must be 1..40 lowercase letters, digits or hyphens",
        ));
    }
    Ok(())
}

async fn save_stopped_inspection(
    directory: &Path,
    key: &str,
    id: &str,
    bytes: &[u8],
) -> Result<()> {
    for attempt in 0..1024 {
        let name = if attempt == 0 {
            format!("{key}-{id}.stopped.json")
        } else {
            format!("{key}-{id}.stopped.{attempt}.json")
        };
        match private_write(&directory.join(name), bytes).await {
            Ok(()) => return Ok(()),
            Err(super::Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(invalid("too many stopped inspection attempts"))
}
/// Internal identity scoped to the canonical private run directory.
pub(crate) fn project_name(run: &str, key: &str, artifacts: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    format!(
        "dc-{run}-{}-{key}",
        &digest(artifacts.as_os_str().as_bytes())[..16]
    )
}
/// Stable Docker ownership name shared by runs under one canonical output parent.
pub(crate) fn directory_owner(artifacts: &Path) -> Result<String> {
    use std::os::unix::ffi::OsStrExt;
    let parent = artifacts
        .parent()
        .ok_or_else(|| invalid("artifact directory has no parent"))?;
    let parent = std::fs::canonicalize(parent)?;
    Ok(format!(
        "doramcap-dir-{}",
        digest(parent.as_os_str().as_bytes())
    ))
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub(crate) async fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::compose::Error;

    fn discovery_project(directory: &Path) -> ComposeProject {
        ComposeProject {
            key: "workload".into(),
            directory: directory.into(),
            files: vec!["compose.yaml".into()],
            env_files: vec![],
            environment: BTreeMap::new(),
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: false,
        }
    }

    #[tokio::test]
    async fn compose_arguments_pass_explicit_run_and_project_identity() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        tokio::fs::write(root.path().join("compose.yaml"), b"services: {}")
            .await
            .unwrap();
        let project = discovery_project(root.path());
        for run in ["review-a", "review-b"] {
            // Act
            let name = project_name(run, &project.key, root.path());
            let (_, args, _) = config_arguments(&project, &name).await.unwrap();
            // Assert
            let names: Vec<_> = args
                .windows(2)
                .filter(|pair| pair[0] == "--project-name")
                .map(|pair| pair[1].as_str())
                .collect();
            assert_eq!(names, [name.as_str()]);
            assert!(project.environment.is_empty());
        }
    }

    #[tokio::test]
    async fn discovery_and_resolution_reject_invalid_identity_before_io() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("engine.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let adapter = DockerAdapter {
            docker: Docker::connect_with_unix(
                socket.to_str().unwrap(),
                120,
                bollard::API_DEFAULT_VERSION,
            )
            .unwrap(),
            socket: socket.to_string_lossy().into_owned(),
            starting: Default::default(),
        };
        for (run, key) in [("invalid/run", "workload"), ("valid", "invalid/key")] {
            let mut project = discovery_project(Path::new("/nonexistent-project"));
            project.key = key.into();
            // Act
            let profiles = adapter.profiles(run, &project).await;
            let resolution = adapter
                .resolve(ResolveInput {
                    run: run.into(),
                    artifacts: "/nonexistent-artifacts".into(),
                    project,
                })
                .await;
            // Assert
            for error in [profiles.unwrap_err(), resolution.unwrap_err()] {
                assert!(
                    matches!(error, Error::Invalid(ref message) if message.starts_with("identity must"))
                );
            }
        }
    }

    #[test]
    fn digest_preserves_lowercase_sha256_encoding() {
        // Arrange
        let bytes = b"abc";
        // Act
        let actual = digest(bytes);
        // Assert
        assert_eq!(
            actual,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn state() -> ComposeProjectState {
        ComposeProjectState {
            plan: ResolvedComposeProject {
                run: "run".into(),
                key: "project".into(),
                name: "dc-run-project".into(),
                manifest: "/unused".into(),
                sha256: String::new(),
                services: BTreeMap::from([(
                    "worker".into(),
                    ComposeService {
                        image_id: "image".into(),
                        replicas: 1,
                        completed: false,
                    },
                )]),
                diagnostics: vec![],
                remove_volumes: false,
            },
            containers: vec![Container {
                id: "id".into(),
                service: "worker".into(),
                image_id: "image".into(),
                pid: 1,
                status: "running".into(),
                health: None,
                exit_code: None,
                restart_count: 0,
            }],
        }
    }
    #[test]
    fn health_completion_and_image_identity_are_distinct() {
        // Arrange
        let mut state = state();
        // Act / Assert: running alone is process readiness; health starting is not.
        assert!(DockerAdapter::ready(&state).unwrap());
        state.containers[0].health = Some("starting".into());
        assert!(!DockerAdapter::ready(&state).unwrap());
        state.containers[0].health = Some("unhealthy".into());
        assert!(DockerAdapter::ready(&state).is_err());
        // Act / Assert: successful exit only satisfies a declared completion role.
        state.containers[0].health = None;
        state.containers[0].status = "exited".into();
        state.containers[0].exit_code = Some(0);
        assert!(DockerAdapter::ready(&state).is_err());
        state.plan.services.get_mut("worker").unwrap().completed = true;
        assert!(DockerAdapter::ready(&state).unwrap());
        // Act / Assert: unexpected image replacement must never count as ready.
        state.containers[0].image_id = "another-image".into();
        assert!(DockerAdapter::ready(&state).is_err());
    }
    #[test]
    fn missing_replicas_never_count_as_ready() {
        // Arrange
        let mut state = state();
        state.plan.services.get_mut("worker").unwrap().replicas = 2;
        // Act / Assert
        assert!(!DockerAdapter::ready(&state).unwrap());
    }

    pub(crate) fn test_engine(
        root: &Path,
        responses: Vec<(&'static str, Value)>,
    ) -> (DockerAdapter, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let socket = root.join("engine.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let adapter = DockerAdapter {
            docker: Docker::connect_with_unix(
                socket.to_str().unwrap(),
                1,
                bollard::API_DEFAULT_VERSION,
            )
            .unwrap(),
            socket: socket.to_string_lossy().into_owned(),
            starting: Default::default(),
        };
        let server = tokio::spawn(async move {
            for (path, response) in responses {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                assert!(line.contains(path), "unexpected Engine request: {line}");
                loop {
                    line.clear();
                    assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                let body = response.to_string();
                stream.get_mut().write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",body.len()
                ).as_bytes()).await.unwrap();
            }
        });
        (adapter, server)
    }

    #[tokio::test]
    async fn independent_startups_progress_and_duplicate_starts_are_rejected() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let (adapter, server) = test_engine(root.path(), vec![]);
        server.await.unwrap();
        let (release, waiting) = tokio::sync::oneshot::channel();
        let plan: ResolvedComposeProject = serde_json::from_value(json!({
            "run":"run","key":"slow","name":"slow","manifest":root.path().join("manifest.json"),
            "sha256":"", "services":{},"diagnostics":[],"remove_volumes":false
        }))
        .unwrap();
        adapter.starting.lock().await.insert(
            plan.name.clone(),
            Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
                waiting.await.unwrap();
                Ok(("slow".into(), String::new()))
            })))),
        );
        adapter.starting.lock().await.insert(
            "fast".into(),
            Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async {
                Ok(("fast".into(), String::new()))
            })))),
        );
        let mut slow = Box::pin(adapter.finish_start(&plan.name));
        assert!(futures_util::poll!(&mut slow).is_pending());

        // Act / Assert: a different project completes while the first remains blocked.
        let fast = tokio::time::timeout(Duration::from_secs(1), adapter.finish_start("fast"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fast.unwrap().0, "fast");
        let duplicate = tokio::time::timeout(
            Duration::from_secs(1),
            adapter.project_command(&plan, vec![], 1),
        )
        .await
        .unwrap();
        assert!(
            duplicate
                .unwrap_err()
                .to_string()
                .contains("unfinished startup")
        );

        // Act / Assert: the original startup still completes and releases its entry.
        release.send(()).unwrap();
        assert_eq!(slow.await.unwrap().unwrap().0, "slow");
        assert!(adapter.starting.lock().await.is_empty());
    }

    #[tokio::test]
    async fn cancelled_wait_retains_startup_and_concurrent_waiters_consume_it_once() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let (adapter, server) = test_engine(root.path(), vec![]);
        server.await.unwrap();
        let (release, waiting) = tokio::sync::oneshot::channel();
        adapter.starting.lock().await.insert(
            "run".into(),
            Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
                waiting.await.unwrap();
                Ok(("finished".into(), String::new()))
            })))),
        );
        let mut cancelled = Box::pin(adapter.finish_start("run"));
        assert!(futures_util::poll!(&mut cancelled).is_pending());

        // Act / Assert: cancellation leaves an unfinished command available for cleanup.
        drop(cancelled);
        let mut first = Box::pin(adapter.finish_start("run"));
        let mut second = Box::pin(adapter.finish_start("run"));
        assert!(futures_util::poll!(&mut first).is_pending());
        assert!(futures_util::poll!(&mut second).is_pending());
        release.send(()).unwrap();
        let (first, second) = tokio::join!(first, second);
        let outputs: Vec<_> = [first.unwrap(), second.unwrap()]
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(outputs, [("finished".into(), String::new())]);
        assert!(adapter.starting.lock().await.is_empty());
    }

    #[tokio::test]
    async fn exec_rejects_a_replaced_image_before_launching_a_command() {
        // Arrange: Engine reports an owned, running replica with a different image.
        let root = tempfile::tempdir().unwrap();
        let manifest = root.path().join("manifest.json");
        std::fs::write(&manifest, b"{}").unwrap();
        let plan: ResolvedComposeProject = serde_json::from_value(json!({
            "run":"run","key":"app","name":"project","manifest":manifest,
            "sha256":digest(b"{}"),"services":{"worker":{"image_id":"pinned","replicas":1,"completed":false}},
            "diagnostics":[],"remove_volumes":false
        })).unwrap();
        let labels =
            json!({RUN_LABEL:"run",PROJECT_LABEL:plan.name,"com.docker.compose.service":"worker"});
        let (adapter, server) = test_engine(
            root.path(),
            vec![
                (
                    "/containers/json",
                    json!([{"Id":"container","Labels":labels}]),
                ),
                (
                    "/containers/container/json",
                    json!({"Id":"container","Image":"replacement",
                "Config":{"Labels":labels},"State":{"Pid":123,"Status":"running","ExitCode":0},"RestartCount":0}),
                ),
            ],
        );

        // Act
        let error = adapter
            .exec(
                &plan,
                "worker",
                &["should-not-run".into()],
                Duration::from_secs(1),
            )
            .await
            .unwrap_err();

        // Assert
        assert!(matches!(error, Error::Invalid(message) if message.contains("image changed")));
        server.await.unwrap();
    }
}
