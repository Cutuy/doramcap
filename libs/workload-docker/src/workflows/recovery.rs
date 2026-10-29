//! Stop a crashed recorded run while retaining its Docker and filesystem evidence.
use crate::compose::{
    Cleanup, DockerAdapter, ResolvedComposeProject, Result, Shutdown, StopInput,
    adapter::{digest, validate_name},
    error::invalid,
    filesystem::{CollectFilesystemChanges, CollectInput, FilesystemBaseline, FilesystemChanges},
};
use crate::evidence::read_regular;
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};
use tokio::io::AsyncReadExt;

#[derive(Deserialize)]
struct Resolution {
    plan: ResolvedComposeProject,
}

/// Trusted ownership summary created before any project starts.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryAnchor {
    pub run: String,
    /// Projects in shutdown order.
    pub projects: Vec<AnchoredProject>,
    /// SHA-256 digests of filesystem baseline files, keyed by basename.
    pub filesystem_baselines: BTreeMap<String, String>,
}

/// Project identity and the digest of its original resolution.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchoredProject {
    pub key: String,
    pub remove_volumes: bool,
    pub resolution_sha256: String,
}

/// Save an exclusive, durable anchor outside workload-accessible mounts.
/// The caller owns and protects `directory` and prevents concurrent writers.
pub(crate) async fn write_anchor(
    directory: &Path,
    artifacts: &Path,
    run: &str,
    plans: &[ResolvedComposeProject],
    baseline_files: &[PathBuf],
) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = tokio::fs::symlink_metadata(directory).await?;
    if !directory.is_absolute()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o077 != 0
        || tokio::fs::canonicalize(directory).await? != directory
        || directory.starts_with(artifacts)
    {
        return Err(invalid(
            "recovery anchor directory must be private and outside artifacts",
        ));
    }
    let mut projects = Vec::new();
    for plan in plans.iter().rev() {
        let bytes = read_regular(
            &artifacts.join(format!("{}.resolution.json", plan.key)),
            2 * 1024 * 1024,
        )
        .await?;
        projects.push(AnchoredProject {
            key: plan.key.clone(),
            remove_volumes: plan.remove_volumes,
            resolution_sha256: digest(&bytes),
        });
    }
    let mut filesystem_baselines = BTreeMap::new();
    for path in baseline_files {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| invalid("invalid baseline filename"))?;
        if path.parent() != Some(artifacts.join("filesystem").as_path())
            || filesystem_baselines.contains_key(name)
        {
            return Err(invalid("invalid or repeated baseline path"));
        }
        filesystem_baselines.insert(name.to_owned(), hash_regular(path).await?);
    }
    if !filesystem_baselines.contains_key("baseline.json") {
        return Err(invalid("filesystem baseline record is absent"));
    }
    let anchor = RecoveryAnchor {
        run: run.into(),
        projects,
        filesystem_baselines,
    };
    let bytes = serde_json::to_vec_pretty(&anchor)?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join(format!("{run}.json")))
        .await?;
    use tokio::io::AsyncWriteExt;
    file.write_all(&bytes).await?;
    file.sync_all().await?;
    tokio::fs::File::open(directory).await?.sync_all().await?;
    Ok(())
}

