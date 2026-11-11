//! Category-preserving metadata evidence for local Docker storage.
//!
//! Host binds and supported local volumes use complete metadata scans; container
//! layers use the Engine changes API. No file contents are read or exported.
//! Scans require cooperative host access and are not atomic filesystem snapshots.
mod wire;
pub(crate) use wire::{read_snapshot, write_record, write_snapshot};

use super::{
    DockerAdapter, Result,
    adapter::{digest, private_write},
    error::{absent, engine, invalid},
};
use crate::docker_proto::{BindMount, HostView, OwnedContainer, VolumeMount};
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
};
use tokio::io::{AsyncWriteExt, BufReader};

const HOST_SHM_SOURCE: &str = "live host shared memory; no scan";

/// Storage category is retained on every record and in separate artifact files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCategory {
    HostBind,
    ContainerLayer,
    NamedVolume,
}
/// An individual path transition, never collapsed into a generic changed flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}
/// Available stat metadata. Modification detection does not prove content equality.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Unix file type and permission bits.
    pub mode: u32,
    /// Byte length reported by stat.
    pub size: u64,
    /// Owning user and group identifiers.
    pub uid: u32,
    pub gid: u32,
    /// Modification and change timestamps as seconds/nanoseconds.
    pub mtime: (i64, i64),
    pub ctime: (i64, i64),
    /// Symlink destination; never followed during traversal.
    pub symlink: Option<PathBuf>,
}
/// One change with its original category, source and consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub category: StorageCategory,
    /// Host source path, volume name or container ID.
    pub source: String,
    /// Project/service/container-target associations; shared sources appear once.
    pub consumers: Vec<String>,
    /// Source-relative path, or container-absolute path for Engine diffs.
    pub path: PathBuf,
    pub kind: ChangeKind,
    /// Unavailable for Engine changes and for absent sides of a transition.
    pub before: Option<FileMetadata>,
    pub after: Option<FileMetadata>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) entries: BTreeMap<PathBuf, FileMetadata>,
    pub(crate) complete: bool,
    pub(crate) reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Source {
    category: StorageCategory,
    identity: String,
    path: Option<PathBuf>,
    consumers: Vec<String>,
    baseline: PathBuf,
    reason: Option<String>,
}
/// References to baseline files. Owns no containers or filesystem mounts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesystemBaseline {
    sources: Vec<Source>,
    pub directory: PathBuf,
}
/// Explicit inputs to pre-launch metadata capture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaselineInput {
    /// Resolved binds, preserving each backend-provided consumer name.
    pub bind_mounts: Vec<BindMount>,
    /// Named volumes with their final Engine names.
    pub volume_mounts: Vec<VolumeMount>,
    /// Existing private run artifact directory.
    pub artifacts: PathBuf,
}
/// Input to collection after all container writers have stopped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectInput {
    /// Every active isolated view for this run, keyed by canonical, non-overlapping host roots.
    /// Collection scans the original host path when no root matches.
    #[serde(default)]
    pub host_views: Vec<HostView>,
    /// Pre-launch baseline for all in-scope mounts in this run.
    pub baseline: FilesystemBaseline,
    /// All in-scope containers, including partial starts, with ownership verified by the caller.
    pub containers: Vec<OwnedContainer>,
}
/// Counts refer to metadata transitions, not captured file contents.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangeCounts {
    pub added: u64,
    pub modified: u64,
    pub deleted: u64,
}
/// A category-specific file list and coverage report; warnings do not fail the run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryChanges {
    pub category: StorageCategory,
    pub manifest: PathBuf,
    pub counts: ChangeCounts,
    /// Whether observation of the supplied sources was complete within the scan scope.
    /// Missing in older reports defaults to false.
    #[serde(default)]
    pub coverage_complete: bool,
    /// Source exceptions and incomplete scans, distinct from file changes.
    pub warnings: Vec<String>,
}
/// Three independent categories; consumers need not infer categories from paths.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesystemChanges {
    pub host_binds: CategoryChanges,
    pub container_layers: CategoryChanges,
    pub named_volumes: CategoryChanges,
}

