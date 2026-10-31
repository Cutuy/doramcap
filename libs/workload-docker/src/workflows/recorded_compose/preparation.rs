//! Resolve app inputs natively, then freeze one experiment and its startup stages.
use super::{ComposeRun, observation};
use crate::compose::{
    adapter::{digest, private_write, validate_name},
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    overlay::OverlaySet,
    *,
};
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedRun {
    pub artifacts: PathBuf,
    pub plan: ResolvedComposeProject,
    pub system_services: Vec<String>,
    pub user_services: Vec<String>,
    pub baseline: FilesystemBaseline,
    pub dora: super::super::dora::DoraRuntime,
}
impl PreparedRun {
    pub fn execution_plan(&self, overlays: Option<&OverlaySet>) -> ResolvedComposeProject {
        overlays.map_or_else(|| self.plan.clone(), |o| o.projects[0].clone())
    }
}
pub struct PrepareRun {
    pub executor: Weak<LocalExecutor>,
    pub adapter: Rc<DockerAdapter>,
}
impl Step for PrepareRun {
    const ID: Option<&'static str> = Some("compose.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare project");
    type Input = ComposeRun;
    type Output = PreparedRun;
    async fn run(&mut self, spec: ComposeRun) -> orchestrator::Result<PreparedRun> {
        self.prepare(spec)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
impl PrepareRun {
    async fn prepare(&self, mut spec: ComposeRun) -> Result<PreparedRun> {
        super::super::shared_dora::resolve(&mut spec.dora, &spec.artifacts)?;
        super::super::shared_dora::validate_endpoints(&spec.dora)?;
        validate(&spec)?;
        let mut builder = tokio::fs::DirBuilder::new();
        builder.mode(0o700).create(&spec.artifacts).await?;
        let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
        observation::snapshot(&artifacts, "before").await?;
        if spec.filesystem.is_some() {
            self.adapter.verify_overlay_namespace().await?;
        }
        let dora = super::super::shared_dora::prepare(&spec.dora, &artifacts).await?;
        let mut model = json!({"services":{}});
        let mut system_services = Vec::new();
        let mut user_services = Vec::new();
        for (system, apps) in [(true, &spec.first_party), (false, &spec.projects)] {
            for app in apps {
                let mut app = app.clone();
                // Normalize every app with the final internal project identity, retaining its own cwd/env.
                let key = app.key.clone();
                app.key = "experiment".into();
                if app
                    .environment
                    .insert(
                        "DORAMCAP_DORA_RUNTIME_DIR".into(),
                        dora.runtime_directory.clone(),
                    )
                    .is_some_and(|old| old != dora.runtime_directory)
                {
                    return Err(invalid("conflicting Dora interpolation setting"));
                }
                let normalized = self
                    .adapter
                    .normalize(&ResolveInput {
                        run: spec.id.clone(),
                        artifacts: artifacts.clone(),
                        project: app.clone(),
                    })
                    .await?;
                private_write(
                    &artifacts.join(format!("{key}.inputs.json")),
                    &serde_json::to_vec(
                        &json!({"sources":normalized.input_hashes,"diagnostics":normalized.stderr}),
                    )?,
                )
                .await?;
                let mut part = normalized.model;
                if part["services"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|service| {
                        service["labels"]
                            .get(crate::compose::adapter::DIRECTORY_OWNER_LABEL)
                            .is_some()
                    })
                {
                    return Err(invalid("reserved directory-owner label"));
                }
                if let Some(volumes) = part.get_mut("volumes").and_then(Value::as_object_mut) {
                    for volume in volumes.values_mut().filter(|v| v["external"] != true) {
                        if volume["labels"].get("io.doramcap.remove-volume").is_some() {
                            return Err(invalid("reserved volume-removal policy label"));
                        }
                        volume["labels"]["io.doramcap.remove-volume"] =
                            app.remove_volumes.to_string().into();
                    }
                }
                let names = part["services"]
                    .as_object()
                    .ok_or_else(|| invalid("Compose input has no services"))?
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                if system {
                    system_services.extend(names);
                } else {
                    user_services.extend(names);
                }
                merge(&mut model, part)?;
            }
        }
        let mut diagnostics = Vec::new();
        {
            let d = &dora;
            let reserved = super::super::dora::services(d);
            merge(&mut model, json!({"services":reserved}))?;
            let services = model["services"].as_object_mut().unwrap();
            for (name, service) in services.iter_mut() {
                let daemon = name == super::super::shared_dora::DAEMON;
                if !daemon {
                    service["ipc"] =
                        format!("service:{}", super::super::shared_dora::DAEMON).into();
                    service.as_object_mut().unwrap().remove("shm_size");
                }
                service["ulimits"]["memlock"] =
                    json!({"soft":d.memlock_bytes,"hard":d.memlock_bytes});
                if name == super::super::shared_dora::DAEMON
                    || name == super::super::shared_dora::COORDINATOR
                {
                    continue;
                }
                let env = service
                    .as_object_mut()
                    .unwrap()
                    .entry("environment")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or_else(|| invalid("invalid Compose environment"))?;
                for (key, value) in
                    dora_app::environment::settings(name, d.daemon_port, &d.zenoh_overlay)
                {
                    if key == dora_app::environment::NODE_ID && env.contains_key(key) {
                        continue;
                    }
                    env.insert(key.into(), value.into());
                }
                let mounts = service
                    .as_object_mut()
                    .unwrap()
                    .entry("volumes")
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .ok_or_else(|| invalid("invalid Compose volumes"))?;
                if mounts.iter().any(|v| {
                    v["target"].as_str().is_some_and(|t| {
                        ["/", "/dev", "/dev/shm"].contains(&t) || t == d.zenoh_overlay
                    })
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
            if !d.graph.is_empty() {
                let graph = super::super::shared_dora::graph(
                    d,
                    &crate::evidence::read_regular(Path::new(&d.graph), 2 * 1024 * 1024).await?,
                )?;
                for node in &graph.nodes {
                    let name = node.id.as_ref();
                    let service = services.get(name).ok_or_else(|| {
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
            }
        }
        model["services"][super::super::shared_dora::DAEMON]["container_name"] =
            crate::compose::adapter::directory_owner(&artifacts)?.into();
        model["services"][super::super::shared_dora::DAEMON]["labels"]
            [crate::compose::adapter::DIRECTORY_OWNER_LABEL] = "true".into();
        for name in &system_services {
            if model["services"][name]["depends_on"]
                .as_object()
                .is_some_and(|deps| deps.keys().any(|dep| user_services.contains(dep)))
            {
                return Err(invalid(
                    "first-party apps cannot depend on adopted apps during startup",
                ));
            }
        }
        freeze_files(&mut model, &artifacts).await?;
        let manifest = artifacts.join("experiment.input.json");
        private_write(&manifest, &serde_json::to_vec(&model)?).await?;
        let mut plan = self
            .adapter
            .resolve(ResolveInput {
                run: spec.id.clone(),
                artifacts: artifacts.clone(),
                project: ComposeProject {
                    key: "experiment".into(),
                    directory: artifacts.clone(),
                    files: vec![manifest],
                    env_files: vec![],
                    environment: BTreeMap::new(),
                    profiles: vec![],
                    services: vec![],
                    completed_services: spec
                        .first_party
                        .iter()
                        .chain(&spec.projects)
                        .flat_map(|p| p.completed_services.clone())
                        .collect(),
                    remove_volumes: spec
                        .first_party
                        .iter()
                        .chain(&spec.projects)
                        .all(|p| p.remove_volumes),
                },
            })
            .await?;
        for warning in &diagnostics {
            eprintln!("warning: {warning}");
        }
        plan.diagnostics.extend(diagnostics);
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| invalid("executor dropped"))?;
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(BaselineInput {
                projects: vec![plan.clone()],
                artifacts: artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?
            .into_value();
        if let Some(directory) = &spec.recovery_anchor_dir {
            super::super::recovery::write_anchor(
                directory,
                &artifacts,
                &spec.id,
                &plan,
                &baseline.recovery_files(),
            )
            .await?;
        }
        Ok(PreparedRun {
            artifacts,
            plan,
            system_services,
            user_services,
            baseline,
            dora,
        })
    }
}

fn merge(model: &mut Value, part: Value) -> Result<()> {
    for kind in ["services", "networks", "volumes", "configs", "secrets"] {
        if let Some(entries) = part[kind].as_object() {
            let target = model
                .as_object_mut()
                .unwrap()
                .entry(kind)
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .unwrap();
            for (key, value) in entries {
                if target.contains_key(key) {
                    if kind == "networks" && key == "default" && target[key] == *value {
                        continue;
                    }
                    return Err(invalid(format!("Compose naming conflict: {kind}/{key}")));
                }
                target.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(())
}
async fn snapshot_file(source: &str, directory: &Path) -> Result<String> {
    let source = tokio::fs::canonicalize(source).await?;
    let bytes = crate::evidence::read_regular(&source, 256 * 1024 * 1024).await?;
    use std::os::unix::fs::PermissionsExt;
    let permissions = tokio::fs::metadata(&source).await?.permissions();
    let destination = directory.join(format!("{}-{:o}", digest(&bytes), permissions.mode()));
    if !destination.exists() {
        private_write(&destination, &bytes).await?;
        tokio::fs::set_permissions(&destination, permissions).await?;
    }
    Ok(destination.to_string_lossy().into_owned())
}
async fn freeze_files(model: &mut Value, artifacts: &Path) -> Result<()> {
    let directory = artifacts.join("inputs");
    let mut builder = tokio::fs::DirBuilder::new();
    builder.mode(0o700).create(&directory).await?;
    for service in model["services"].as_object_mut().unwrap().values_mut() {
        if let Some(mounts) = service.get_mut("volumes").and_then(Value::as_array_mut) {
            for mount in mounts {
                if mount["type"] == "bind"
                    && mount["read_only"] == true
                    && let Some(source) = mount["source"].as_str()
                    && tokio::fs::metadata(source).await?.is_file()
                {
                    mount["source"] = snapshot_file(source, &directory).await?.into();
                }
            }
        }
    }
    for kind in ["configs", "secrets"] {
        if let Some(items) = model.get_mut(kind).and_then(Value::as_object_mut) {
            for item in items.values_mut() {
                if let Some(source) = item["file"].as_str() {
                    item["file"] = snapshot_file(source, &directory).await?.into();
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate(spec: &ComposeRun) -> Result<()> {
    validate_name(&spec.id)?;
    super::validate_budgets(spec.readiness_seconds, spec.stop_seconds)?;
    if !spec.artifacts.is_absolute() || (spec.projects.is_empty() && spec.first_party.is_empty()) {
        return Err(invalid(
            "require an absolute artifact directory, app inputs and positive lifecycle budgets",
        ));
    }
    if spec
        .recovery_anchor_dir
        .as_ref()
        .is_some_and(|p| !p.is_absolute())
    {
        return Err(invalid("recovery anchor must be absolute"));
    }
    let mut keys = HashSet::new();
    for p in spec.first_party.iter().chain(&spec.projects) {
        validate_name(&p.key)?;
        if !p.directory.is_absolute() || !keys.insert(&p.key) {
            return Err(invalid("app directories must be absolute and keys unique"));
        }
    }
    Ok(())
}
