//! Compile adopted Compose files into frozen execution plans.
use super::{
    ARTIFACTS_LABEL, DockerAdapter, RUN_LABEL, digest, private_write, project_name, validate_name,
};
use crate::compose::{
    ComposeProject, ComposeService, ResolveInput, ResolvedComposeProject, Result,
    command::compose,
    error::{absent, engine, invalid},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Duration};
/// Normalized Compose document and evidence used to freeze its execution plan.
pub(crate) struct NormalizedCompose {
    pub(crate) model: Value,
    pub(crate) compose_version: String,
    pub(crate) input_hashes: BTreeMap<String, String>,
    pub(crate) stderr: String,
}
/// Compiles explicit Compose inputs using the shared Engine and CLI connection.
pub struct ComposeConfiguration<'a> {
    pub(super) adapter: &'a DockerAdapter,
}
impl ComposeConfiguration<'_> {
    /// Require a local socket peer sharing the caller's mount namespace.
    /// Overlay binds must resolve in the daemon's namespace. Different namespaces
    /// fail here. This requires a direct local socket; peer namespace comparison
    /// does not establish the destination behind a proxy.
    pub async fn verify_overlay_namespace(&self) -> Result<()> {
        use std::os::fd::AsRawFd;
        let stream = tokio::net::UnixStream::connect(&self.adapter.socket).await?;
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
            &self.adapter.socket,
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
        let project = input
            .project
            .as_ref()
            .ok_or_else(|| invalid("Compose project missing"))?;
        validate_name(&project.key)?;
        tokio::fs::canonicalize(&input.artifacts).await?;
        let name = project_name(&input.run, &project.key);
        let (directory, mut args, sources) = config_arguments(project, &name).await?;
        args.extend(["config".into(), "--format".into(), "json".into()]);
        args.extend(project.services.clone());
        let (stdout, stderr) = compose(
            &self.adapter.socket,
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
            &self.adapter.socket,
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
        let project = input
            .project
            .as_ref()
            .ok_or_else(|| invalid("Compose project missing"))?;
        let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
        let name = project_name(&input.run, &project.key);
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
            return Err(crate::compose::Error::EmptySelection);
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
                .adapter
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
                .unwrap_or(1);
            if replicas == 0 {
                return Err(invalid(format!(
                    "{service_name}: zero replicas is unsupported; deselect via profiles"
                )));
            }
            if let Some(fixed) = service["container_name"].as_str() {
                match self.adapter.docker.inspect_container(fixed, None).await {
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
            manifest: manifest
                .to_str()
                .ok_or_else(|| invalid("manifest path is not UTF-8"))?
                .into(),
            sha256: String::new(),
            services,
            diagnostics,
            remove_volumes: project.remove_volumes,
        };
        if !self.adapter.project(&plan).containers().await?.is_empty() {
            return Err(invalid("runtime project name already exists"));
        }
        if !self.adapter.project(&plan).networks().await?.is_empty()
            || !self.adapter.project(&plan).volumes().await?.is_empty()
        {
            return Err(invalid("runtime project resources already exist"));
        }
        // Compose's config renderer already escapes literal dollars for re-import.
        // Preserve that representation; escaping again changes commands/environment.
        let bytes = serde_json::to_vec_pretty(&model)?;
        private_write(Path::new(&plan.manifest), &bytes).await?;
        let plan = ResolvedComposeProject {
            sha256: digest(&bytes),
            ..plan
        };
        let mut check = self.adapter.project(&plan).args();
        check.extend(["config".into(), "--quiet".into()]);
        compose(
            &self.adapter.socket,
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
}
// Shared Compose arguments preserve the explicit environment and input order.
pub(super) async fn config_arguments(
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