/// Injected adapter for metadata baseline capture. No mount or content mutation.
pub struct CaptureFilesystemBaseline {
    pub adapter: Rc<DockerAdapter>,
}
impl Step for CaptureFilesystemBaseline {
    type Input = BaselineInput;
    type Output = FilesystemBaseline;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        capture(&self.adapter, input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
/// Collect filesystem metadata after quiescence, before resource removal.
/// Returns category manifests and warnings based on the caller's inventory.
/// Retain resources on Engine errors, live containers, or artifact failures.
pub struct CollectFilesystemChanges {
    pub adapter: Rc<DockerAdapter>,
}
impl Step for CollectFilesystemChanges {
    type Input = CollectInput;
    type Output = FilesystemChanges;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        collect(&self.adapter, input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}

fn incomplete(reason: impl Into<String>) -> Snapshot {
    Snapshot {
        entries: BTreeMap::new(),
        complete: false,
        reason: Some(reason.into()),
    }
}
/// Incremental local metadata traversal with no entry, depth, size or time limit.
///
/// Open with `open`, then await `next_entry` until `None`. Paths are relative to
/// the root (the root itself is the empty path). Reads stat metadata and symlink
/// targets only. Symlinks are never followed; special entries are stat'ed safely.
/// Nested filesystems are rejected explicitly. Requires cooperative writers;
/// traversal is not an atomic snapshot.
///
/// Owns one directory handle per active depth. Memory and handles grow with
/// depth; no task fanout. Filesystem I/O errors are returned to
/// the caller. On error, abandon this traversal; already yielded entries remain
/// partial evidence. Dropping closes directory handles. Call sequentially.
pub struct MetadataWalk {
    root: PathBuf,
    device: u64,
    stack: Vec<tokio::fs::ReadDir>,
    next: Option<PathBuf>,
}
impl MetadataWalk {
    /// Stat the root and initialize traversal. Returns underlying filesystem errors.
    pub async fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let device = tokio::fs::symlink_metadata(&root).await?.dev();
        Ok(Self {
            next: Some(root.clone()),
            root,
            device,
            stack: Vec::new(),
        })
    }

    /// Yield one path and its metadata, or `None` after the entire tree is visited.
    pub async fn next_entry(&mut self) -> Result<Option<(PathBuf, FileMetadata)>> {
        let path = loop {
            if let Some(path) = self.next.take() {
                break path;
            }
            let Some(dir) = self.stack.last_mut() else {
                return Ok(None);
            };
            if let Some(entry) = dir.next_entry().await? {
                break entry.path();
            }
            self.stack.pop();
        };
        let m = tokio::fs::symlink_metadata(&path).await?;
        if m.dev() != self.device {
            return Err(invalid(format!(
                "nested filesystem excluded: {}",
                path.display()
            )));
        }
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| invalid("scan path escaped root"))?
            .to_path_buf();
        let symlink = if m.file_type().is_symlink() {
            Some(tokio::fs::read_link(&path).await?)
        } else {
            None
        };
        if m.is_dir() {
            self.stack.push(tokio::fs::read_dir(&path).await?);
        }
        Ok(Some((
            relative,
            FileMetadata {
                mode: m.mode(),
                size: m.size(),
                uid: m.uid(),
                gid: m.gid(),
                mtime: (m.mtime(), m.mtime_nsec()),
                ctime: (m.ctime(), m.ctime_nsec()),
                symlink,
            },
        )))
    }
}

/// Sequential length-delimited protobuf change reader.
///
/// Each call reads one record. Paths and symlink targets retain Unix bytes.
/// Malformed or truncated records return errors; dropping closes the file.
pub struct FileChanges {
    input: BufReader<tokio::fs::File>,
}
impl FileChanges {
    /// Open a category manifest without loading the whole file.
    pub async fn open(manifest: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            input: BufReader::new(tokio::fs::File::open(manifest).await?),
        })
    }
    /// Return the next change, or None at a clean record boundary.
    pub async fn next_change(&mut self) -> Result<Option<FileChange>> {
        wire::read_record::<_, crate::filesystem_proto::FileChange>(&mut self.input)
            .await?
            .map(wire::decode_change)
            .transpose()
    }
}

