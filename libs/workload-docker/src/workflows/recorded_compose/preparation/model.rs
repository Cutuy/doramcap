//! Pure rules for assembling one shared-Dora Compose project.
use super::*;
use crate::compose::adapter::{EXPERIMENT_OWNER_LABEL, validate_name};
use crate::workflows::{
    dora,
    recorded_compose::{MAX_STOP_SECONDS, MIN_STOP_SECONDS},
};
use dora_core::descriptor::Descriptor;
use std::collections::HashSet;

/// Clone app inputs under the experiment identity and approved runtime directory.
pub(super) fn app_input(app: &ComposeProject, runtime_directory: &str) -> Result<ComposeProject> {
    let mut input = app.clone();
    input.key = "experiment".into();
    if input
        .environment
        .insert(
            "DORAMCAP_DORA_RUNTIME_DIR".into(),
            runtime_directory.to_owned(),
        )
        .is_some_and(|old| old != runtime_directory)
    {
        return Err(invalid("conflicting Dora interpolation setting"));
    }
    Ok(input)
}

/// Apply volume policy, record service names and merge normalized Compose input.
pub(super) fn merge_app(
    model: &mut Value,
    mut part: Value,
    remove_volumes: bool,
    services: &mut Vec<String>,
) -> Result<Vec<String>> {
    let names = part["services"]
        .as_object_mut()
        .ok_or_else(|| invalid("Compose input has no services"))?;
    services.extend(names.keys().cloned());
    // Normalization already applied profile selection; the merged manifest has none to activate.
    for service in names.values_mut().filter_map(Value::as_object_mut) {
        service.remove("profiles");
    }
    if let Some(volumes) = part.get_mut("volumes").and_then(Value::as_object_mut) {
        for volume in volumes.values_mut().filter(|v| v["external"] != true) {
            if volume["labels"].get("io.doramcap.remove-volume").is_some() {
                return Err(invalid("reserved volume-removal policy label"));
            }
            volume["labels"]["io.doramcap.remove-volume"] = remove_volumes.to_string().into();
        }
    }
    merge(model, part)
}

const MCAP_MEMLOCK_BYTES: u64 = 256 * 1024 * 1024;

/// Assign owned infrastructure to its run's system-app slice; user settings remain native.
pub(super) fn configure_system_cgroup(
    model: &mut Value,
    services: &[String],
    parent: &str,
) -> Result<()> {
    for name in services {
        let service = model["services"][name]
            .as_object_mut()
            .ok_or_else(|| invalid(format!("{name}: system-app service missing")))?;
        service.insert("cgroup_parent".into(), parent.into());
    }
    Ok(())
}

/// Reserved system service keeping the run's /dev/shm available until final shutdown.
pub(crate) const SHM_SERVICE: &str = "doramcap-shm";