async fn hash_regular(path: &Path) -> Result<String> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .await?;
    if !file.metadata().await?.is_file() {
        return Err(invalid("baseline evidence is not a regular file"));
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let size = file.read(&mut buffer).await?;
        if size == 0 {
            break;
        }
        hasher.update(&buffer[..size]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Authenticate the pre-launch filesystem baseline against a trusted anchor.
///
/// The caller must protect the anchor and hold exclusive access to the run
/// directory throughout verification and subsequent collection. Missing, changed,
/// linked, extra or relocated baseline records fail closed. This does not stop
/// workloads or collect changes; writers must be quiesced before collection.
pub async fn verify_filesystem_baseline(
    artifacts: &Path,
    anchor: &RecoveryAnchor,
) -> Result<FilesystemBaseline> {
    if !artifacts.is_absolute()
        || artifacts.file_name().and_then(|name| name.to_str()) != Some(&anchor.run)
        || tokio::fs::canonicalize(artifacts).await? != artifacts
    {
        return Err(invalid("invalid baseline artifact directory"));
    }
    let directory = artifacts.join("filesystem");
    let metadata = tokio::fs::symlink_metadata(&directory).await?;
    if !metadata.is_dir() || tokio::fs::canonicalize(&directory).await? != directory {
        return Err(invalid("invalid baseline directory"));
    }
    let baseline_path = directory.join("baseline.json");
    let bytes = read_regular(&baseline_path, 16 * 1024 * 1024).await?;
    if anchor.filesystem_baselines.get("baseline.json") != Some(&digest(&bytes)) {
        return Err(invalid("filesystem baseline record changed"));
    }
    let baseline: FilesystemBaseline = serde_json::from_slice(&bytes)?;
    if baseline.directory != directory {
        return Err(invalid("filesystem baseline directory changed"));
    }
    let mut seen = HashSet::new();
    for path in baseline.recovery_files() {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| invalid("invalid baseline filename"))?;
        if path.parent() != Some(directory.as_path()) || !seen.insert(name.to_owned()) {
            return Err(invalid("invalid or repeated baseline path"));
        }
        if anchor.filesystem_baselines.get(name) != Some(&hash_regular(&path).await?) {
            return Err(invalid("filesystem baseline file changed"));
        }
    }
    if seen.len() != anchor.filesystem_baselines.len() {
        return Err(invalid("filesystem baseline file set changed"));
    }
    Ok(baseline)
}

/// An expected project, supplied by the trusted caller in shutdown order.
pub struct ExpectedProject<'a> {
    pub key: &'a str,
    pub remove_volumes: bool,
    /// SHA-256 of the original resolution, stored outside workload-writable mounts.
    pub resolution_sha256: &'a str,
}

