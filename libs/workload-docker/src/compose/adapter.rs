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
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;

pub(crate) const RUN_LABEL: &str = "io.doramcap.run";
const PROJECT_LABEL: &str = "com.docker.compose.project";

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
        Ok(Self { docker, socket })
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
                    project: item.labels.get(PROJECT_LABEL).cloned(),
                    kind: "volume",
                    id: item.name,
                });
            }
        }
        Ok(found)
    }

    /// Discover declared profile names using the same explicit inputs as resolution.
    /// Compose diagnostics and errors remain private to the caller.
    pub async fn profiles(&self, project: &ComposeProject) -> Result<Vec<String>> {
        let mut discovery = project.clone();
        discovery.profiles.clear();
        discovery.services.clear();
        let (directory, mut args, _) = config_arguments(&discovery, "dc-console-discovery").await?;
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

    /// Resolve supplied files, freeze local image IDs, and write a private launch
    /// manifest. No Docker resources are created. Host mounts remain live inputs.
    ///
    /// # Parameters and return value
    ///
    /// `input` supplies a run ID, existing artifact directory and prepared project.
    /// Returns a pinned [`ResolvedComposeProject`]; writes private manifest and resolution evidence.
    ///
    /// # Errors
    ///
    /// Invalid settings, unsupported resources, ownership collisions, missing local
    /// images, filesystem failures and Compose/Engine failures return errors.
    /// Partially written artifacts may remain; input files are not modified.
    pub async fn resolve(&self, input: ResolveInput) -> Result<ResolvedComposeProject> {
        validate_name(&input.run)?;
        validate_name(&input.project.key)?;
        let project = &input.project;
        let name = format!("dc-{}-{}", input.run, project.key);
        let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
        let manifest = artifacts.join(format!("{}.compose.json", project.key));
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
        let mut model: Value = serde_json::from_str(&stdout)?;
        let (version, _) = compose(
            &self.socket,
            &directory,
            &BTreeMap::new(),
            &["version".into(), "--short".into()],
            Duration::from_secs(10),
        )
        .await?;
        let mut diagnostics = Vec::new();
        let mut policy_changes = Vec::new();
        if !stderr.trim().is_empty() {
            diagnostics.push(stderr);
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
                        if labels.contains_key(RUN_LABEL) {
                            return Err(invalid("reserved resource ownership label"));
                        }
                        labels.insert(RUN_LABEL.into(), json!(input.run));
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
            if labels.contains_key(RUN_LABEL) {
                return Err(invalid(format!("reserved label {RUN_LABEL}")));
            }
            labels.insert(RUN_LABEL.into(), json!(input.run));
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
        let filters = Some(HashMap::from([(
            "label".into(),
            vec![format!("{PROJECT_LABEL}={}", plan.name)],
        )]));
        if !self
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: filters.clone(),
            }))
            .await
            .map_err(|e| engine("preflight networks", e))?
            .is_empty()
            || !self
                .docker
                .list_volumes(Some(ListVolumesOptions { filters }))
                .await
                .map_err(|e| engine("preflight volumes", e))?
                .volumes
                .unwrap_or_default()
                .is_empty()
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
                &json!({"compose_version":version.trim(),"sources":sources,"request":project,"plan":plan,"policy_changes":policy_changes}),
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
                filters: Some(HashMap::from([(
                    "label".into(),
                    vec![format!("{PROJECT_LABEL}={}", plan.name)],
                )])),
                ..Default::default()
            }))
            .await
            .map_err(|e| engine("list project containers", e))
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
    /// stop Docker workloads; always retain the plan for [`Self::stop`].
    pub async fn start(&self, input: StartInput) -> Result<ComposeProjectState> {
        self.verify_manifest(&input.plan).await?;
        let mut args = Self::args(&input.plan);
        args.extend(["up", "--detach", "--no-build", "--pull", "never"].map(str::to_owned));
        let (out, err) = compose(
            &self.socket,
            input.plan.manifest.parent().unwrap(),
            &BTreeMap::new(),
            &args,
            Duration::from_secs(input.readiness_seconds),
        )
        .await?;
        private_write(
            &input.plan.manifest.with_extension("launch.log"),
            format!("{out}\n{err}").as_bytes(),
        )
        .await?;
        let deadline = Instant::now() + Duration::from_secs(input.readiness_seconds);
        loop {
            let state = self.inspect(&input.plan).await?;
            if Self::ready(&state)? {
                return Ok(state);
            }
            if Instant::now() >= deadline {
                return Err(invalid(format!("{} readiness timeout", input.plan.name)));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub(crate) async fn resources_owned(&self, plan: &ResolvedComposeProject) -> Result<bool> {
        let filters = Some(HashMap::from([(
            "label".into(),
            vec![format!("{PROJECT_LABEL}={}", plan.name)],
        )]));
        let networks = self
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: filters.clone(),
            }))
            .await
            .map_err(|e| engine("inspect network ownership", e))?;
        let volumes = self
            .docker
            .list_volumes(Some(ListVolumesOptions { filters }))
            .await
            .map_err(|e| engine("inspect volume ownership", e))?;
        Ok(networks
            .iter()
            .all(|n| n.labels.as_ref().and_then(|l| l.get(RUN_LABEL)) == Some(&plan.run))
            && volumes
                .volumes
                .unwrap_or_default()
                .iter()
                .all(|v| v.labels.get(RUN_LABEL) == Some(&plan.run)))
    }

    /// Execute a trusted argument vector in one owned, running service replica.
    /// Verifies the frozen manifest, ownership and pinned image before delegating
    /// to Compose. Uses no shell; output limits and cancellation follow `compose`.
    /// A cancelled exec can continue until its container is quiesced by the caller.
    pub(crate) async fn exec_service(
        &self,
        plan: &ResolvedComposeProject,
        service: &str,
        command: &[String],
        timeout: Duration,
    ) -> Result<String> {
        self.verify_manifest(plan).await?;
        let state = self.inspect(plan).await?;
        let replicas: Vec<_> = state
            .containers
            .iter()
            .filter(|c| c.service == service)
            .collect();
        let expected = plan
            .services
            .get(service)
            .ok_or_else(|| invalid("exec service absent from plan"))?;
        if replicas.len() != 1
            || replicas[0].status != "running"
            || replicas[0].image_id != expected.image_id
        {
            return Err(invalid("exec requires one running, pinned service replica"));
        }
        let mut args = Self::args(plan);
        args.extend(["exec".into(), "-T".into(), service.into()]);
        args.extend_from_slice(command);
        let (stdout, _) = compose(
            &self.socket,
            plan.manifest.parent().unwrap(),
            &BTreeMap::new(),
            &args,
            timeout,
        )
        .await?;
        Ok(stdout)
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
        let state = self.inspect(plan).await?;
        if !Self::ready(&state)? {
            return Err(invalid("project is not ready"));
        }
        let mut containers = state.containers.iter().filter(|c| c.service == service);
        let container = containers
            .next()
            .ok_or_else(|| invalid("endpoint service absent"))?;
        if containers.next().is_some() {
            return Err(invalid("endpoint service has multiple containers"));
        }
        let info = self
            .docker
            .inspect_container(&container.id, None)
            .await
            .map_err(|e| engine("inspect endpoint container", e))?;
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
        let plan = &input.plan;
        let mut result = Shutdown {
            project: plan.name.clone(),
            diagnostics: Vec::new(),
            stopped: false,
        };
        let before = self.project_containers(plan).await?;
        let all_owned = before.iter().all(|c| Self::owned(c, plan));
        // Preserve dependency-order stop semantics through Compose, with a bounded
        // grace period. Fall back to per-container Engine operations on CLI error.
        let mut args = Self::args(plan);
        args.extend([
            "stop".into(),
            "--timeout".into(),
            input.stop_seconds.to_string(),
        ]);
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
            .all(|c| c.pid == 0 && matches!(c.status.as_str(), "exited" | "created"));
        Ok(result)
    }

    /// Remove an already stopped project after its evidence has been collected.
    ///
    /// Refuses live or unowned containers. Returns independent cleanup diagnostics;
    /// inspect `verified` even on success. Retains volumes unless explicitly selected
    /// for deletion. Cancellation leaves the caller responsible for reconciliation.
    pub async fn remove(&self, input: StopInput) -> Result<Cleanup> {
        let plan = &input.plan;
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
        let current = self.project_containers(plan).await?;
        if current.iter().all(|c| Self::owned(c, plan))
            && self.verify_manifest(plan).await.is_ok()
            && self.resources_owned(plan).await?
        {
            let mut args = Self::args(plan);
            args.extend([
                "down".into(),
                "--remove-orphans".into(),
                "--timeout".into(),
                input.stop_seconds.to_string(),
            ]);
            if plan.remove_volumes {
                args.push("--volumes".into());
            }
            if let Err(e) = compose(
                &self.socket,
                plan.manifest.parent().unwrap(),
                &BTreeMap::new(),
                &args,
                Duration::from_secs(30 + input.stop_seconds),
            )
            .await
            {
                result.diagnostics.push(e.diagnostic("compose down"));
            }
        }
        // Docker can outlive the CLI. Reconcile independently, preserving unowned resources.
        for summary in self.project_containers(plan).await? {
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
        let filters = Some(HashMap::from([(
            "label".into(),
            vec![format!("{PROJECT_LABEL}={}", plan.name)],
        )]));
        let networks = self
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: filters.clone(),
            }))
            .await
            .map_err(|e| engine("verify networks", e))?;
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
        let volumes = self
            .docker
            .list_volumes(Some(ListVolumesOptions { filters }))
            .await
            .map_err(|e| engine("verify volumes", e))?;
        for volume in volumes.volumes.unwrap_or_default() {
            if volume.labels.get(RUN_LABEL) == Some(&plan.run) {
                if !plan.remove_volumes {
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
        for c in self.project_containers(plan).await? {
            result
                .remaining
                .push(format!("container:{}", c.id.unwrap_or_default()));
        }
        let networks = self
            .docker
            .list_networks(Some(ListNetworksOptions {
                filters: Some(HashMap::from([(
                    "label".into(),
                    vec![format!("{PROJECT_LABEL}={}", plan.name)],
                )])),
            }))
            .await
            .map_err(|e| engine("final network inspection", e))?;
        result.remaining.extend(
            networks
                .into_iter()
                .filter_map(|n| n.id)
                .map(|id| format!("network:{id}")),
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
mod tests {
    use super::*;

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
}
