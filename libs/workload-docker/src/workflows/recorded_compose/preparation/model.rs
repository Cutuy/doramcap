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

/// Attach participants to the declared daemon's IPC and native Zenoh config.
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
        if d.services.contains_key(name) {
            continue;
        }
        service["ipc"] = format!("service:{daemon}").into();
        service.as_object_mut().unwrap().remove("shm_size");
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
            mount["target"].as_str().is_some_and(|target| {
                ["/", "/dev", "/dev/shm"].contains(&target) || target == overlay.as_str()
            })
        }) {
            return Err(invalid("app mount overrides shared IPC or Dora config"));
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

/// Bind selected live host entries into every service without changing its IPC namespace.
/// Errors: an existing mount at a selected target uses another source or access mode.
pub(super) fn configure_host_shm(model: &mut Value, names: &[String]) -> Result<()> {
    use workload_lifecycle::plan::HOST_SHM_ROOT;
    for (service, app) in model["services"].as_object_mut().unwrap() {
        for name in names {
            let path = format!("{HOST_SHM_ROOT}/{name}");
            let mounts = app
                .as_object_mut()
                .unwrap()
                .entry("volumes")
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or_else(|| invalid("invalid Compose volumes"))?;
            if let Some(mount) = mounts.iter_mut().find(|mount| mount["target"] == path) {
                if mount["type"] != "bind" || mount["source"] != path || mount["read_only"] == true
                {
                    return Err(invalid(format!(
                        "{service}: mount conflicts with host_shm entry {name}"
                    )));
                }
                mount["bind"]["create_host_path"] = false.into();
            } else {
                mounts.push(json!({"type":"bind", "source":path, "target":path,
                    "read_only":false, "bind":{"create_host_path":false}}));
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
    validate_name(&spec.id)?;
    workload_lifecycle::plan::validate_host_shm(&spec.host_shm).map_err(invalid)?;
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