/// Verify frozen ownership records and stop the listed projects in caller order.
///
/// `artifacts` must be the absolute, private directory named `run`. `expected`
/// must list every allowed project, with workloads before system services and
/// resolution digests retained independently of workload-writable directories.
/// The caller must exclusively control the directory and prevent the old manager
/// and other Docker clients from modifying the run throughout this operation.
///
/// All records, manifests and Engine ownership are checked before any stop. On
/// validation failure no stop is attempted. Stopping stops at the first project
/// whose shutdown cannot be verified, leaving later projects running. The returned
/// returned vector must contain one successful, stopped result for every expected
/// project. Containers, volumes and evidence are retained; this function neither
/// collects filesystem changes nor completes recovery. The stop budget is 1..=60
/// seconds per project.
pub async fn quiesce_crashed_run(
    adapter: &DockerAdapter,
    artifacts: &Path,
    run: &str,
    expected: &[ExpectedProject<'_>],
    stop_seconds: u64,
) -> Result<Vec<Result<Shutdown>>> {
    if !(1..=60).contains(&stop_seconds) {
        return Err(invalid("recovery stop budget must be 1..=60 seconds"));
    }
    let plans = verified_plans(adapter, artifacts, run, expected).await?;
    let mut results = Vec::new();
    for plan in plans {
        let dora_error = super::dora::recover(adapter, &plan).await.err();
        let result = adapter
            .quiesce(StopInput { plan, stop_seconds })
            .await
            .map(|mut shutdown| {
                if let Some(error) = dora_error {
                    shutdown.diagnostics.push(error.diagnostic("Dora shutdown"));
                }
                shutdown
            });
        let stopped = result.as_ref().is_ok_and(|shutdown| shutdown.stopped);
        results.push(result);
        if !stopped {
            break;
        }
    }
    Ok(results)
}

async fn verified_plans(
    adapter: &DockerAdapter,
    artifacts: &Path,
    run: &str,
    expected: &[ExpectedProject<'_>],
) -> Result<Vec<ResolvedComposeProject>> {
    validate_name(run)?;
    if !artifacts.is_absolute()
        || artifacts.file_name().and_then(|name| name.to_str()) != Some(run)
        || tokio::fs::symlink_metadata(artifacts)
            .await?
            .file_type()
            .is_symlink()
        || tokio::fs::canonicalize(artifacts).await? != artifacts
        || expected.is_empty()
    {
        return Err(invalid(
            "invalid recovery artifact directory or project list",
        ));
    }
    let mut keys = HashSet::new();
    let mut plans = Vec::new();
    for project in expected {
        validate_name(project.key)?;
        if !keys.insert(project.key) {
            return Err(invalid("duplicate recovery project"));
        }
        let resolution = artifacts.join(format!("{}.resolution.json", project.key));
        let bytes = read_regular(&resolution, 2 * 1024 * 1024).await?;
        if digest(&bytes) != project.resolution_sha256 {
            return Err(invalid("trusted recovery resolution digest changed"));
        }
        let plan: Resolution = serde_json::from_slice(&bytes)?;
        let plan = plan.plan;
        let manifest = artifacts.join(format!("{}.compose.json", project.key));
        if plan.run != run
            || plan.key != project.key
            || plan.name != format!("dc-{run}-{}", project.key)
            || plan.manifest != manifest
            || plan.remove_volumes != project.remove_volumes
            || digest(&read_regular(&manifest, 16 * 1024 * 1024).await?) != plan.sha256
        {
            return Err(invalid("recovery project identity or manifest changed"));
        }
        plans.push(plan);
    }
    for resource in adapter
        .discover_runs(run)
        .await?
        .into_iter()
        .filter(|r| r.run == run)
    {
        if !plans
            .iter()
            .any(|p| resource.project.as_deref() == Some(&p.name))
        {
            return Err(invalid("unexpected resource bearing this run label"));
        }
    }
    for plan in &plans {
        adapter.inspect(plan).await?;
        if !adapter.resources_owned(plan).await? {
            return Err(invalid("unowned resource in recovery project"));
        }
    }
    Ok(plans)
}

/// Collect filesystem changes from a quiesced crashed run into a fresh directory.
///
/// The caller supplies a trusted anchor, a fixed project policy, and a new private
/// output directory outside workload mounts. It must hold exclusive access to the
/// run, anchor and output throughout collection. A failed attempt retains its
/// partial evidence; retry with a different empty directory. No Docker resources
/// are removed. Missing or altered baselines and live or unowned writers fail.
/// Runs with an `overlays` directory are retained: this recovery path cannot yet
/// authenticate execution manifests, collect active views or finalize write-back.
pub async fn collect_crashed_run(
    adapter: Rc<DockerAdapter>,
    artifacts: &Path,
    anchor: &RecoveryAnchor,
    expected: &[ExpectedProject<'_>],
    output: &Path,
) -> Result<FilesystemChanges> {
    use std::os::unix::fs::PermissionsExt;
    if anchor.projects.len() != expected.len()
        || anchor.projects.iter().zip(expected).any(|(a, e)| {
            a.key != e.key
                || a.remove_volumes != e.remove_volumes
                || a.resolution_sha256 != e.resolution_sha256
        })
    {
        return Err(invalid("trusted recovery project policy changed"));
    }
    let metadata = tokio::fs::symlink_metadata(output).await?;
    if !output.is_absolute()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o077 != 0
        || tokio::fs::canonicalize(output).await? != output
        || output.starts_with(artifacts)
        || tokio::fs::read_dir(output)
            .await?
            .next_entry()
            .await?
            .is_some()
    {
        return Err(invalid(
            "recovery output must be an empty private directory outside artifacts",
        ));
    }
    let plans = verified_plans(&adapter, artifacts, &anchor.run, expected).await?;
    for plan in &plans {
        let state = adapter.inspect(plan).await?;
        if plan.services.iter().any(|(service, expected)| {
            state
                .containers
                .iter()
                .filter(|container| &container.service == service)
                .count()
                != expected.replicas
        }) {
            return Err(invalid(
                "filesystem recovery is missing expected container evidence",
            ));
        }
        if state
            .containers
            .iter()
            .any(|c| c.pid != 0 || !matches!(c.status.as_str(), "exited" | "created"))
        {
            return Err(invalid("filesystem recovery requires stopped writers"));
        }
    }
    let mut baseline = verify_filesystem_baseline(artifacts, anchor).await?;
    // The original resolution does not authenticate rewritten execution manifests
    // or active overlay views. Retain this run until overlay-aware recovery exists.
    match tokio::fs::symlink_metadata(artifacts.join("overlays")).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => {
            return Err(invalid(
                "OverlayFS recovery requires explicit finalization; run retained",
            ));
        }
    }
    baseline.directory = output.to_path_buf();
    let changes = CollectFilesystemChanges { adapter }
        .run(CollectInput {
            host_views: Default::default(),
            baseline,
            projects: plans,
        })
        .await
        .map_err(|error| invalid(error.to_string()))?;
    for name in [
        "host-binds.pb",
        "container-layers.pb",
        "named-volumes.pb",
        "changes.json",
    ] {
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(output.join(name))
            .await?;
        if !file.metadata().await?.is_file() {
            return Err(invalid("recovery evidence is not a regular file"));
        }
        file.sync_all().await?;
    }
    tokio::fs::File::open(output).await?.sync_all().await?;
    Ok(changes)
}

/// Remove stopped projects only after complete, durable filesystem evidence.
///
/// `changes` must be the successful result of [`collect_crashed_run`] for this
/// exact run, kept under the caller's exclusive control. The caller must retain
/// its trusted anchor, evidence and exclusive Engine/run lease throughout.
/// Incomplete coverage, changed ownership or any live writer rejects the entire
/// removal before it starts. Once removal starts, a failed project stops further
/// removal. Inspect each returned cleanup, its diagnostics and the result count;
/// this function does not publish artifacts or release any manager lease.
pub async fn remove_crashed_run(
    adapter: &DockerAdapter,
    artifacts: &Path,
    anchor: &RecoveryAnchor,
    expected: &[ExpectedProject<'_>],
    changes: &FilesystemChanges,
    stop_seconds: u64,
) -> Result<Vec<Cleanup>> {
    if !(1..=60).contains(&stop_seconds)
        || !changes.host_binds.coverage_complete
        || !changes.container_layers.coverage_complete
        || !changes.named_volumes.coverage_complete
    {
        return Err(invalid(
            "recovery requires complete filesystem coverage and a valid stop budget",
        ));
    }
    if anchor.projects.len() != expected.len()
        || anchor.projects.iter().zip(expected).any(|(a, e)| {
            a.key != e.key
                || a.remove_volumes != e.remove_volumes
                || a.resolution_sha256 != e.resolution_sha256
        })
    {
        return Err(invalid("trusted recovery project policy changed"));
    }
    verify_filesystem_baseline(artifacts, anchor).await?;
    let plans = verified_plans(adapter, artifacts, &anchor.run, expected).await?;
    for plan in &plans {
        if adapter
            .inspect(plan)
            .await?
            .containers
            .iter()
            .any(|c| c.pid != 0 || !matches!(c.status.as_str(), "exited" | "created"))
        {
            return Err(invalid("recovery removal requires stopped writers"));
        }
    }
    let mut results = Vec::new();
    for plan in plans {
        let cleanup = adapter.remove(StopInput { plan, stop_seconds }).await?;
        let verified = cleanup.verified && cleanup.diagnostics.is_empty();
        results.push(cleanup);
        if !verified {
            break;
        }
    }
    Ok(results)
}

include!(concat!(env!("OUT_DIR"), "/doramcap.recovery.v1.rs"));

/// Trusted paths and ownership policy for a group of recorded child runs.
/// Run IDs are `parent_run-N`, where N is 1..=999. The caller holds exclusive
/// ownership of the parent, artifacts, anchors and Docker resources.
pub struct RecoveryGroup<'a> {
    pub artifacts_root: &'a Path,
    pub anchors: &'a Path,
    pub parent_run: &'a str,
    pub policy: &'a [RecoveryProject],
    pub stop_seconds: u64,
    /// Quiesce writers and retain resources without attempting collection/removal.
    pub preserve_only: bool,
    /// Caller-selected fresh identity; an existing attempt directory is rejected.
    pub attempt: u128,
}

/// Reconcile child runs through the existing ownership, stop, collect and remove APIs.
/// Missing/changed anchors, incomplete evidence and unsupported recovery stop the
/// sequence with an error. Evidence and remaining resources are retained.
/// Completed collection is checkpointed beneath `anchors/<run>` before removal.
/// Retries reuse it after partial removal; damaged checkpoints fail closed.
pub async fn reconcile_group(
    adapter: Rc<DockerAdapter>,
    group: RecoveryGroup<'_>,
) -> std::result::Result<(), String> {
    use super::compose_lifecycle::{read_outer_evidence, save_outer_evidence};
    use std::{
        collections::BTreeSet,
        fs,
        io::Read,
        os::unix::fs::{DirBuilderExt, PermissionsExt},
    };
    let RecoveryGroup {
        artifacts_root,
        anchors,
        parent_run,
        policy,
        stop_seconds,
        preserve_only,
        attempt,
    } = group;
    let prefix = format!("{parent_run}-");
    let resources = adapter
        .discover_runs(&prefix)
        .await
        .map_err(|e| e.to_string())?;
    if resources.is_empty() {
        return Ok(());
    }
    let mut runs = BTreeSet::new();
    for resource in resources {
        let suffix = resource
            .run
            .strip_prefix(&prefix)
            .ok_or("nested run prefix changed")?;
        if suffix.is_empty()
            || suffix.len() > 3
            || suffix.starts_with('0')
            || !suffix.bytes().all(|b| b.is_ascii_digit())
            || !policy.iter().any(|project| {
                resource.project.as_deref()
                    == Some(format!("dc-{}-{}", resource.run, project.key).as_str())
            })
        {
            return Err("unexpected nested resource identity".into());
        }
        runs.insert(resource.run);
    }
    if runs.len() > 999 {
        return Err("too many nested runs".into());
    }
    if fs::canonicalize(anchors).map_err(|e| e.to_string())? != anchors {
        return Err("nested anchor directory is not canonical".into());
    }
    let anchor_dir = crate::evidence::root(anchors).map_err(|e| e.to_string())?;
    for run in runs {
        let file = crate::evidence::open(&anchor_dir, &format!("{run}.json"), false)
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("nested recovery anchor is too large".into());
        }
        let anchor: RecoveryAnchor = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if anchor.run != run
            || anchor.projects.len() != policy.len()
            || anchor
                .projects
                .iter()
                .zip(policy)
                .any(|(a, p)| a.key != p.key || a.remove_volumes != p.remove_volumes)
        {
            return Err("nested recovery anchor does not match frozen policy".into());
        }
        let expected: Vec<_> = anchor
            .projects
            .iter()
            .map(|p| ExpectedProject {
                key: &p.key,
                remove_volumes: p.remove_volumes,
                resolution_sha256: &p.resolution_sha256,
            })
            .collect();
        let artifacts = artifacts_root.join(&run);
        let shutdowns =
            quiesce_crashed_run(&adapter, &artifacts, &run, &expected, stop_seconds.min(60))
                .await
                .map_err(|e| e.to_string())?;
        if shutdowns.len() != expected.len() {
            return Err("nested shutdown stopped before every project".into());
        }
        for shutdown in shutdowns {
            let shutdown = shutdown.map_err(|e| e.to_string())?;
            if !shutdown.stopped || !shutdown.diagnostics.is_empty() {
                return Err("nested shutdown could not be cleanly verified".into());
            }
        }
        if preserve_only {
            continue;
        }
        let evidence = anchors.join(&run);
        match fs::DirBuilder::new().mode(0o700).create(&evidence) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
        let metadata = fs::symlink_metadata(&evidence).map_err(|e| e.to_string())?;
        if !metadata.is_dir()
            || metadata.permissions().mode() & 0o077 != 0
            || fs::canonicalize(&evidence).map_err(|e| e.to_string())? != evidence
        {
            return Err("nested evidence directory must be canonical and private".into());
        }
        let changes = match read_outer_evidence(&evidence, &run)? {
            Some(changes) => changes,
            None => {
                let output = evidence.join(format!("recovery-{attempt:032x}"));
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&output)
                    .map_err(|e| e.to_string())?;
                collect_crashed_run(adapter.clone(), &artifacts, &anchor, &expected, &output)
                    .await
                    .map_err(|e| e.to_string())?
            }
        };
        if !changes.host_binds.coverage_complete
            || !changes.container_layers.coverage_complete
            || !changes.named_volumes.coverage_complete
        {
            return Err("nested filesystem coverage is incomplete".into());
        }
        save_outer_evidence(&evidence, &run, &changes)?;
        let removed = remove_crashed_run(
            &adapter,
            &artifacts,
            &anchor,
            &expected,
            &changes,
            stop_seconds.min(60),
        )
        .await
        .map_err(|e| e.to_string())?;
        if removed.len() != expected.len()
            || removed
                .iter()
                .any(|r| !r.verified || !r.diagnostics.is_empty())
        {
            return Err("nested removal could not be verified".into());
        }
    }
    if preserve_only {
        return Ok(());
    }
    if !adapter
        .discover_runs(&prefix)
        .await
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        return Err("nested resources remain after recovery".into());
    }
    Ok(())
}

