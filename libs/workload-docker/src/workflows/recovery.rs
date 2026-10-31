//! Stop a crashed recorded run while retaining its Docker and filesystem evidence.
use crate::compose::{
    DockerAdapter, ResolvedComposeProject, Result, StopInput,
    adapter::{digest, validate_name},
    error::invalid,
    filesystem::{CollectFilesystemChanges, CollectInput, FilesystemBaseline},
};
use crate::evidence::read_regular;
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
    /// The single experiment project.
    pub project: RecoveryProject,
    pub resolution_sha256: String,
    /// SHA-256 digests of filesystem baseline files, keyed by basename.
    pub filesystem_baselines: BTreeMap<String, String>,
}

/// Save an exclusive, durable anchor outside workload-accessible mounts.
/// The caller owns and protects `directory` and prevents concurrent writers.
pub(crate) async fn write_anchor(
    directory: &Path,
    artifacts: &Path,
    run: &str,
    plan: &ResolvedComposeProject,
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
    let bytes = read_regular(
        &artifacts.join(format!("{}.resolution.json", plan.key)),
        2 * 1024 * 1024,
    )
    .await?;
    let project = RecoveryProject {
        key: plan.key.clone(),
        remove_volumes: plan.remove_volumes,
    };
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
        project,
        resolution_sha256: digest(&bytes),
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

async fn reject_legacy_dora_checkpoint(artifacts: &Path, key: &str) -> Result<()> {
    match tokio::fs::symlink_metadata(artifacts.join(format!("{key}.dora-started.json"))).await {
        Ok(_) => Err(invalid("legacy Dora recovery is unavailable; run retained")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn verified_plan(
    adapter: &DockerAdapter,
    artifacts: &Path,
    run: &str,
    expected: &RecoveryProject,
    resolution_sha256: Option<&str>,
) -> Result<ResolvedComposeProject> {
    validate_name(run)?;
    if !artifacts.is_absolute()
        || tokio::fs::symlink_metadata(artifacts)
            .await?
            .file_type()
            .is_symlink()
        || tokio::fs::canonicalize(artifacts).await? != artifacts
    {
        return Err(invalid(
            "invalid recovery artifact directory or project list",
        ));
    }
    validate_name(&expected.key)?;
    let bytes = read_regular(
        &artifacts.join(format!("{}.resolution.json", expected.key)),
        2 * 1024 * 1024,
    )
    .await?;
    if resolution_sha256.is_some_and(|hash| digest(&bytes) != hash) {
        return Err(invalid("trusted recovery resolution digest changed"));
    }
    let plan: Resolution = serde_json::from_slice(&bytes)?;
    let plan = plan.plan;
    let manifest = artifacts.join(format!("{}.compose.json", expected.key));
    if plan.run != run
        || plan.key != expected.key
        || plan.name != crate::compose::adapter::project_name(run, &expected.key, artifacts)
        || plan.manifest != manifest
        || plan.remove_volumes != expected.remove_volumes
        || digest(&read_regular(&manifest, 16 * 1024 * 1024).await?) != plan.sha256
    {
        return Err(invalid("recovery project identity or manifest changed"));
    }
    for resource in adapter
        .discover_runs(run)
        .await?
        .into_iter()
        .filter(|r| r.run == run && r.artifacts.as_deref() == Some(artifacts))
    {
        if resource.project.as_deref() != Some(&plan.name) {
            return Err(invalid("unexpected resource bearing this run label"));
        }
    }
    adapter.inspect(&plan).await?;
    if !adapter.resources_owned(&plan).await? {
        return Err(invalid("unowned resource in recovery project"));
    }
    Ok(plan)
}

include!(concat!(env!("OUT_DIR"), "/doramcap.recovery.v1.rs"));

/// Trusted paths and ownership policy for a group of recorded child runs.
/// Run IDs are `parent_run-N`, where N is 1..=999. The caller holds exclusive
/// ownership of the parent, artifacts, anchors and Docker resources.
pub struct RecoveryGroup<'a> {
    pub artifacts_root: &'a Path,
    pub anchors: &'a Path,
    pub parent_run: &'a str,
    pub policy: &'a RecoveryProject,
    pub stop_seconds: u64,
    /// Quiesce writers and retain resources without attempting collection/removal.
    pub preserve_only: bool,
    /// Caller-selected fresh identity; an existing attempt directory is rejected.
    pub attempt: u128,
}

/// Recover anchored child experiments through the shared lifecycle steps.
/// Changed ownership, damaged checkpoints and incomplete evidence retain resources.
/// Successful filesystem collection is reused after partial removal.
pub async fn reconcile_group(
    adapter: Rc<DockerAdapter>,
    group: RecoveryGroup<'_>,
) -> std::result::Result<(), String> {
    use std::{collections::BTreeSet, fs, io::Read};
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
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|r| {
            r.artifacts
                .as_ref()
                .is_some_and(|p| p.starts_with(artifacts_root))
        })
        .collect::<Vec<_>>();
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
            || resource.project.as_deref()
                != Some(
                    crate::compose::adapter::project_name(
                        &resource.run,
                        &policy.key,
                        &artifacts_root.join(&resource.run),
                    )
                    .as_str(),
                )
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
            || anchor.project.key != policy.key
            || anchor.project.remove_volumes != policy.remove_volumes
        {
            return Err("nested recovery anchor does not match frozen policy".into());
        }
        let artifacts = artifacts_root.join(&run);
        let report = recover_session(
            adapter.clone(),
            RecoverySession {
                artifacts: &artifacts,
                evidence: &anchors.join(&run),
                run: &run,
                expected: policy,
                resolution_sha256: Some(&anchor.resolution_sha256),
                anchor: Some(&anchor),
                stop_seconds,
                preserve_only,
                attempt,
            },
        )
        .await?;
        if !report.quiesced || (!preserve_only && !report.cleanup_verified) {
            return Err("nested recovery could not be verified".into());
        }
    }
    if preserve_only {
        return Ok(());
    }
    if !adapter
        .discover_runs(&prefix)
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .all(|r| {
            !r.artifacts
                .as_ref()
                .is_some_and(|p| p.starts_with(artifacts_root))
        })
    {
        return Err("nested resources remain after recovery".into());
    }
    Ok(())
}

struct RecoverySession<'a> {
    artifacts: &'a Path,
    evidence: &'a Path,
    run: &'a str,
    expected: &'a RecoveryProject,
    resolution_sha256: Option<&'a str>,
    anchor: Option<&'a RecoveryAnchor>,
    stop_seconds: u64,
    preserve_only: bool,
    attempt: u128,
}