/// Apply the run's IPC default, preserving explicit IPC and complete private shm mounts.
/// BLOCKED requires the configured placeholder when a service uses the run default.
pub(super) fn configure_shm(
    model: &mut Value,
    system_services: &mut Vec<String>,
    access: i32,
) -> Result<()> {
    use workload_lifecycle::launch::HostShmAccess;
    let access = workload_lifecycle::plan::validate_host_shm_access(access).map_err(invalid)?;
    let services = model["services"].as_object_mut().unwrap();
    if services.contains_key(SHM_SERVICE) && !system_services.iter().any(|s| s == SHM_SERVICE) {
        return Err(invalid(format!(
            "{SHM_SERVICE} is reserved for the shared-memory system app"
        )));
    }
    system_services.retain(|name| name != SHM_SERVICE);
    let mut needs_placeholder = false;
    for (name, service) in services.iter_mut().filter(|(name, _)| *name != SHM_SERVICE) {
        if service.get("ipc").is_some() {
            needs_placeholder |= service["ipc"] == format!("service:{SHM_SERVICE}");
            continue;
        }
        let private_mount = service["volumes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|mount| {
                mount["target"].as_str().is_some_and(|target| {
                    Path::new(workload_lifecycle::files::SHM_PATH).starts_with(target)
                })
            })
            || service["tmpfs"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|mount| {
                    mount.as_str().is_some_and(|target| {
                        Path::new(workload_lifecycle::files::SHM_PATH)
                            .starts_with(target.split(':').next().unwrap())
                    })
                });
        if private_mount {
            continue;
        }
        if service.get("shm_size").is_some() {
            return Err(invalid(format!(
                "{name}: shm_size requires explicit private or shareable IPC; it conflicts with the run's shared IPC default"
            )));
        }
        service["ipc"] = match access {
            HostShmAccess::Blocked => {
                needs_placeholder = true;
                format!("service:{SHM_SERVICE}").into()
            }
            HostShmAccess::Passthrough => "host".into(),
            HostShmAccess::Unspecified => unreachable!(),
        };
    }
    if needs_placeholder {
        let placeholder = services.get(SHM_SERVICE).ok_or_else(|| {
            invalid(format!(
                "BLOCKED shared memory requires the {SHM_SERVICE} system app"
            ))
        })?;
        if placeholder["ipc"] != "shareable" {
            return Err(invalid(format!(
                "{SHM_SERVICE}: the shared-memory system app requires ipc: shareable"
            )));
        }
        if placeholder
            .get("volumes")
            .is_some_and(|v| v.as_array().is_some_and(|v| !v.is_empty()))
            || placeholder
                .get("tmpfs")
                .is_some_and(|v| v.as_array().is_some_and(|v| !v.is_empty()))
        {
            return Err(invalid(format!(
                "{SHM_SERVICE}: the shared-memory system app cannot replace its native shm mount"
            )));
        }
    } else {
        services.remove(SHM_SERVICE);
    }
    Ok(())
}

/// Disable seccomp for each named service and raise its memlock limit.
/// Errors: a name that matches no service.
pub(super) fn configure_mcap_writers(model: &mut Value, services: &[String]) -> Result<()> {
    for name in services {
        let service = model["services"]
            .get_mut(name)
            .ok_or_else(|| invalid(format!("unknown MCAP writer service: {name}")))?;
        let mut options: Vec<Value> = service["security_opt"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|option| !option.as_str().is_some_and(|o| o.starts_with("seccomp")))
            .cloned()
            .collect();
        options.push("seccomp=unconfined".into());
        service["security_opt"] = options.into();
        let limit = service["ulimits"]["memlock"]["hard"]
            .as_u64()
            .unwrap_or(0)
            .max(MCAP_MEMLOCK_BYTES);
        service["ulimits"]["memlock"] = json!({"soft":limit,"hard":limit});
    }
    Ok(())
}

/// Attach participants to the declared daemon's native Zenoh config.
pub(super) fn configure_dora(
    model: &mut Value,
    d: &dora::DoraApps,
    runtime: Option<&workload_lifecycle::launch::DoraRuntime>,
    owner: String,
) -> Result<Vec<String>> {
    let Some(runtime) = runtime else {
        return Ok(Vec::new());
    };
    let daemon = d.daemon()?.0;
    let zenoh = runtime
        .zenoh
        .as_ref()
        .ok_or_else(|| invalid("Dora participant binding missing"))?;
    let overlay = &zenoh.target;
    let source = &zenoh.source;
    let mut diagnostics = Vec::new();
    for (name, service) in model["services"].as_object_mut().unwrap() {
        if d.services.contains_key(name) || name == SHM_SERVICE {
            continue;
        }
        let app = service.as_object_mut().unwrap();
        if app
            .get("labels")
            .is_some_and(|labels| labels.get(EXPERIMENT_OWNER_LABEL).is_some())
        {
            return Err(invalid("reserved experiment-owner label"));
        }
        let env = app
            .entry("environment")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| invalid("invalid Compose environment"))?;
        for (key, value) in dora_app::environment::settings(name, runtime.daemon_port, overlay) {
            if key != dora_app::environment::NODE_ID || !env.contains_key(key) {
                env.insert(key.into(), value.into());
            }
        }
        let mounts = app
            .entry("volumes")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| invalid("invalid Compose volumes"))?;
        if mounts.iter().any(|mount| {
            mount["target"]
                .as_str()
                .is_some_and(|target| target == overlay.as_str())
        }) {
            return Err(invalid("app mount overrides Dora config"));
        }
        mounts.push(json!({"type":"bind","source":source,"target":overlay,"read_only":true}));
        if service["network_mode"] != "host" {
            diagnostics.push(format!(
                "{name}: Dora participants need host networking to reach the daemon"
            ));
        }
    }
    model["services"][daemon]["container_name"] = owner.into();
    model["services"][daemon]["labels"][EXPERIMENT_OWNER_LABEL] = "true".into();
    Ok(diagnostics)
}