/// Recover one direct-mount Compose session under the caller's exclusive lease.
/// Authenticate its saved identity, stop writers, checkpoint filesystem changes,
/// and remove owned resources. Overlay runs require their existing finalization
/// path and are retained. No application results are published here.
pub async fn recover_project(
    adapter: Rc<DockerAdapter>,
    metadata: &Path,
    run: &str,
    key: &str,
    remove_volumes: bool,
    stop_seconds: u64,
) -> std::result::Result<super::compose_lifecycle::StopReport, String> {
    validate_name(run).map_err(|e| e.to_string())?;
    validate_name(key).map_err(|e| e.to_string())?;
    if !metadata.is_absolute()
        || tokio::fs::canonicalize(metadata)
            .await
            .map_err(|e| e.to_string())?
            != metadata
        || !(1..=300).contains(&stop_seconds)
    {
        return Err("invalid recovery directory or stop budget".into());
    }
    match tokio::fs::symlink_metadata(metadata.join("overlays")).await {
        Ok(_) => {
            return Err("OverlayFS recovery requires explicit finalization; run retained".into());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let bytes = read_regular(
        &metadata.join(format!("{key}.resolution.json")),
        2 * 1024 * 1024,
    )
    .await
    .map_err(|e| e.to_string())?;
    let resolution: Resolution = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let plan = resolution.plan;
    if plan.run != run
        || plan.key != key
        || plan.name != format!("dc-{run}-{key}")
        || plan.manifest != metadata.join(format!("{key}.compose.json"))
        || plan.remove_volumes != remove_volumes
        || digest(
            &read_regular(&plan.manifest, 16 * 1024 * 1024)
                .await
                .map_err(|e| e.to_string())?,
        ) != plan.sha256
    {
        return Err("stored Compose identity or manifest changed".into());
    }
    let bytes = read_regular(&metadata.join("filesystem/baseline.json"), 2 * 1024 * 1024)
        .await
        .map_err(|e| e.to_string())?;
    let mut baseline: FilesystemBaseline =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let attempt = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let output = metadata.join(format!("recovery-{attempt:032x}"));
    let mut builder = tokio::fs::DirBuilder::new();
    builder
        .mode(0o700)
        .create(&output)
        .await
        .map_err(|e| e.to_string())?;
    baseline.directory = output;
    let diagnostics = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut executor = orchestrator::LocalExecutor::default();
    super::steps::register(&mut executor, adapter.clone(), diagnostics);
    let dora_error = super::dora::recover(&adapter, &plan).await.err();
    let mut report = super::compose_lifecycle::cleanup(
        &executor,
        &adapter,
        &plan,
        Some(&baseline),
        stop_seconds,
        true,
    )
    .await;
    if let Some(error) = dora_error {
        report.errors.push(format!("Dora shutdown: {error}"));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{RecoveryAnchor, read_regular, verify_filesystem_baseline, write_anchor};
    use crate::compose::ResolvedComposeProject;
    use std::{collections::BTreeMap, os::unix::fs::PermissionsExt};

    #[tokio::test]
    async fn recovery_metadata_rejects_links_directories_and_oversize_files() {
        // Arrange
        let root = std::env::temp_dir().join(format!("recovery-metadata-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("file");
        std::fs::write(&file, b"data").unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();

        // Act
        let valid = read_regular(&file, 4).await;
        let too_large = read_regular(&file, 3).await;
        let symlink = read_regular(&link, 4).await;
        let directory = read_regular(&root, 4).await;

        // Assert
        assert_eq!(valid.unwrap(), b"data");
        assert!(too_large.is_err());
        assert!(symlink.is_err());
        assert!(directory.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn anchor_is_private_exclusive_and_ordered_before_launch() {
        // Arrange
        let root = std::env::temp_dir().join(format!("recovery-anchor-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let artifacts = root.join("run");
        let anchor_dir = root.join("anchors");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir(artifacts.join("filesystem")).unwrap();
        let baseline = artifacts.join("filesystem/baseline.json");
        let baseline_contents = serde_json::to_vec(&serde_json::json!({
            "sources": [], "directory": artifacts.join("filesystem")
        }))
        .unwrap();
        std::fs::write(&baseline, &baseline_contents).unwrap();
        std::fs::create_dir(&anchor_dir).unwrap();
        std::fs::set_permissions(&anchor_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let plans: Vec<_> = ["system", "demo"]
            .into_iter()
            .map(|key| {
                std::fs::write(artifacts.join(format!("{key}.resolution.json")), key).unwrap();
                ResolvedComposeProject {
                    run: "run".into(),
                    key: key.into(),
                    name: format!("dc-run-{key}"),
                    manifest: artifacts.join(format!("{key}.compose.json")),
                    sha256: String::new(),
                    services: BTreeMap::new(),
                    diagnostics: vec![],
                    remove_volumes: key == "demo",
                }
            })
            .collect();

        // Act
        write_anchor(
            &anchor_dir,
            &artifacts,
            "run",
            &plans,
            std::slice::from_ref(&baseline),
        )
        .await
        .unwrap();
        let repeat = write_anchor(
            &anchor_dir,
            &artifacts,
            "run",
            &plans,
            std::slice::from_ref(&baseline),
        )
        .await;
        let inside = write_anchor(
            &artifacts,
            &artifacts,
            "run",
            &plans,
            std::slice::from_ref(&baseline),
        )
        .await;
        let path = anchor_dir.join("run.json");
        let anchor: RecoveryAnchor =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let verified = verify_filesystem_baseline(&artifacts, &anchor).await;
        std::fs::write(&baseline, b"tampered").unwrap();
        let changed = verify_filesystem_baseline(&artifacts, &anchor).await;

        // Assert
        assert_eq!(anchor.run, "run");
        assert_eq!(
            anchor
                .projects
                .iter()
                .map(|p| p.key.as_str())
                .collect::<Vec<_>>(),
            ["demo", "system"]
        );
        assert_eq!(anchor.projects[0].resolution_sha256, super::digest(b"demo"));
        assert_eq!(
            anchor.filesystem_baselines["baseline.json"],
            super::digest(&baseline_contents)
        );
        assert!(verified.is_ok());
        assert!(changed.is_err());
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(repeat.is_err());
        assert!(inside.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