async fn recover_session(
    adapter: Rc<DockerAdapter>,
    session: RecoverySession<'_>,
) -> std::result::Result<super::compose_lifecycle::StopReport, String> {
    use super::compose_lifecycle::{read_outer_evidence, save_outer_evidence};
    use std::os::unix::fs::PermissionsExt;
    let RecoverySession {
        artifacts,
        evidence,
        run,
        expected,
        resolution_sha256,
        anchor,
        stop_seconds,
        preserve_only,
        attempt,
    } = session;
    if !(1..=300).contains(&stop_seconds) {
        return Err("recovery requires one experiment and a stop budget in 1..=300".into());
    }
    let plan = verified_plan(&adapter, artifacts, run, expected, resolution_sha256)
        .await
        .map_err(|e| e.to_string())?;
    reject_legacy_dora_checkpoint(artifacts, &plan.key)
        .await
        .map_err(|e| e.to_string())?;
    match tokio::fs::symlink_metadata(artifacts.join("overlays")).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
        Ok(_) => {
            return Err("OverlayFS recovery requires explicit finalization; run retained".into());
        }
    }
    let mut baseline = if let Some(anchor) = anchor {
        verify_filesystem_baseline(artifacts, anchor)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let bytes = read_regular(&artifacts.join("filesystem/baseline.json"), 2 * 1024 * 1024)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_slice::<FilesystemBaseline>(&bytes).map_err(|e| e.to_string())?
    };
    if baseline.directory != artifacts.join("filesystem") {
        return Err("filesystem baseline directory changed".into());
    }
    let diagnostics = Rc::new(std::cell::RefCell::new(Vec::new()));
    let executor = super::steps::executor(
        adapter.clone(),
        diagnostics.clone(),
        |_: &orchestrator::Event| {},
        run,
    );
    let stopped = executor
        .run::<crate::compose::QuiesceCompose>(StopInput {
            plan: plan.clone(),
            services: vec![],
            allow_volume_deletion: false,
            stop_seconds,
        })
        .await
        .map_err(|e| e.to_string())?;
    if !stopped.value().stopped {
        return Err("recovery requires verified stopped writers".into());
    }
    let mut report = super::compose_lifecycle::StopReport {
        quiesced: true,
        cleanup_verified: false,
        changes: None,
        errors: stopped
            .value()
            .diagnostics
            .iter()
            .map(|d| format!("{}: {}", d.operation, d.message))
            .collect(),
    };
    if preserve_only {
        return Ok(report);
    }
    match tokio::fs::DirBuilder::new()
        .mode(0o700)
        .create(evidence)
        .await
    {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    let metadata = tokio::fs::symlink_metadata(evidence)
        .await
        .map_err(|e| e.to_string())?;
    if !metadata.is_dir()
        || metadata.permissions().mode() & 0o077 != 0
        || tokio::fs::canonicalize(evidence)
            .await
            .map_err(|e| e.to_string())?
            != evidence
    {
        return Err("recovery evidence directory must be canonical and private".into());
    }
    let changes = match read_outer_evidence(evidence, run)? {
        Some(changes) => changes,
        None => {
            if anchor.is_some() {
                let state = adapter.inspect(&plan).await.map_err(|e| e.to_string())?;
                if plan.services.iter().any(|(name, service)| {
                    state
                        .containers
                        .iter()
                        .filter(|c| &c.service == name)
                        .count()
                        != service.replicas
                }) {
                    return Err("filesystem recovery is missing expected container evidence".into());
                }
            }
            baseline.directory = evidence.join(format!("recovery-{attempt:032x}"));
            tokio::fs::DirBuilder::new()
                .mode(0o700)
                .create(&baseline.directory)
                .await
                .map_err(|e| e.to_string())?;
            executor
                .run::<CollectFilesystemChanges>(CollectInput {
                    baseline,
                    projects: vec![plan.clone()],
                    host_views: Default::default(),
                })
                .await
                .map_err(|e| e.to_string())?
                .into_value()
        }
    };
    if anchor.is_some()
        && (!changes.host_binds.coverage_complete
            || !changes.container_layers.coverage_complete
            || !changes.named_volumes.coverage_complete)
    {
        return Err("nested filesystem coverage is incomplete".into());
    }
    save_outer_evidence(evidence, run, &changes)?;
    let removed = executor
        .run::<crate::compose::RemoveCompose>(StopInput {
            allow_volume_deletion: changes.named_volumes.coverage_complete,
            services: vec![],
            plan,
            stop_seconds,
        })
        .await
        .map_err(|e| e.to_string())?;
    report.cleanup_verified = removed.value().verified;
    report.changes = Some(changes);
    report.errors.extend(
        removed
            .value()
            .diagnostics
            .iter()
            .chain(diagnostics.borrow().iter())
            .map(|d| format!("{}: {}", d.operation, d.message)),
    );
    if artifacts.join("dora/graph.yml").is_file() {
        report
            .errors
            .push("launcher interrupted Dora recording; outputs retained as incomplete".into());
    }
    Ok(report)
}