/// Return the host source of a local bind-backed named volume.
pub(super) fn local_volume_source<'a>(driver: Option<&str>, options: &'a Value) -> Option<&'a str> {
    if driver.is_some_and(|driver| driver != "local")
        || !options["o"]
            .as_str()?
            .split(',')
            .any(|option| matches!(option, "bind" | "rbind"))
    {
        return None;
    }
    options["device"].as_str()
}

/// Reject a host-shm root or ancestor source, including mount and symlink aliases.
pub(super) fn validate_host_shm_source(service: &str, source: &str) -> Result<()> {
    let exposes_host = match workload_lifecycle::files::exposes_host_shm_tree(Path::new(source)) {
        Ok(exposes) => exposes,
        // Missing native sources are checked by execution preparation.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(invalid(format!(
                "{service}: mount source {source}: {error}"
            )));
        }
    };
    if exposes_host {
        return Err(invalid(format!(
            "{service}: whole-host shm mount {source} requires host_shm_access PASSTHROUGH"
        )));
    }
    Ok(())
}

/// Reject whole-host IPC or shm exposure when the run requires isolation.
/// Native sources are resolved on the host, including local named-volume binds.
pub(super) fn validate_host_shm(model: &Value, access: i32) -> Result<()> {
    use workload_lifecycle::launch::HostShmAccess;
    let access = workload_lifecycle::plan::validate_host_shm_access(access).map_err(invalid)?;
    if access == HostShmAccess::Passthrough {
        return Ok(());
    }
    for (name, service) in model["services"].as_object().unwrap() {
        if service["ipc"] == "host" {
            return Err(invalid(format!(
                "{name}: ipc: host requires host_shm_access PASSTHROUGH"
            )));
        }
        for mount in service["volumes"].as_array().into_iter().flatten() {
            let source = match mount["type"].as_str() {
                Some("bind") => mount["source"].as_str(),
                Some("volume") => mount["source"].as_str().and_then(|key| {
                    let volume = &model["volumes"][key];
                    local_volume_source(volume["driver"].as_str(), &volume["driver_opts"])
                }),
                _ => None,
            };
            if let Some(source) = source {
                validate_host_shm_source(name, source)?;
            }
        }
    }
    Ok(())
}

/// Give services without a `stop_grace_period` the run's `stop_seconds`.
/// Errors: a declared period outside `MIN_STOP_SECONDS..=MAX_STOP_SECONDS`, or one that cannot be read.
/// Docker stores whole seconds, so a shorter period would SIGKILL the service on stop.
pub(super) fn configure_stop_grace(model: &mut Value, stop_seconds: u64) -> Result<()> {
    for (name, service) in model["services"].as_object_mut().unwrap().iter_mut() {
        let Some(declared) = service.get("stop_grace_period") else {
            service["stop_grace_period"] = format!("{stop_seconds}s").into();
            continue;
        };
        let period = declared
            .as_str()
            .and_then(|period| humantime::parse_duration(period).ok())
            .ok_or_else(|| {
                invalid(format!(
                    "{name}: cannot read stop_grace_period {declared}; use whole seconds or minutes"
                ))
            })?;
        let allowed = std::time::Duration::from_secs(MIN_STOP_SECONDS)
            ..=std::time::Duration::from_secs(MAX_STOP_SECONDS);
        if !allowed.contains(&period) {
            return Err(invalid(format!(
                "{name}: stop_grace_period {declared} must be from {MIN_STOP_SECONDS}s to {MAX_STOP_SECONDS}s"
            )));
        }
    }
    Ok(())
}

