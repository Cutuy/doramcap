//! Pure rules for assembling one shared-Dora Compose project.
use super::*;
use crate::compose::adapter::{CONTROLLER_LABEL, EXPERIMENT_OWNER_LABEL, validate_name};
use crate::workflows::{dora, shared_dora::DAEMON};
use dora_core::descriptor::Descriptor;
use std::collections::HashSet;

/// Clone app inputs under the experiment identity and approved runtime directory.
pub(super) fn app_input(
    app: &ComposeProject,
    runtime: &dora::DoraRuntime,
) -> Result<ComposeProject> {
    let mut input = app.clone();
    input.key = "experiment".into();
    if input
        .environment
        .insert(
            "DORAMCAP_DORA_RUNTIME_DIR".into(),
            runtime.runtime_directory.clone(),
        )
        .is_some_and(|old| old != runtime.runtime_directory)
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
        .as_object()
        .ok_or_else(|| invalid("Compose input has no services"))?;
    services.extend(names.keys().cloned());
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

/// Join apps to shared IPC, supply approved settings and add owned Dora services.
pub(super) fn configure_dora(
    model: &mut Value,
    d: &dora::DoraRuntime,
    owner: String,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let services = model["services"].as_object_mut().unwrap();
    for (name, service) in services.iter_mut() {
        service["ipc"] = format!("service:{DAEMON}").into();
        service.as_object_mut().unwrap().remove("shm_size");
        service["ulimits"]["memlock"] = json!({"soft":d.memlock_bytes,"hard":d.memlock_bytes});
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
        for (key, value) in dora_app::environment::settings(name, d.daemon_port, &d.zenoh_overlay) {
            if key != dora_app::environment::NODE_ID || !env.contains_key(key) {
                env.insert(key.into(), value.into());
            }
        }
        let mounts = app
            .entry("volumes")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| invalid("invalid Compose volumes"))?;
        if mounts.iter().any(|v| {
            v["target"]
                .as_str()
                .is_some_and(|t| ["/", "/dev", "/dev/shm"].contains(&t) || t == d.zenoh_overlay)
        }) {
            return Err(invalid(
                "app mount overrides shared IPC or reserved Dora settings",
            ));
        }
        mounts.push(json!({"type":"bind","source":d.zenoh_overlay,"target":d.zenoh_overlay,"read_only":true}));
        if service["network_mode"] != "host" {
            diagnostics.push(format!("{name}: IPC silos: Compose networking is preserved; Dora 1.1 nodes need host networking to reach the daemon on localhost. IPC and /dev/shm are shared."));
        }
    }
    diagnostics.extend(merge(model, json!({"services":dora::services(d)}))?);
    model["services"][DAEMON]["container_name"] = owner.into();
    model["services"][DAEMON]["labels"][EXPERIMENT_OWNER_LABEL] = "true".into();
    let scope = super::super::reset::directory();
    let scope = scope
        .to_str()
        .ok_or_else(|| invalid("controller path is not UTF-8"))?;
    for section in ["services", "networks"] {
        if let Some(resources) = model[section].as_object_mut() {
            for resource in resources.values_mut().filter(|r| r["external"] != true) {
                if resource["labels"].get(CONTROLLER_LABEL).is_some() {
                    return Err(invalid("reserved experiment-controller label"));
                }
                resource["labels"][CONTROLLER_LABEL] = scope.into();
            }
        }
    }
    Ok(diagnostics)
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
    if !spec.repository_root.is_absolute() {
        return Err(invalid("repository root must be absolute"));
    }
    super::super::validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
    if (spec.projects.is_empty() && spec.system_apps.is_empty())
        || std::iter::once(&spec.artifacts).any(|path| !path.is_absolute())
    {
        return Err(invalid("require absolute artifact paths and app inputs"));
    }
    let mut keys = HashSet::new();
    for p in spec.system_apps.iter().chain(&spec.projects) {
        validate_name(&p.key)?;
        if !p.directory.is_absolute() || !keys.insert(&p.key) {
            return Err(invalid("app directories must be absolute and keys unique"));
        }
    }
    Ok(())
}