/// Recover a direct-mount experiment using saved Docker ownership.
/// Saved identity, stopped writers and durable filesystem evidence gate removal.
/// Interrupted Dora recordings remain incomplete; overlays require explicit finalization.
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
    let expected = RecoveryProject {
        key: key.into(),
        remove_volumes,
    };
    let attempt = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    recover_session(
        adapter,
        RecoverySession {
            artifacts: metadata,
            evidence: metadata,
            run,
            expected: &expected,
            resolution_sha256: None,
            anchor: None,
            stop_seconds,
            preserve_only: false,
            attempt,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{RecoveryAnchor, read_regular, verify_filesystem_baseline, write_anchor};
    use crate::compose::ResolvedComposeProject;
    use std::{collections::BTreeMap, os::unix::fs::PermissionsExt};

    #[tokio::test]
    async fn legacy_dora_checkpoint_prevents_recovery() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("legacy-dora-recovery-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();

        // Act / Assert: ordinary runs remain eligible for recovery.
        assert!(
            super::reject_legacy_dora_checkpoint(&root, "app")
                .await
                .is_ok()
        );

        // Act / Assert: retain any run with a legacy flow checkpoint, including a symlink.
        let marker = root.join("app.dora-started.json");
        std::os::unix::fs::symlink(root.join("missing"), &marker).unwrap();
        let error = super::reject_legacy_dora_checkpoint(&root, "app")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("legacy Dora recovery"));
        std::fs::remove_file(marker).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

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
    async fn anchor_is_private_exclusive_and_frozen_before_launch() {
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
        std::fs::write(artifacts.join("demo.resolution.json"), "demo").unwrap();
        let plan = ResolvedComposeProject {
            run: "run".into(),
            key: "demo".into(),
            name: "dc-run-demo".into(),
            manifest: artifacts.join("demo.compose.json"),
            sha256: String::new(),
            services: BTreeMap::new(),
            diagnostics: vec![],
            remove_volumes: true,
        };

        // Act
        write_anchor(
            &anchor_dir,
            &artifacts,
            "run",
            &plan,
            std::slice::from_ref(&baseline),
        )
        .await
        .unwrap();
        let repeat = write_anchor(
            &anchor_dir,
            &artifacts,
            "run",
            &plan,
            std::slice::from_ref(&baseline),
        )
        .await;
        let inside = write_anchor(
            &artifacts,
            &artifacts,
            "run",
            &plan,
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
        assert_eq!(anchor.project.key, "demo");
        assert_eq!(anchor.resolution_sha256, super::digest(b"demo"));
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

    #[tokio::test]
    async fn recovery_accepts_produced_identity_and_rejects_a_changed_identity() {
        // Arrange: use the actual naming producer and the actual recovery verifier.
        use crate::compose::adapter::{digest, project_name, tests::test_engine};
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("run");
        std::fs::create_dir(&artifacts).unwrap();
        let artifacts = std::fs::canonicalize(artifacts).unwrap();
        let manifest = artifacts.join("app.compose.json");
        std::fs::write(&manifest, b"{}").unwrap();
        let mut plan: ResolvedComposeProject = serde_json::from_value(serde_json::json!({
            "run":"run", "key":"app", "name":project_name("run","app",&artifacts),
            "manifest":manifest,"sha256":digest(b"{}"),"services":{},"diagnostics":[],"remove_volumes":false
        })).unwrap();
        let resolution = artifacts.join("app.resolution.json");
        std::fs::write(
            &resolution,
            serde_json::to_vec(&serde_json::json!({"plan":plan})).unwrap(),
        )
        .unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![
                ("/containers/json", serde_json::json!([])),
                ("/networks", serde_json::json!([])),
                ("/volumes", serde_json::json!({"Volumes":[]})),
                ("/containers/json", serde_json::json!([])),
                ("/networks", serde_json::json!([])),
                ("/volumes", serde_json::json!({"Volumes":[]})),
            ],
        );
        let expected = super::RecoveryProject {
            key: "app".into(),
            remove_volumes: false,
        };

        // Act / Assert: this crosses the naming producer / recovery consumer boundary.
        let recovered = super::verified_plan(&adapter, &artifacts, "run", &expected, None)
            .await
            .unwrap();
        assert_eq!(recovered.name, plan.name);
        server.await.unwrap();

        // Act / Assert: a changed stored name fails before contacting Engine.
        plan.name.push_str("-altered");
        std::fs::write(
            &resolution,
            serde_json::to_vec(&serde_json::json!({"plan":plan})).unwrap(),
        )
        .unwrap();
        let error = super::verified_plan(&adapter, &artifacts, "run", &expected, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("identity or manifest changed"));
    }
}