// Diff indexing deliberately allocates with source size. Traversal itself streams.
async fn scan_inner(root: &Path) -> Result<Snapshot> {
    let mut walk = MetadataWalk::open(root).await?;
    let mut entries = BTreeMap::new();
    while let Some((path, metadata)) = walk.next_entry().await? {
        entries.insert(path, metadata);
    }
    Ok(Snapshot {
        entries,
        complete: true,
        reason: None,
    })
}
async fn scan(root: &Path) -> Snapshot {
    match scan_inner(root).await {
        Ok(s) => s,
        Err(e) => incomplete(format!("metadata scan incomplete: {e}")),
    }
}
fn excluded(path: &Path, artifacts: &Path) -> bool {
    ["/proc", "/sys", "/dev"]
        .iter()
        .any(|p| path.starts_with(p))
        || path.starts_with(artifacts)
        || artifacts.starts_with(path)
}
async fn volume_path(adapter: &DockerAdapter, name: &str) -> Result<Option<PathBuf>> {
    match adapter.docker.inspect_volume(name).await {
        Ok(v)
            if v.driver == "local"
                && v.options.is_empty()
                && Path::new(&v.mountpoint).is_absolute() =>
        {
            Ok(Some(PathBuf::from(v.mountpoint)))
        }
        Ok(v) if v.driver == "local" => {
            if v.options.get("o").is_some_and(|options| {
                options
                    .split(',')
                    .any(|option| matches!(option, "bind" | "rbind"))
            }) && let Some(source) = v.options.get("device")
                && workload_lifecycle::files::is_host_shm_source(Path::new(source))?
            {
                return Ok(Some(PathBuf::from(source)));
            }
            Err(invalid(
                "volume driver/options do not support direct local metadata inspection",
            ))
        }
        Ok(_) => Err(invalid(
            "volume driver/options do not support direct local metadata inspection",
        )),
        Err(e) if absent(&e) => Ok(None),
        Err(e) => Err(engine("inspect volume metadata", e)),
    }
}
async fn capture(adapter: &DockerAdapter, input: BaselineInput) -> Result<FilesystemBaseline> {
    let directory = input.artifacts.join("filesystem");
    crate::evidence::create_private_dir(&directory).await?;
    let mut sources: BTreeMap<String, Source> = BTreeMap::new();
    let mut record = |category: StorageCategory,
                      identity: String,
                      path: Option<PathBuf>,
                      reason: Option<String>,
                      read_only: bool,
                      consumer: String| {
        let key = format!("{category:?}:{identity}");
        let baseline = directory.join(format!("{}.baseline.pb", digest(key.as_bytes())));
        let source = sources.entry(key).or_insert(Source {
            category,
            identity,
            path,
            consumers: vec![],
            baseline,
            reason: reason.clone(),
        });
        // A writable alias must be observed even if the first alias is read-only.
        if source.reason.as_deref() == Some("read-only input; no scan") && !read_only {
            source.reason = reason;
        }
        source.consumers.push(consumer);
    };
    for bind in input.bind_mounts {
        let path = tokio::fs::canonicalize(&bind.source)
            .await
            .unwrap_or_else(|_| PathBuf::from(&bind.source));
        let reason = if bind.read_only {
            Some("read-only input; no scan".into())
        } else if excluded(&path, &input.artifacts) {
            Some("pseudo-filesystem or managed artifact path; no scan".into())
        } else {
            None
        };
        record(
            StorageCategory::HostBind,
            path.to_string_lossy().into_owned(),
            Some(path),
            reason,
            bind.read_only,
            format!("{}:{}", bind.consumer, bind.target),
        );
    }
    for mount in input.volume_mounts {
        let (path, reason) = match volume_path(adapter, &mount.name).await {
            Ok(Some(path)) if workload_lifecycle::files::is_host_shm_source(&path)? => {
                (Some(path), Some(HOST_SHM_SOURCE.into()))
            }
            Ok(path) => (path, None),
            Err(error) => (None, Some(error.to_string())),
        };
        record(
            StorageCategory::NamedVolume,
            mount.name,
            path,
            reason,
            mount.read_only,
            format!("{}:{}", mount.consumer, mount.target),
        );
    }
    for source in sources.values() {
        let snapshot = if let Some(reason) = &source.reason {
            incomplete(reason)
        } else if let Some(path) = &source.path {
            scan(path).await
        } else {
            Snapshot {
                entries: BTreeMap::new(),
                complete: true,
                reason: Some(
                    "volume absent before launch; creation includes image-seeded files".into(),
                ),
            }
        };
        wire::write_snapshot(&source.baseline, &snapshot).await?;
    }
    let baseline = FilesystemBaseline {
        sources: sources.into_values().collect(),
        directory,
    };
    private_write(
        &baseline.directory.join("baseline.json"),
        &serde_json::to_vec_pretty(&baseline)?,
    )
    .await?;
    Ok(baseline)
}
async fn writer(
    directory: &Path,
    category: StorageCategory,
    name: &str,
) -> Result<(CategoryChanges, tokio::io::BufWriter<tokio::fs::File>)> {
    let path = directory.join(name);
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .await?;
    Ok((
        CategoryChanges {
            category,
            manifest: path,
            counts: ChangeCounts::default(),
            coverage_complete: true,
            warnings: vec![],
        },
        tokio::io::BufWriter::new(file),
    ))
}
async fn emit(
    out: &mut (CategoryChanges, tokio::io::BufWriter<tokio::fs::File>),
    row: FileChange,
) -> Result<()> {
    match row.kind {
        ChangeKind::Added => out.0.counts.added += 1,
        ChangeKind::Modified => out.0.counts.modified += 1,
        ChangeKind::Deleted => out.0.counts.deleted += 1,
    }
    wire::write_record(&mut out.1, &wire::change(&row)).await?;
    let counts = &out.0.counts;
    if counts.added + counts.modified + counts.deleted <= 10 {
        eprintln!(
            "warning: filesystem {:?} {:?}: {} {:?}",
            row.category, row.kind, row.source, row.path
        );
    }
    Ok(())
}
async fn collect(adapter: &DockerAdapter, input: CollectInput) -> Result<FilesystemChanges> {
    let mut host = writer(
        &input.baseline.directory,
        StorageCategory::HostBind,
        "host-binds.pb",
    )
    .await?;
    let mut volumes = writer(
        &input.baseline.directory,
        StorageCategory::NamedVolume,
        "named-volumes.pb",
    )
    .await?;
    let mut containers = writer(
        &input.baseline.directory,
        StorageCategory::ContainerLayer,
        "container-layers.pb",
    )
    .await?;
    // The backend verifies ownership; collection verifies stopped writers before diffing.
    for container in &input.containers {
        let info = adapter
            .docker
            .inspect_container(&container.id, None)
            .await
            .map_err(|e| engine(format!("inspect {}", container.id), e))?;
        let state = info
            .state
            .ok_or_else(|| invalid("Engine omitted container state"))?;
        if state.pid.unwrap_or(0) != 0
            || !matches!(
                state.status.map(|s| s.to_string()).as_deref(),
                Some("exited" | "created")
            )
        {
            return Err(invalid("filesystem collection requires stopped writers"));
        }
    }
    for container in &input.containers {
        let changes = adapter
            .docker
            .container_changes(&container.id)
            .await
            .map_err(|e| engine("container filesystem diff", e))?
            // Moby ChangesDirs returns a nil slice when no paths changed;
            // the successful HTTP response encodes it as JSON null. Engine
            // and decoding failures were propagated above, not converted.
            // github.com/moby/moby/blob/v28.0.0/pkg/archive/changes.go
            .unwrap_or_default();
        for change in changes {
            let kind = match change.kind {
                bollard::models::ChangeType::_0 => ChangeKind::Modified,
                bollard::models::ChangeType::_1 => ChangeKind::Added,
                bollard::models::ChangeType::_2 => ChangeKind::Deleted,
            };
            emit(
                &mut containers,
                FileChange {
                    category: StorageCategory::ContainerLayer,
                    source: container.id.clone(),
                    consumers: vec![container.consumer.clone()],
                    path: PathBuf::from(change.path),
                    kind,
                    before: None,
                    after: None,
                },
            )
            .await?;
        }
    }
    for source in input.baseline.sources {
        let out = if source.category == StorageCategory::HostBind {
            &mut host
        } else {
            &mut volumes
        };
        if source.category == StorageCategory::NamedVolume {
            out.0.warnings.push(format!(
                "{}: {} volume; metadata observation does not reset persistent state",
                source.identity,
                if source.path.is_some() {
                    "pre-existing"
                } else {
                    "new or unavailable"
                }
            ));
        }
        let before = wire::read_snapshot(&source.baseline).await?;
        if !before.complete {
            if (source.category == StorageCategory::NamedVolume
                && source.reason.as_deref() != Some(HOST_SHM_SOURCE))
                || source.reason.is_none()
            {
                out.0.coverage_complete = false;
            }
            out.0.warnings.push(format!(
                "{}: {}",
                source.identity,
                before.reason.unwrap_or_default()
            ));
            continue;
        }
        let path = if source.category == StorageCategory::NamedVolume {
            match volume_path(adapter, &source.identity).await {
                Ok(Some(path)) if workload_lifecycle::files::is_host_shm_source(&path)? => {
                    out.0
                        .warnings
                        .push(format!("{}: {HOST_SHM_SOURCE}", source.identity));
                    continue;
                }
                Ok(p) => p,
                Err(e) => {
                    out.0.coverage_complete = false;
                    out.0.warnings.push(format!("{}: {e}", source.identity));
                    continue;
                }
            }
        } else {
            source.path.as_ref().map(|path| {
                input
                    .host_views
                    .iter()
                    .find(|view| path.starts_with(&view.source))
                    .map(|view| {
                        Path::new(&view.view).join(path.strip_prefix(&view.source).unwrap())
                    })
                    .unwrap_or_else(|| path.clone())
            })
        };
        let after = if let Some(path) = path {
            scan(&path).await
        } else {
            incomplete("volume absent after launch; no diff available")
        };
        if !after.complete {
            out.0.coverage_complete = false;
            out.0.warnings.push(format!(
                "{}: {}",
                source.identity,
                after.reason.unwrap_or_default()
            ));
            continue;
        }
        for (path, meta) in &before.entries {
            let new = after.entries.get(path);
            if new != Some(meta) {
                emit(
                    out,
                    FileChange {
                        category: source.category,
                        source: source.identity.clone(),
                        consumers: source.consumers.clone(),
                        path: path.clone(),
                        kind: if new.is_none() {
                            ChangeKind::Deleted
                        } else {
                            ChangeKind::Modified
                        },
                        before: Some(meta.clone()),
                        after: new.cloned(),
                    },
                )
                .await?;
            }
        }
        for (path, meta) in &after.entries {
            if !before.entries.contains_key(path) {
                emit(
                    out,
                    FileChange {
                        category: source.category,
                        source: source.identity.clone(),
                        consumers: source.consumers.clone(),
                        path: path.clone(),
                        kind: ChangeKind::Added,
                        before: None,
                        after: Some(meta.clone()),
                    },
                )
                .await?;
            }
        }
    }
    for out in [&mut host, &mut containers, &mut volumes] {
        out.1.flush().await?;
        let c = &out.0.counts;
        if c.added + c.modified + c.deleted > 0 || !out.0.warnings.is_empty() {
            eprintln!(
                "warning: filesystem {:?}: added={} modified={} deleted={} coverage_notes={} manifest={}",
                out.0.category,
                c.added,
                c.modified,
                c.deleted,
                out.0.warnings.len(),
                out.0.manifest.display()
            );
        }
    }
    let result = FilesystemChanges {
        host_binds: host.0,
        container_layers: containers.0,
        named_volumes: volumes.0,
    };
    private_write(
        &input.baseline.directory.join("changes.json"),
        &serde_json::to_vec_pretty(&result)?,
    )
    .await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mount_inputs_preserve_change_record_consumers() {
        use crate::compose::adapter::tests::test_engine;
        use serde_json::json;
        // Arrange: read-only and writable aliases share the same baseline.
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let artifacts = root.path().join("artifacts");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&artifacts).unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![
                (
                    "/containers/owned/json",
                    json!({"State":{"Pid":0,"Status":"exited"}}),
                ),
                (
                    "/containers/owned/changes",
                    json!([{"Path":"/result","Kind":1}]),
                ),
            ],
        );
        let bind = |consumer: &str, target: &str, read_only| BindMount {
            consumer: consumer.into(),
            source: source.to_string_lossy().into(),
            target: target.into(),
            read_only,
            ..Default::default()
        };
        let baseline = capture(
            &adapter,
            BaselineInput {
                artifacts,
                bind_mounts: vec![
                    bind("dc-project/reader", "/in", true),
                    bind("dc-project/writer", "/out", false),
                ],
                volume_mounts: vec![],
            },
        )
        .await
        .unwrap();
        std::fs::write(source.join("result"), b"saved").unwrap();

        // Act
        let changes = collect(
            &adapter,
            CollectInput {
                host_views: vec![],
                baseline,
                containers: vec![OwnedContainer {
                    consumer: "dc-project/app".into(),
                    id: "owned".into(),
                }],
            },
        )
        .await
        .unwrap();

        // Assert
        let mut host = FileChanges::open(changes.host_binds.manifest)
            .await
            .unwrap();
        let mut result = None;
        while let Some(change) = host.next_change().await.unwrap() {
            if change.path == Path::new("result") {
                result = Some(change);
            }
        }
        let result = result.unwrap();
        assert_eq!(result.kind, ChangeKind::Added);
        assert_eq!(
            result.consumers,
            ["dc-project/reader:/in", "dc-project/writer:/out"]
        );
        let mut layers = FileChanges::open(changes.container_layers.manifest)
            .await
            .unwrap();
        let result = layers.next_change().await.unwrap().unwrap();
        assert_eq!(result.consumers, ["dc-project/app"]);
        assert_eq!(result.path, Path::new("/result"));
        assert!(layers.next_change().await.unwrap().is_none());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn live_container_rejects_collection_before_requesting_diff() {
        use crate::compose::adapter::tests::test_engine;
        use serde_json::json;
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![(
                "/containers/owned/json",
                json!({"State":{"Pid":123,"Status":"running"}}),
            )],
        );
        let baseline = capture(
            &adapter,
            BaselineInput {
                artifacts: root.path().into(),
                bind_mounts: vec![],
                volume_mounts: vec![],
            },
        )
        .await
        .unwrap();

        // Act
        let error = collect(
            &adapter,
            CollectInput {
                host_views: vec![],
                baseline,
                containers: vec![OwnedContainer {
                    consumer: "dc-project/app".into(),
                    id: "owned".into(),
                }],
            },
        )
        .await
        .unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            "filesystem collection requires stopped writers"
        );
        server.await.unwrap();
    }

    #[test]
    fn older_category_reports_do_not_claim_complete_coverage() {
        // Arrange
        let old = serde_json::json!({"category":"named_volume", "manifest":"/tmp/old",
            "counts":{"added":0,"modified":0,"deleted":0}, "warnings":[]});

        // Act
        let report: CategoryChanges = serde_json::from_value(old).unwrap();

        // Assert
        assert!(!report.coverage_complete);
    }
    #[tokio::test]
    async fn metadata_scan_does_not_follow_symlinks_and_stats_special_entries() {
        // Arrange
        let root = std::env::temp_dir().join(format!("metadata-scan-{}", std::process::id()));
        tokio::fs::create_dir(&root).await.unwrap();
        tokio::fs::write(root.join("file"), b"input").await.unwrap();
        std::os::unix::fs::symlink("/proc", root.join("link")).unwrap();
        // Act
        let before = scan(&root).await;
        // Assert
        assert!(before.complete);
        assert_eq!(before.entries.len(), 3);
        assert_eq!(
            before.entries[Path::new("link")].symlink.as_deref(),
            Some(Path::new("/proc"))
        );
        // Act
        let socket = std::os::unix::net::UnixListener::bind(root.join("socket")).unwrap();
        let with_socket = scan(&root).await;
        // Assert: stat handles sockets without reading their contents.
        assert!(with_socket.complete);
        assert!(with_socket.entries.contains_key(Path::new("socket")));
        drop(socket);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
    #[tokio::test]
    async fn complete_large_deep_scan_and_incremental_change_readback() {
        // Arrange: exceed the former entry, relative-path byte and depth caps.
        let root = std::env::temp_dir().join(format!("metadata-large-{}", std::process::id()));
        let mut leaf = root.clone();
        for _ in 0..140 {
            leaf.push("dd");
        }
        tokio::fs::create_dir_all(&leaf).await.unwrap();
        for i in 0..10_001 {
            tokio::fs::write(leaf.join(format!("{i:05}-{}", "x".repeat(195))), b"")
                .await
                .unwrap();
        }
        // Act
        let snapshot = scan(&root).await;
        // Assert
        assert!(snapshot.complete, "{:?}", snapshot.reason);
        assert_eq!(snapshot.entries.len(), 10_142);
        assert!(
            snapshot
                .entries
                .keys()
                .map(|p| p.as_os_str().len())
                .sum::<usize>()
                > 4 * 1024 * 1024
        );
        // Act: persist every row, then consume incrementally through the public API.
        let mut out = writer(&root, StorageCategory::HostBind, "changes.pb")
            .await
            .unwrap();
        for (i, (path, metadata)) in snapshot.entries.into_iter().enumerate() {
            emit(
                &mut out,
                FileChange {
                    category: StorageCategory::HostBind,
                    source: "fixture".into(),
                    consumers: vec![],
                    path,
                    kind: match i % 3 {
                        0 => ChangeKind::Added,
                        1 => ChangeKind::Modified,
                        _ => ChangeKind::Deleted,
                    },
                    before: None,
                    after: Some(metadata),
                },
            )
            .await
            .unwrap();
        }
        out.1.flush().await.unwrap();
        let mut reader = FileChanges::open(&out.0.manifest).await.unwrap();
        let mut counts = [0; 3];
        while let Some(row) = reader.next_change().await.unwrap() {
            assert_eq!(row.category, StorageCategory::HostBind);
            counts[match row.kind {
                ChangeKind::Added => 0,
                ChangeKind::Modified => 1,
                ChangeKind::Deleted => 2,
            }] += 1;
        }
        // Assert: log preview truncation does not truncate the manifest or stream.
        assert_eq!(counts, [3381, 3381, 3380]);
        // Act: a malformed next record must surface as an error, not EOF.
        out.1.write_all(&[0x80]).await.unwrap();
        out.1.flush().await.unwrap();
        // Assert
        assert!(reader.next_change().await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
    #[tokio::test]
    async fn protobuf_snapshot_and_change_stream_preserve_unix_bytes() {
        use std::os::unix::ffi::OsStringExt;
        // Arrange: real non-UTF-8 filename and symlink target.
        let root = std::env::temp_dir().join(format!("metadata-protobuf-{}", std::process::id()));
        tokio::fs::create_dir(&root).await.unwrap();
        let name = std::ffi::OsString::from_vec(b"file-\xff".to_vec());
        let target = std::ffi::OsString::from_vec(b"target-\xfe".to_vec());
        tokio::fs::write(root.join(&name), b"data").await.unwrap();
        std::os::unix::fs::symlink(&target, root.join("link")).unwrap();
        let snapshot = scan(&root).await;
        assert!(snapshot.complete, "{:?}", snapshot.reason);
        let snapshot_file = root.join("snapshot.pb");
        // Act: persist and read the complete baseline as delimited protobuf records.
        wire::write_snapshot(&snapshot_file, &snapshot)
            .await
            .unwrap();
        let decoded = wire::read_snapshot(&snapshot_file).await.unwrap();
        // Assert: both names and targets retain exact bytes.
        assert_eq!(snapshot.entries, decoded.entries);
        assert!(decoded.entries.contains_key(Path::new(&name)));
        assert_eq!(
            decoded.entries[Path::new("link")]
                .symlink
                .as_ref()
                .unwrap()
                .as_os_str(),
            target
        );
        let mut output = writer(&root, StorageCategory::HostBind, "changes.pb")
            .await
            .unwrap();
        let mut empty = FileChanges::open(&output.0.manifest).await.unwrap();
        assert!(empty.next_change().await.unwrap().is_none());
        let expected = FileChange {
            category: StorageCategory::HostBind,
            source: "fixture".into(),
            consumers: vec!["test".into()],
            path: PathBuf::from(&name),
            kind: ChangeKind::Added,
            before: None,
            after: Some(decoded.entries[Path::new("link")].clone()),
        };
        // Act: write and consume one record, then EOF.
        emit(&mut output, expected.clone()).await.unwrap();
        output.1.flush().await.unwrap();
        let mut reader = FileChanges::open(&output.0.manifest).await.unwrap();
        let actual = reader.next_change().await.unwrap().unwrap();
        assert_eq!(actual.path, expected.path);
        assert_eq!(actual.after, expected.after);
        assert_eq!(actual.category, expected.category);
        assert!(reader.next_change().await.unwrap().is_none());
        // Act: an incomplete frame is an error, not EOF.
        output.1.write_all(&[0x03, 0x01]).await.unwrap();
        output.1.flush().await.unwrap();
        assert!(reader.next_change().await.is_err());
        // Act: drop a complete final snapshot record at its boundary.
        use prost::Message;
        let mut bytes = tokio::fs::read(&snapshot_file).await.unwrap();
        let mut input = bytes.as_slice();
        let mut last_offset = 0;
        while !input.is_empty() {
            last_offset = bytes.len() - input.len();
            crate::filesystem_proto::SnapshotRecord::decode_length_delimited(&mut input).unwrap();
        }
        // Remove an entire record: the header count still detects the truncation.
        bytes.truncate(last_offset);
        tokio::fs::write(root.join("truncated.pb"), bytes)
            .await
            .unwrap();
        // Assert
        assert!(
            wire::read_snapshot(&root.join("truncated.pb"))
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
