//! Resolve app inputs natively, then freeze one experiment and its startup stages.
mod model;
mod snapshot;
pub(super) use model::SHM_SERVICE;

use super::ComposeRun;
use crate::compose::{
    adapter::{digest, private_write},
    error::invalid,
    filesystem::{BaselineInput, CaptureFilesystemBaseline, FilesystemBaseline},
    mounts,
    overlay::PreparedOverlays,
    *,
};
use crate::workflows::shared_dora::MAX_GRAPH_BYTES;
use orchestrator::{LocalExecutor, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

/// Largest read-only bind, config or secret file frozen into run artifacts.
const MAX_FROZEN_FILE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedRun {
    /// Shared inputs with Dora runtime facts filled by this preparation.
    pub run: workload_lifecycle::launch::RunPlan,
    pub artifacts: PathBuf,
    pub plan: ResolvedComposeProject,
    pub system_cgroup: Option<super::plan::CgroupPlacement>,
    pub system_services: Vec<String>,
    pub user_services: Vec<String>,
    pub mcap_services: Vec<String>,
    pub baseline: FilesystemBaseline,
    pub dora: super::super::dora::DoraApps,
    pub snapshot: ConfigurationSnapshot,
}
impl PreparedRun {
    pub fn execution_plan(&self, overlays: Option<&PreparedOverlays>) -> ResolvedComposeProject {
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
/// Expand native Dora modules relative to their source file for configuration assembly.
/// Completed graph validation remains part of experiment resolution.
pub async fn expand_graph(path: &Path) -> Result<Vec<u8>> {
    use dora_core::descriptor::{Descriptor, DescriptorExt};
    let bytes = crate::evidence::read_regular(path, MAX_GRAPH_BYTES).await?;
    let graph = Descriptor::parse(bytes)
        .map_err(|error| invalid(error.to_string()))?
        .expand(
            path.parent()
                .ok_or_else(|| invalid("graph parent missing"))?,
        )
        .map_err(|error| invalid(error.to_string()))?;
    Ok(serde_json::to_vec(&graph)?)
}

/// List profiles using the same repository-scoped Compose identity as execution.
pub async fn profiles(
    adapter: &DockerAdapter,
    run: &str,
    repository: &Path,
    project: &ComposeProject,
) -> Result<Vec<String>> {
    adapter
        .configuration()
        .profiles(
            run,
            project,
            &crate::compose::adapter::project_name("experiment", repository),
        )
        .await
}

/// Frozen experiment inputs; resolution creates private files without starting resources.
#[derive(Clone, Debug)]
pub struct ResolvedRun {
    /// Shared inputs with Dora runtime facts filled by this preparation.
    pub run: workload_lifecycle::launch::RunPlan,
    pub artifacts: PathBuf,
    pub plan: ResolvedComposeProject,
    pub system_cgroup: Option<super::plan::CgroupPlacement>,
    pub system_services: Vec<String>,
    pub user_services: Vec<String>,
    pub mcap_services: Vec<String>,
    pub dora: super::super::dora::DoraApps,
    pub snapshot: ConfigurationSnapshot,
}

/// Resolve inputs using the execution path, without reset, reservation, mounts or containers.
/// `spec.artifacts` must name a new private directory beneath an existing parent.
/// Callers compare the snapshot with fresh preparation in the runner's preflight callback.
pub async fn resolve(adapter: &DockerAdapter, spec: ComposeRun) -> Result<ResolvedRun> {
    // Validate the request before creating run artifacts.
    model::validate(&spec)?;
    crate::evidence::create_private_dir(&spec.artifacts).await?;
    let artifacts = tokio::fs::canonicalize(&spec.artifacts).await?;
    if !spec.mcap_writers.services.is_empty() || !spec.mcap_writers.apps.is_empty() {
        validate_recording_host(&spec.mcap_writers.directories, &spec.id).await?;
    }
    // Freeze Dora inputs and merge apps while recording their startup stages.
    let runtime_directory = artifacts.join("dora/runtime");
    tokio::fs::create_dir_all(&runtime_directory).await?;
    let runtime_directory = runtime_directory.to_string_lossy().into_owned();
    let mut run = spec.run_plan();
    let mut dora = super::super::dora::DoraApps::default();
    let mut model = json!({"services":{}});
    let mut dora_services = Vec::new();
    let mut system_services = Vec::new();
    let mut user_services = Vec::new();
    let mut diagnostics = Vec::new();
    let mut mcap_services = spec.mcap_writers.services.clone();
    for (apps, services) in [
        (&spec.dora_apps, &mut dora_services),
        (&spec.system_apps, &mut system_services),
        (&spec.projects, &mut user_services),
    ] {
        for app in apps {
            let mut normalized = adapter
                .configuration()
                .normalize(
                    &spec.id,
                    &model::app_input(app, &runtime_directory)?,
                    &crate::compose::adapter::project_name("experiment", &spec.repository_root),
                )
                .await?;
            if spec.dora_apps.iter().any(|d| d.key == app.key)
                && let Some(manifest) = &normalized.app
            {
                let name = &manifest.compose.as_ref().unwrap().service;
                let service = &mut normalized.model["services"][name];
                let volumes = service
                    .as_object_mut()
                    .ok_or_else(|| invalid("Dora app service is absent"))?
                    .entry("volumes")
                    .or_insert_with(|| json!([]));
                for target in &manifest.output_targets {
                    volumes.as_array_mut().ok_or_else(|| invalid("Dora volumes must be an array"))?
                        .push(json!({"type":"bind","source":runtime_directory,"target":target,"bind":{"create_host_path":false}}));
                }
            }
            let service_start = services.len();
            let warnings =
                model::merge_app(&mut model, normalized.model, app.remove_volumes, services)?;
            if spec.mcap_writers.apps.contains(&app.key) {
                mcap_services.extend_from_slice(&services[service_start..]);
            }
            private_write(
                    &artifacts.join(format!("{}.inputs.json", app.key)),
                    &serde_json::to_vec(
                        &json!({"sources":normalized.input_hashes,"diagnostics":normalized.stderr,"warnings":warnings}),
                    )?,
                )
                .await?;
            diagnostics.extend(warnings);
        }
    }
    model::validate_host_shm(&model, spec.host_shm_access)?;
    validate_external_volume_shm(adapter, &model, spec.host_shm_access).await?;
    validate_external_ipc(adapter, &model, spec.host_shm_access).await?;
    model::configure_shm(&mut model, &mut system_services, spec.host_shm_access)?;
    let system_cgroup = if !dora_services.is_empty()
        || !system_services.is_empty()
        || model["services"].get(SHM_SERVICE).is_some()
    {
        let placement = super::cgroup::placement(adapter, &spec).await?;
        model::configure_system_cgroup(
            &mut model,
            &[dora_services.clone(), system_services.clone()].concat(),
            &placement.parent,
        )?;
        Some(placement)
    } else {
        None
    };
    dora.services = dora_services
        .iter()
        .map(|name| (name.clone(), model["services"][name].clone()))
        .collect();
    run.dora_runtime = super::super::shared_dora::resolve(&mut dora, runtime_directory)?;
    super::super::shared_dora::prepare(
        &mut dora,
        run.dora_runtime.as_mut(),
        &spec.dataflow_file,
        &artifacts,
    )
    .await?;
    for (name, service) in &dora.services {
        model["services"][name] = service.clone();
    }
    // Configure Dora bindings, then validate graph bindings and startup dependencies.
    diagnostics.extend(model::configure_dora(
        &mut model,
        &dora,
        run.dora_runtime.as_ref(),
        super::super::shared_dora::owner(&spec.repository_root),
    )?);
    model::configure_stop_grace(&mut model, spec.stop_seconds)?;
    if let Some(dataflow) = run
        .dora_runtime
        .as_ref()
        .and_then(|runtime| runtime.dataflow.as_ref())
    {
        let graph = super::super::shared_dora::graph(
            &dataflow.source,
            &crate::evidence::read_regular(Path::new(&dataflow.source), MAX_GRAPH_BYTES).await?,
        )?;
        model::validate_graph(&model, &graph)?;
    }
    model::validate_dependencies(
        &model,
        &dora_services,
        &[system_services.clone(), user_services.clone()].concat(),
    )?;
    model::validate_dependencies(&model, &system_services, &user_services)?;
    // Docker's default seccomp filter blocks the io_uring calls MCAP writers need.
    model::configure_mcap_writers(&mut model, &mcap_services)?;
    // Snapshot file inputs and resolve one project with pinned images.
    freeze_files(&mut model, &artifacts).await?;
    // Keep shared bindings aligned with the frozen files each process will see.
    if let Some(runtime) = run.dora_runtime.as_mut() {
        if let Some(zenoh) = runtime.zenoh.as_mut() {
            let service = system_services
                .first()
                .or(user_services.first())
                .map(String::as_str)
                .unwrap_or(dora.daemon()?.0);
            zenoh.source = super::super::dora::DoraApps::host_path(
                &model["services"][service],
                &zenoh.target,
            )?;
            for participant in system_services.iter().chain(&user_services) {
                if super::super::dora::DoraApps::host_path(
                    &model["services"][participant],
                    &zenoh.target,
                )? != zenoh.source
                {
                    return Err(invalid(
                        "Dora participant configuration changed during preparation",
                    ));
                }
            }
        }
        if let Some(dataflow) = runtime.dataflow.as_mut() {
            dataflow.source = super::super::dora::DoraApps::host_path(
                &model["services"][dora.coordinator()?.0],
                &dataflow.target,
            )?;
        }
    }
    let manifest = artifacts.join("experiment.input.json");
    private_write(&manifest, &serde_json::to_vec(&model)?).await?;
    let apps = spec
        .dora_apps
        .iter()
        .chain(&spec.system_apps)
        .chain(&spec.projects);
    let mut plan = adapter
        .configuration()
        .resolve(ResolveInput {
            repository_root: Some(
                spec.repository_root
                    .to_str()
                    .ok_or_else(|| invalid("repository path is not UTF-8"))?
                    .into(),
            ),
            run: spec.id.clone(),
            artifacts: artifacts
                .to_str()
                .ok_or_else(|| invalid("artifact path is not UTF-8"))?
                .into(),
            project: Some(ComposeProject {
                key: "experiment".into(),
                directory: spec
                    .repository_root
                    .to_str()
                    .ok_or_else(|| invalid("repository path is not UTF-8"))?
                    .into(),
                files: vec![
                    manifest
                        .to_str()
                        .ok_or_else(|| invalid("manifest path is not UTF-8"))?
                        .into(),
                ],
                completed_services: apps
                    .clone()
                    .flat_map(|p| p.completed_services.clone())
                    .collect(),
                remove_volumes: apps.clone().all(|p| p.remove_volumes),
                ..Default::default()
            }),
        })
        .await?;
    for warning in &diagnostics {
        eprintln!("warning: {warning}");
    }
    plan.diagnostics.extend(diagnostics);
    let snapshot = snapshot::capture(
        &spec,
        &artifacts,
        &plan,
        &system_services,
        &user_services,
        &dora,
        run.dora_runtime.as_ref(),
    )
    .await?;
    Ok(ResolvedRun {
        run,
        artifacts,
        plan,
        system_cgroup,
        system_services,
        user_services,
        mcap_services,
        dora,
        snapshot,
    })
}

/// Resolve native external IPC references, refusing host access or unresolvable targets.
async fn validate_external_ipc(adapter: &DockerAdapter, model: &Value, access: i32) -> Result<()> {
    use crate::compose::error::engine;
    use std::collections::BTreeSet;
    use workload_lifecycle::launch::HostShmAccess;
    if access != HostShmAccess::Blocked as i32 {
        return Ok(());
    }
    for (service, value) in model["services"].as_object().unwrap() {
        let Some(target) = value["ipc"]
            .as_str()
            .and_then(|ipc| ipc.strip_prefix("container:"))
        else {
            continue;
        };
        let mut target = target.to_owned();
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(target.clone()) {
                return Err(invalid(format!(
                    "{service}: cyclic external IPC reference {target}"
                )));
            }
            let container = adapter
                .docker
                .inspect_container(&target, None)
                .await
                .map_err(|error| {
                    engine(
                        format!("{service}: inspect external IPC container {target}"),
                        error,
                    )
                })?;
            let ipc = container
                .host_config
                .as_ref()
                .and_then(|config| config.ipc_mode.as_deref())
                .ok_or_else(|| {
                    invalid(format!(
                        "{service}: cannot resolve external IPC container {target}"
                    ))
                })?;
            if ipc == "host" {
                return Err(invalid(format!(
                    "{service}: external IPC container {target} uses host IPC and requires host_shm_access PASSTHROUGH"
                )));
            }
            for mount in container
                .mounts
                .iter()
                .flatten()
                .filter(|mount| mount.destination.as_deref() == Some("/dev/shm"))
            {
                if mount.typ.as_deref() == Some("bind")
                    && let Some(source) = &mount.source
                {
                    model::validate_host_shm_source(service, source)?;
                }
                if mount.typ.as_deref() == Some("volume")
                    && let Some(name) = &mount.name
                {
                    let volume = adapter.docker.inspect_volume(name).await.map_err(|error| {
                        engine(
                            format!("{service}: inspect external IPC volume {name}"),
                            error,
                        )
                    })?;
                    let options = serde_json::to_value(&volume.options)?;
                    if let Some(source) = model::local_volume_source(Some(&volume.driver), &options)
                    {
                        model::validate_host_shm_source(service, source)?;
                    }
                }
            }
            if let Some(parent) = ipc.strip_prefix("container:") {
                target = parent.to_owned();
            } else if matches!(ipc, "" | "none" | "private" | "shareable") {
                break;
            } else {
                return Err(invalid(format!(
                    "{service}: cannot resolve external IPC mode {ipc} from {target}"
                )));
            }
        }
    }
    Ok(())
}

/// Check Engine-owned external volume sources before any container starts.
async fn validate_external_volume_shm(
    adapter: &DockerAdapter,
    model: &Value,
    access: i32,
) -> Result<()> {
    use crate::compose::error::engine;
    use std::collections::BTreeMap;
    use workload_lifecycle::launch::HostShmAccess;
    if access != HostShmAccess::Blocked as i32 {
        return Ok(());
    }
    let mut volumes = BTreeMap::new();
    for (service, value) in model["services"].as_object().unwrap() {
        for mount in value["volumes"].as_array().into_iter().flatten() {
            if mount["type"] == "volume"
                && let Some(key) = mount["source"].as_str()
                && model["volumes"][key]["external"] == true
            {
                let name = model["volumes"][key]["name"].as_str().unwrap_or(key);
                volumes.entry(name).or_insert(service);
            }
        }
    }
    for (name, service) in volumes {
        let volume =
            adapter.docker.inspect_volume(name).await.map_err(|error| {
                engine(format!("{service}: inspect external volume {name}"), error)
            })?;
        if volume.driver == "local" {
            let options = serde_json::to_value(&volume.options)?;
            if let Some(source) = model::local_volume_source(Some(&volume.driver), &options) {
                model::validate_host_shm_source(service, source)?;
            }
        }
    }
    Ok(())
}

/// Whether a kernel release is at least 6.1, the MCAP writer's io_uring requirement.
fn kernel_supports_mcap_writer(release: &str) -> bool {
    let mut parts = release.split(['.', '-']).map(str::parse::<u32>);
    matches!(
        (parts.next(), parts.next()),
        (Some(Ok(major)), Some(Ok(minor))) if (major, minor) >= (6, 1)
    )
}

/// Refuse a host where the MCAP writer cannot record into `directories`.
/// Errors: kernel older than 6.1, io_uring disabled, or a filesystem without `O_DIRECT`.
async fn validate_recording_host(directories: &[PathBuf], run: &str) -> Result<()> {
    let release = tokio::fs::read_to_string("/proc/sys/kernel/osrelease").await?;
    let release = release.trim();
    if !kernel_supports_mcap_writer(release) {
        return Err(invalid(format!(
            "MCAP recording needs Linux 6.1 or newer; this host runs {release}"
        )));
    }
    // Kernels before 6.6 have no such setting and leave io_uring enabled.
    match tokio::fs::read_to_string("/proc/sys/kernel/io_uring_disabled").await {
        Ok(value) if value.trim() != "0" => {
            return Err(invalid(
                "MCAP recording needs io_uring; set the kernel.io_uring_disabled sysctl to 0",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    for directory in directories {
        use std::os::unix::fs::OpenOptionsExt;
        // A shared directory can hold another run's leftover probe after a crash.
        let probe = directory.join(format!(".direct-io-probe-{run}"));
        match std::fs::File::options()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&probe)
        {
            Ok(_) => tokio::fs::remove_file(&probe).await?,
            Err(error) if error.raw_os_error() == Some(libc::EINVAL) => {
                return Err(invalid(format!(
                    "MCAP recording needs direct I/O; the filesystem of {} does not support it",
                    directory.display()
                )));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

impl PrepareRun {
    async fn prepare(&self, spec: ComposeRun) -> Result<PreparedRun> {
        let overlays = spec.filesystem.is_some();
        let resolved = resolve(&self.adapter, spec).await?;
        let model = mounts::model(&resolved.plan, "filesystem baseline manifest changed").await?;
        prepare_sources(&ResolvedBind::from_manifest(&model)?, overlays).await?;
        // Runtime endpoint availability is checked after execution resets the experiment.
        super::super::shared_dora::validate_endpoints(resolved.run.dora_runtime.as_ref())?;
        // Capture the filesystem baseline only for execution.
        let executor = self
            .executor
            .upgrade()
            .ok_or_else(|| invalid("executor dropped"))?;
        let baseline = executor
            .run::<CaptureFilesystemBaseline>(BaselineInput {
                bind_mounts: mounts::binds(&resolved.plan.name, &model)?,
                volume_mounts: mounts::volumes(&resolved.plan, &model)?,
                artifacts: resolved.artifacts.clone(),
            })
            .await
            .map_err(|e| invalid(e.to_string()))?
            .into_value();
        Ok(PreparedRun {
            run: resolved.run,
            artifacts: resolved.artifacts,
            plan: resolved.plan,
            system_cgroup: resolved.system_cgroup,
            system_services: resolved.system_services,
            user_services: resolved.user_services,
            mcap_services: resolved.mcap_services,
            dora: resolved.dora,
            snapshot: resolved.snapshot,
            baseline,
        })
    }
}

async fn snapshot_file(source: &str, directory: &Path) -> Result<String> {
    let source = tokio::fs::canonicalize(source).await?;
    let bytes = crate::evidence::read_regular(&source, MAX_FROZEN_FILE_BYTES).await?;
    use std::os::unix::fs::PermissionsExt;
    let permissions = tokio::fs::metadata(&source).await?.permissions();
    let destination = directory.join(format!("{}-{:o}", digest(&bytes), permissions.mode()));
    if !destination.exists() {
        private_write(&destination, &bytes).await?;
        tokio::fs::set_permissions(&destination, permissions).await?;
    }
    Ok(destination.to_string_lossy().into_owned())
}
/// Refuse unreadable bind sources, naming every failure, and leave the host unchanged on refusal.
/// `overlays` creates missing sources Docker would create, because overlays need a lower directory.
async fn prepare_sources(binds: &[ResolvedBind], overlays: bool) -> Result<()> {
    let mut failures = Vec::new();
    let mut missing = Vec::new();
    let named = |bind: &ResolvedBind, error: std::io::Error| {
        format!(
            "{}: bind mount source {}: {error}",
            bind.service, bind.source
        )
    };
    for bind in binds {
        if let Err(error) = tokio::fs::metadata(&bind.source).await {
            if !bind.create_host_path || error.kind() != std::io::ErrorKind::NotFound {
                failures.push(named(bind, error));
            } else if overlays {
                missing.push(bind);
            }
        }
    }
    failed(&failures)?;
    for bind in missing {
        if let Err(error) = tokio::fs::create_dir_all(&bind.source).await {
            failures.push(named(bind, error));
        }
    }
    failed(&failures)
}
/// Join named source failures into one validation error.
fn failed(failures: &[String]) -> Result<()> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(invalid(failures.join("; ")))
    }
}
/// Snapshot read-only bind files, config files and secret files into `artifacts/inputs`.
/// Missing bind paths stay in place for preparation; every other failure is named.
async fn freeze_files(model: &mut Value, artifacts: &Path) -> Result<()> {
    let directory = artifacts.join("inputs");
    crate::evidence::create_private_dir(&directory).await?;
    let mut failures = Vec::new();
    for (name, service) in model["services"].as_object_mut().unwrap().iter_mut() {
        if let Some(mounts) = service.get_mut("volumes").and_then(Value::as_array_mut) {
            for mount in mounts {
                if mount["type"] == "bind"
                    && mount["read_only"] == true
                    && let Some(source) = mount["source"].as_str()
                {
                    let frozen = match tokio::fs::metadata(source).await {
                        // Execution preparation validates missing bind sources.
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(error) => Err(error.into()),
                        Ok(metadata) if !metadata.is_file() => continue,
                        Ok(_) => {
                            match workload_lifecycle::files::is_host_shm_source(Path::new(source)) {
                                Ok(true) => continue,
                                Ok(false) => snapshot_file(source, &directory).await,
                                Err(error) => Err(error.into()),
                            }
                        }
                    };
                    match frozen {
                        Ok(frozen) => mount["source"] = frozen.into(),
                        Err(error) => {
                            failures.push(format!("{name}: bind mount source {source}: {error}"))
                        }
                    }
                }
            }
        }
    }
    for kind in ["configs", "secrets"] {
        if let Some(items) = model.get_mut(kind).and_then(Value::as_object_mut) {
            for (name, item) in items.iter_mut() {
                if let Some(source) = item["file"].as_str() {
                    match snapshot_file(source, &directory).await {
                        Ok(frozen) => item["file"] = frozen.into(),
                        Err(error) => {
                            failures.push(format!("{kind}/{name}: file {source}: {error}"))
                        }
                    }
                }
            }
        }
    }
    failed(&failures)
}

#[cfg(test)]
mod tests;
