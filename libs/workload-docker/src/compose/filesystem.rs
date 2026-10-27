//! Category-preserving metadata evidence for a local Compose run.
//!
//! Host binds and supported local volumes use complete metadata scans; container
//! layers use the Engine changes API. No file contents are read or exported.
//! Scans require cooperative host access and are not atomic filesystem snapshots.
use super::{
    DockerAdapter, ResolvedComposeProject, Result,
    adapter::{digest, private_write},
    error::{absent, engine, invalid},
};
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};

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
struct Snapshot {
    entries: BTreeMap<PathBuf, FileMetadata>,
    complete: bool,
    reason: Option<String>,
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
    pub projects: Vec<ResolvedComposeProject>,
    /// Existing private run artifact directory.
    pub artifacts: PathBuf,
}
/// Input to collection after all project writers have stopped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectInput {
    pub baseline: FilesystemBaseline,
    /// Attempted projects, including partially started ones.
    pub projects: Vec<ResolvedComposeProject>,
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
///
/// Returns category-specific manifest references and bounded warning summaries.
/// Expected changes and unsupported scan sources are warnings. Engine diff errors,
/// live/unowned containers or artifact failures return errors: retain resources.
/// Adapter ownership stays with the caller; no callbacks or handles are serialized.
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
/// depth; no task fanout. I/O and JSON-incompatible path errors are returned to
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
        if relative.to_str().is_none() {
            return Err(invalid(format!(
                "non-UTF8 path cannot be represented in JSON: {relative:?}"
            )));
        }
        let symlink = if m.file_type().is_symlink() {
            Some(tokio::fs::read_link(&path).await?)
        } else {
            None
        };
        if symlink.as_ref().is_some_and(|p| p.to_str().is_none()) {
            return Err(invalid(format!("non-UTF8 symlink target at {relative:?}")));
        }
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

