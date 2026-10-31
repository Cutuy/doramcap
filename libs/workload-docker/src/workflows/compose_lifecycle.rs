//! Teardown for a resolved Compose project with durable filesystem evidence.
//!
//! The caller owns admission and the mount policy. This sequence operates on
//! existing mounts; callers with OverlayFS views must finalize those before
//! removal using the recorded workflow instead.
use crate::compose::{
    DockerAdapter, QuiesceCompose, RemoveCompose, ResolvedComposeProject, StopInput,
    filesystem::{
        CollectFilesystemChanges, CollectInput, FilesystemBaseline, FilesystemChanges,
        StorageCategory,
    },
};
use crate::evidence::write_private_once;
use orchestrator::{Input, LocalExecutor};
use std::path::Path;

/// Finalization and removal evidence. `cleanup_verified` only describes Docker
/// resource cleanup; diagnostics and metadata coverage must also be inspected.
pub struct StopReport {
    /// All known outer writers have stopped; retained resources may still exist.
    pub quiesced: bool,
    pub cleanup_verified: bool,
    pub errors: Vec<String>,
    pub changes: Option<FilesystemChanges>,
}

/// Stop writers, collect and checkpoint metadata, then remove owned resources.
/// The caller holds exclusive ownership of the project and private metadata.
/// `require_layer_evidence` rejects missing containers after a successful launch;
/// partial startup can set it false. Failures retain remaining resources and
/// return a report with incomplete cleanup. The executor needs shared steps.
pub async fn cleanup(
    executor: &LocalExecutor,
    adapter: &DockerAdapter,
    plan: &ResolvedComposeProject,
    baseline: Option<&FilesystemBaseline>,
    stop_seconds: u64,
    require_layer_evidence: bool,
) -> StopReport {
    let input = StopInput {
        services: vec![],
        allow_volume_deletion: true,
        plan: plan.clone(),
        stop_seconds,
    };
    let mut report = StopReport {
        quiesced: false,
        cleanup_verified: false,
        errors: Vec::new(),
        changes: None,
    };
    let stopped = match executor.run::<QuiesceCompose>(input.clone()).await {
        Ok(shutdown) => {
            report.errors.extend(
                shutdown
                    .value()
                    .diagnostics
                    .iter()
                    .map(|d| format!("{}: {}", d.operation, d.message)),
            );
            if !shutdown.value().stopped {
                report.errors.push("writers did not stop".into());
                return report;
            }
            report.quiesced = true;
            shutdown
        }
        Err(error) => {
            report.errors.push(format!("quiesce: {error}"));
            return report;
        }
    };
    let mut collected = None;
    if let Some(baseline) = baseline {
        let complete_replicas = match adapter.inspect(plan).await {
            Ok(state) => plan.services.iter().all(|(service, expected)| {
                state
                    .containers
                    .iter()
                    .filter(|container| &container.service == service)
                    .count()
                    == expected.replicas
            }),
            Err(error) => {
                report
                    .errors
                    .push(format!("inspect filesystem evidence: {error}"));
                return report;
            }
        };
        let metadata = match plan.manifest.parent() {
            Some(path) => path,
            None => {
                report
                    .errors
                    .push("filesystem evidence has no metadata directory".into());
                return report;
            }
        };
        let previous = match read_outer_evidence(metadata, &plan.run) {
            Ok(previous) => previous,
            Err(error) => {
                report
                    .errors
                    .push(format!("filesystem checkpoint: {error}"));
                return report;
            }
        };
        if !complete_replicas && require_layer_evidence && previous.is_none() {
            report.errors.push(
                "filesystem evidence is missing expected containers; resources retained".into(),
            );
            return report;
        }
        if let Some(previous) = previous {
            report.changes = Some(previous);
        } else {
            match executor
                .run::<CollectFilesystemChanges>(
                    Input::new(CollectInput {
                        host_views: Default::default(),
                        baseline: baseline.clone(),
                        projects: vec![plan.clone()],
                    })
                    .after(&stopped),
                )
                .await
            {
                Ok(changes) => {
                    let changes_value = changes.value().clone();
                    if complete_replicas
                        && let Err(error) = save_outer_evidence(metadata, &plan.run, &changes_value)
                    {
                        report
                            .errors
                            .push(format!("filesystem checkpoint: {error}"));
                        return report;
                    }
                    report.changes = Some(changes_value);
                    collected = Some(changes);
                }
                Err(error) => {
                    report.errors.push(format!("filesystem evidence: {error}"));
                    return report;
                }
            }
        }
    }
    let mut removal = Input::new(input).after(&stopped);
    if let Some(changes) = &collected {
        removal = removal.after(changes);
    }
    match executor.run::<RemoveCompose>(removal).await {
        Ok(cleanup) => {
            let cleanup = cleanup.into_value();
            report.cleanup_verified = cleanup.verified;
            report.errors.extend(
                cleanup
                    .diagnostics
                    .iter()
                    .map(|d| format!("{}: {}", d.operation, d.message)),
            );
            report
                .errors
                .extend(cleanup.remaining.iter().map(|r| format!("remaining: {r}")));
        }
        Err(error) => report.errors.push(format!("remove: {error}")),
    }
    report
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OuterEvidence {
    run: String,
    changes: FilesystemChanges,
}

/// Validate a completed checkpoint using files beneath the private metadata root.
pub(crate) fn read_outer_evidence(
    metadata: &Path,
    run: &str,
) -> Result<Option<FilesystemChanges>, String> {
    use std::io::Read;
    let root = crate::evidence::root(metadata).map_err(|e| e.to_string())?;
    let file = match crate::evidence::open(&root, "outer-evidence.json", false) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("outer filesystem checkpoint is too large".into());
    }
    let evidence: OuterEvidence = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if evidence.run != run || !evidence.changes.container_layers.coverage_complete {
        return Err("outer filesystem checkpoint identity or coverage is invalid".into());
    }
    let mut parent = None;
    for (category, expected_category, name) in [
        (
            &evidence.changes.host_binds,
            StorageCategory::HostBind,
            "host-binds.pb",
        ),
        (
            &evidence.changes.container_layers,
            StorageCategory::ContainerLayer,
            "container-layers.pb",
        ),
        (
            &evidence.changes.named_volumes,
            StorageCategory::NamedVolume,
            "named-volumes.pb",
        ),
    ] {
        if category.category != expected_category
            || category.manifest.file_name().and_then(|v| v.to_str()) != Some(name)
        {
            return Err("outer filesystem checkpoint manifest is invalid".into());
        }
        let directory = category
            .manifest
            .parent()
            .ok_or("filesystem manifest has no directory")?;
        if parent.is_some_and(|previous| previous != directory) {
            return Err("outer filesystem checkpoint directories differ".into());
        }
        parent = Some(directory);
        let relative = category
            .manifest
            .strip_prefix(metadata)
            .map_err(|_| "filesystem manifest escapes metadata")?;
        crate::evidence::open(
            &root,
            relative
                .to_str()
                .ok_or("filesystem manifest is not UTF-8")?,
            false,
        )
        .map_err(|e| e.to_string())?;
    }
    let directory = parent.ok_or("filesystem checkpoint has no directory")?;
    let relative = directory
        .strip_prefix(metadata)
        .map_err(|_| "filesystem checkpoint escapes metadata")?;
    let name = relative
        .to_str()
        .ok_or("filesystem checkpoint path is not UTF-8")?;
    let valid_recovery = name.strip_prefix("recovery-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    });
    if name != "filesystem" && !valid_recovery {
        return Err("outer filesystem checkpoint directory is invalid".into());
    }
    crate::evidence::open(&root, &format!("{name}/changes.json"), false)
        .map_err(|e| e.to_string())?;
    Ok(Some(evidence.changes))
}

/// Sync collected evidence before recording its completion and deleting containers.
pub(crate) fn save_outer_evidence(
    metadata: &Path,
    run: &str,
    changes: &FilesystemChanges,
) -> Result<(), String> {
    let evidence = OuterEvidence {
        run: run.to_owned(),
        changes: changes.clone(),
    };
    let directory = changes
        .container_layers
        .manifest
        .parent()
        .ok_or("filesystem evidence has no directory")?;
    let root = crate::evidence::root(directory).map_err(|e| e.to_string())?;
    for name in [
        "host-binds.pb",
        "container-layers.pb",
        "named-volumes.pb",
        "changes.json",
    ] {
        crate::evidence::open(&root, name, false)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
    }
    root.sync_all().map_err(|e| e.to_string())?;
    write_private_once(&metadata.join("outer-evidence.json"), &evidence)?;
    let metadata_root = crate::evidence::root(metadata).map_err(|e| e.to_string())?;
    metadata_root.sync_all().map_err(|e| e.to_string())?;
    read_outer_evidence(metadata, run)?
        .ok_or_else(|| "filesystem checkpoint disappeared".to_owned())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outer_evidence_requires_complete_private_regular_manifests() {
        use crate::compose::filesystem::{CategoryChanges, ChangeCounts};
        // Arrange
        let metadata = std::env::temp_dir().join(format!(
            "gallery-evidence-{:032x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = metadata.join("filesystem");
        std::fs::create_dir_all(&directory).unwrap();
        for name in [
            "host-binds.pb",
            "container-layers.pb",
            "named-volumes.pb",
            "changes.json",
        ] {
            std::fs::write(directory.join(name), b"fixture").unwrap();
        }
        let category = |category, name: &str| CategoryChanges {
            category,
            manifest: directory.join(name),
            counts: ChangeCounts::default(),
            coverage_complete: true,
            warnings: Vec::new(),
        };
        let changes = FilesystemChanges {
            host_binds: category(StorageCategory::HostBind, "host-binds.pb"),
            container_layers: category(StorageCategory::ContainerLayer, "container-layers.pb"),
            named_volumes: category(StorageCategory::NamedVolume, "named-volumes.pb"),
        };

        // Act / Assert: completed evidence can be reused only by its generation.
        assert!(read_outer_evidence(&metadata, "run").unwrap().is_none());
        save_outer_evidence(&metadata, "run", &changes).unwrap();
        assert!(read_outer_evidence(&metadata, "run").unwrap().is_some());
        assert!(read_outer_evidence(&metadata, "other").is_err());

        // Act / Assert: replacing a manifest with a symlink invalidates the checkpoint.
        let manifest = directory.join("container-layers.pb");
        std::fs::remove_file(&manifest).unwrap();
        std::os::unix::fs::symlink(directory.join("host-binds.pb"), &manifest).unwrap();
        assert!(read_outer_evidence(&metadata, "run").is_err());
        std::fs::remove_dir_all(metadata).unwrap();
    }
}