/// Require every graph node to map to a host-networked Compose service.
pub(super) fn validate_graph(model: &Value, graph: &Descriptor) -> Result<()> {
    for node in &graph.nodes {
        let name = node.id.as_ref();
        let service = model["services"].get(name).ok_or_else(|| {
            invalid(format!(
                "Dora node {name} requires a Compose service with the same name"
            ))
        })?;
        if service["network_mode"] != "host" {
            return Err(invalid(format!(
                "Dora node {name} requires host networking"
            )));
        }
    }
    Ok(())
}

/// Reject system startup dependencies on adopted services.
pub(super) fn validate_dependencies(
    model: &Value,
    system: &[String],
    users: &[String],
) -> Result<()> {
    for name in system {
        if model["services"][name]["depends_on"]
            .as_object()
            .is_some_and(|deps| deps.keys().any(|dep| users.contains(dep)))
        {
            return Err(invalid(
                "system apps cannot depend on adopted apps during startup",
            ));
        }
    }
    Ok(())
}

/// Merge supported sections and return warnings for omitted top-level fields.
pub(super) fn merge(model: &mut Value, part: Value) -> Result<Vec<String>> {
    let Value::Object(mut part) = part else {
        return Err(invalid("Compose input must be an object"));
    };
    let model = model.as_object_mut().unwrap();
    for kind in ["services", "networks", "volumes", "configs", "secrets"] {
        if let Some(Value::Object(entries)) = part.remove(kind) {
            let target = model
                .entry(kind)
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .unwrap();
            for (key, value) in entries {
                if target.contains_key(&key) {
                    if kind == "networks" && key == "default" && target[&key] == value {
                        continue;
                    }
                    return Err(invalid(format!("Compose naming conflict: {kind}/{key}")));
                }
                target.insert(key, value);
            }
        }
    }
    // Project name is assigned separately; version is Compose metadata.
    Ok(part
        .keys()
        .filter(|key| !matches!(key.as_str(), "name" | "version"))
        .map(|field| format!("unsupported Compose top-level field '{field}' is ignored"))
        .collect())
}
/// Validate run identity, paths, app keys and budgets before creating artifacts.
pub(super) fn validate(spec: &ComposeRun) -> Result<()> {
    workload_lifecycle::plan::validate_host_shm_access(spec.host_shm_access).map_err(invalid)?;
    validate_name(&spec.id)?;
    if !spec.repository_root.is_absolute() {
        return Err(invalid("repository root must be absolute"));
    }
    super::super::validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
    if (spec.projects.is_empty() && spec.system_apps.is_empty() && spec.dora_apps.is_empty())
        || std::iter::once(&spec.artifacts).any(|path| !path.is_absolute())
    {
        return Err(invalid("require absolute artifact paths and app inputs"));
    }
    if !spec.dataflow_file.is_empty()
        && (spec.dora_apps.is_empty() || !Path::new(&spec.dataflow_file).is_absolute())
    {
        return Err(invalid(
            "dataflow_file needs Dora apps and an absolute path",
        ));
    }
    let mut keys = HashSet::new();
    for p in spec
        .dora_apps
        .iter()
        .chain(&spec.system_apps)
        .chain(&spec.projects)
    {
        validate_name(&p.key)?;
        if !Path::new(&p.directory).is_absolute() || !keys.insert(&p.key) {
            return Err(invalid("app directories must be absolute and keys unique"));
        }
    }
    Ok(())
}