/// Sequential JSONL change reader. Owns its file and buffers one record at a time.
///
/// Use after collection completes. No row/byte cap; memory grows to accommodate
/// an individual record. I/O and malformed-record errors are returned explicitly.
/// Dropping closes the file; each reader has an independent cursor.
pub struct FileChanges {
    lines: Lines<BufReader<tokio::fs::File>>,
}
impl FileChanges {
    /// Open a category's metadata manifest. Does not load the whole file.
    pub async fn open(manifest: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            lines: BufReader::new(tokio::fs::File::open(manifest).await?).lines(),
        })
    }
    /// Return the next change, or `None` at EOF. Preserves category and change kind.
    pub async fn next_change(&mut self) -> Result<Option<FileChange>> {
        match self.lines.next_line().await? {
            Some(line) => Ok(Some(serde_json::from_str(&line)?)),
            None => Ok(None),
        }
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
        Ok(_) => Err(invalid(
            "volume driver/options do not support direct local metadata inspection",
        )),
        Err(e) if absent(&e) => Ok(None),
        Err(e) => Err(engine("inspect volume metadata", e)),
    }
}
async fn capture(adapter: &DockerAdapter, input: BaselineInput) -> Result<FilesystemBaseline> {
    let directory = input.artifacts.join("filesystem");
    let mut builder = tokio::fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(&directory).await?;
    let mut sources: BTreeMap<String, Source> = BTreeMap::new();
    for plan in input.projects {
        let bytes = tokio::fs::read(&plan.manifest).await?;
        if digest(&bytes) != plan.sha256 {
            return Err(invalid("filesystem baseline manifest changed"));
        }
        let model: Value = serde_json::from_slice(&bytes)?;
        let services = model["services"]
            .as_object()
            .ok_or_else(|| invalid("missing services"))?;
        for (service, value) in services {
            for mount in value["volumes"].as_array().into_iter().flatten() {
                let read_only = mount["read_only"] == true;
                let raw = mount["source"].as_str().unwrap_or("");
                let (category, identity, path, reason) = match mount["type"].as_str() {
                    Some("bind") => {
                        let path = tokio::fs::canonicalize(raw)
                            .await
                            .unwrap_or_else(|_| PathBuf::from(raw));
                        let reason = if read_only {
                            Some("read-only input; no scan".into())
                        } else if excluded(&path, &input.artifacts) {
                            Some("pseudo-filesystem or managed artifact path; no scan".into())
                        } else {
                            None
                        };
                        (
                            StorageCategory::HostBind,
                            path.to_string_lossy().into_owned(),
                            Some(path),
                            reason,
                        )
                    }
                    Some("volume") => {
                        let name = model["volumes"][raw]["name"]
                            .as_str()
                            .ok_or_else(|| invalid("volume has no resolved name"))?
                            .to_owned();
                        let (path, reason) = match volume_path(adapter, &name).await {
                            Ok(p) => (p, None),
                            Err(e) => (None, Some(e.to_string())),
                        };
                        (StorageCategory::NamedVolume, name, path, reason)
                    }
                    _ => continue,
                };
                let key = format!("{category:?}:{identity}");
                let baseline = directory.join(format!("{}.baseline.json", digest(key.as_bytes())));
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
                source.consumers.push(format!(
                    "{}/{service}:{}",
                    plan.name,
                    mount["target"].as_str().unwrap_or("?")
                ));
            }
        }
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
        private_write(&source.baseline, &serde_json::to_vec(&snapshot)?).await?;
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
) -> Result<(CategoryChanges, tokio::fs::File)> {
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
            warnings: vec![],
        },
        file,
    ))
}
async fn emit(out: &mut (CategoryChanges, tokio::fs::File), row: FileChange) -> Result<()> {
    match row.kind {
        ChangeKind::Added => out.0.counts.added += 1,
        ChangeKind::Modified => out.0.counts.modified += 1,
        ChangeKind::Deleted => out.0.counts.deleted += 1,
    }
    let mut bytes = serde_json::to_vec(&row)?;
    bytes.push(b'\n');
    out.1.write_all(&bytes).await?;
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
        "host-binds.jsonl",
    )
    .await?;
    let mut volumes = writer(
        &input.baseline.directory,
        StorageCategory::NamedVolume,
        "named-volumes.jsonl",
    )
    .await?;
    let mut containers = writer(
        &input.baseline.directory,
        StorageCategory::ContainerLayer,
        "container-layers.jsonl",
    )
    .await?;
    // Verify every attempted project before scanning shared storage.
    for plan in &input.projects {
        let state = adapter.inspect(plan).await?;
        if state
            .containers
            .iter()
            .any(|c| c.pid != 0 || !matches!(c.status.as_str(), "exited" | "created"))
        {
            return Err(invalid("filesystem collection requires stopped writers"));
        }
        for container in state.containers {
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
                        consumers: vec![format!("{}/{}", plan.name, container.service)],
                        path: PathBuf::from(change.path),
                        kind,
                        before: None,
                        after: None,
                    },
                )
                .await?;
            }
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
        let before: Snapshot = serde_json::from_slice(&tokio::fs::read(&source.baseline).await?)?;
        if !before.complete {
            out.0.warnings.push(format!(
                "{}: {}",
                source.identity,
                before.reason.unwrap_or_default()
            ));
            continue;
        }
        let path = if source.category == StorageCategory::NamedVolume {
            match volume_path(adapter, &source.identity).await {
                Ok(p) => p,
                Err(e) => {
                    out.0.warnings.push(format!("{}: {e}", source.identity));
                    continue;
                }
            }
        } else {
            source.path.clone()
        };
        let after = if let Some(path) = path {
            scan(&path).await
        } else {
            incomplete("volume absent after launch; no diff available")
        };
        if !after.complete {
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
        let mut out = writer(&root, StorageCategory::HostBind, "changes.jsonl")
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
        out.1.write_all(b"invalid json\n").await.unwrap();
        out.1.flush().await.unwrap();
        // Assert
        assert!(reader.next_change().await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
