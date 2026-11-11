//! Category-preserving metadata evidence for resolved local storage.
//!
//! Host binds and supported local volumes use complete metadata scans; container
//! layers use observations supplied by the backend. No file contents are read or exported.
//! Scans require cooperative host access and are not atomic filesystem snapshots.
mod wire;
pub(crate) use wire::{read_snapshot, write_record, write_snapshot};

use super::{
    Result,
    error::invalid,
    files::{digest, private_write},
};
use crate::filesystem_proto::{DirectoryMount, HostView, MetadataSource, SourceObservation};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncWriteExt, BufReader};

pub const HOST_SHM_SOURCE: &str = "live host shared memory; no scan";

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
/// Explicit inputs to pre-launch metadata capture; owns no mounts or backend resources.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaselineInput {
    /// Resolved binds, preserving each backend-provided consumer name.
    pub bind_mounts: Vec<DirectoryMount>,
    /// Additional backend-resolved metadata sources with category, access and coverage.
    pub metadata_sources: Vec<MetadataSource>,
    /// Existing private run artifact directory.
    pub artifacts: PathBuf,
}
/// Input to collection after the backend has verified every writer stopped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectInput {
    /// Every active isolated view for this run, keyed by canonical, non-overlapping host roots.
    /// Collection scans the original host path when no root matches.
    #[serde(default)]
    pub host_views: Vec<HostView>,
    /// Pre-launch baseline for all in-scope mounts in this run.
    pub baseline: FilesystemBaseline,
    /// Resolved post-run metadata paths for backend-owned sources.
    pub observations: Vec<SourceObservation>,
    /// Completed backend layer observation; absent backends produce an empty category.
    pub container_layers: Option<CategoryChanges>,
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
/// Independent category reports; existing category values and record formats remain stable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesystemChanges {
    pub host_binds: CategoryChanges,
    pub container_layers: CategoryChanges,
    pub named_volumes: CategoryChanges,
}

fn incomplete(reason: impl Into<String>) -> Snapshot {
    Snapshot {
        entries: BTreeMap::new(),
        complete: false,
        reason: Some(reason.into()),
    }
}
/// Streaming stat/symlink traversal; paths are relative, with an empty root path.
/// No capture limits; handles grow with depth and diff indexes with source size. Never follows symlinks.
/// Cooperative writers required. Nested filesystems/I/O errors leave partial evidence; drop closes handles.
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

/// Sequential length-delimited protobuf change reader; each call reads one record.
/// Paths and symlink targets retain Unix bytes.
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
/// Save complete baselines or explicit exclusions before writers start; existing records fail.
pub async fn capture(input: BaselineInput) -> Result<FilesystemBaseline> {
    let directory = input.artifacts.join("filesystem");
    super::files::create_private_dir(&directory).await?;
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
    for source in input.metadata_sources {
        let category = match source.category {
            x if x == crate::filesystem_proto::StorageCategory::NamedVolume as i32 => {
                StorageCategory::NamedVolume
            }
            _ => return Err(invalid("unsupported backend metadata source category")),
        };
        record(
            category,
            source.identity,
            source.path.map(PathBuf::from),
            source.reason,
            source.read_only,
            source.consumer,
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
/// Streaming category manifest; dropping before finish leaves incomplete evidence.
pub struct ChangeWriter((CategoryChanges, tokio::io::BufWriter<tokio::fs::File>));
impl ChangeWriter {
    /// Create a fresh private manifest for the supplied storage category.
    pub async fn create(directory: &Path, category: StorageCategory, name: &str) -> Result<Self> {
        Ok(Self(writer(directory, category, name).await?))
    }
    /// Append one transition and update counts; errors prevent a completed report.
    pub async fn emit(&mut self, row: FileChange) -> Result<()> {
        emit(&mut self.0, row).await
    }
    /// Flush the complete category and return its metadata reference.
    pub async fn finish(mut self) -> Result<CategoryChanges> {
        self.0.1.flush().await?;
        log_category(&self.0.0);
        Ok(self.0.0)
    }
}
impl FilesystemBaseline {
    /// Resolve only complete backend sources; failed baselines cannot establish changes.
    pub async fn pending_sources(&self, category: StorageCategory) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for source in &self.sources {
            if source.category == category && wire::read_snapshot(&source.baseline).await?.complete
            {
                names.push(source.identity.clone());
            }
        }
        Ok(names)
    }
}

fn log_category(category: &CategoryChanges) {
    let c = &category.counts;
    if c.added + c.modified + c.deleted > 0 || !category.warnings.is_empty() {
        eprintln!(
            "warning: filesystem {:?}: added={} modified={} deleted={} coverage_notes={} manifest={}",
            category.category,
            c.added,
            c.modified,
            c.deleted,
            category.warnings.len(),
            category.manifest.display()
        );
    }
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
/// Save category manifests and coverage after quiescence; failures prevent finalization.
pub async fn collect(input: CollectInput) -> Result<FilesystemChanges> {
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
    let container_layers = if let Some(category) = input.container_layers {
        category
    } else {
        let (category, mut file) = writer(
            &input.baseline.directory,
            StorageCategory::ContainerLayer,
            "container-layers.pb",
        )
        .await?;
        file.flush().await?;
        category
    };
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
            let observation = input
                .observations
                .iter()
                .find(|o| o.identity == source.identity)
                .ok_or_else(|| invalid("backend metadata observation missing"))?;
            if let Some(reason) = &observation.reason {
                if reason != HOST_SHM_SOURCE {
                    out.0.coverage_complete = false;
                }
                out.0
                    .warnings
                    .push(format!("{}: {reason}", source.identity));
                continue;
            }
            observation.path.as_ref().map(PathBuf::from)
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
    for out in [&mut host, &mut volumes] {
        out.1.flush().await?;
        log_category(&out.0);
    }
    let result = FilesystemChanges {
        host_binds: host.0,
        container_layers,
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
