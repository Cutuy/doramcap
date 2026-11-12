//! Category-preserving metadata evidence for resolved local storage.
//!
//! Host changes come from stopped uppers; fresh run outputs are listed directly.
//! Container layers use backend observations. Named volumes and unsupported mounts
//! carry coverage notes. No file contents are read or exported.
mod wire;
pub(crate) use wire::write_record;

use super::{Result, error::invalid, files::private_write};
use crate::filesystem_proto::{DirectoryMount, MetadataSource};
use crate::overlay::{self, OverlaySet};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncWriteExt, BufReader};

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
struct Source {
    category: StorageCategory,
    identity: String,
    path: Option<PathBuf>,
    consumers: Vec<String>,
    read_only: bool,
    reason: Option<String>,
}
/// Resolved mount identities and coverage. Preparation walks no source folders.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesystemSources {
    sources: Vec<Source>,
    pub directory: PathBuf,
    artifacts: PathBuf,
    /// Caller-owned record paths relative to the run, excluded from output listings.
    pub excluded: Vec<PathBuf>,
}
/// Mounts to report; preparation owns no views or backend resources.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourcesInput {
    /// Caller-owned records relative to the run, including records written during teardown.
    pub excluded: Vec<PathBuf>,
    pub bind_mounts: Vec<DirectoryMount>,
    /// Backend-owned sources; named volumes are always reported as not tracked.
    pub metadata_sources: Vec<MetadataSource>,
    pub artifacts: PathBuf,
}
/// Collection after every writer has stopped. Reading uppers requires no mounted view.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectInput {
    pub sources: FilesystemSources,
    pub overlays: Option<OverlaySet>,
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

/// Run-relative folder holding baselines, manifests and the report checkpoint.
const FILESYSTEM_DIRECTORY: &str = "filesystem";
/// Completion checkpoint, published atomically after every manifest is synced.
const REPORT_FILE: &str = "report.pb";
/// Unpublished checkpoint; linked to `REPORT_FILE` only once fully synced.
const PENDING_REPORT_FILE: &str = "report.pending.pb";

impl FilesystemChanges {
    /// Run-relative completion reference; valid only after successful collection.
    /// Full notes live in the atomically published checkpoint.
    pub fn reference(&self) -> crate::filesystem_proto::FileChangeReference {
        crate::filesystem_proto::FileChangeReference {
            checkpoint: format!("{FILESYSTEM_DIRECTORY}/{REPORT_FILE}"),
            categories: self.categories().into_iter().map(|c| c.summary()).collect(),
        }
    }
    fn categories(&self) -> [&CategoryChanges; 3] {
        [
            &self.host_binds,
            &self.container_layers,
            &self.named_volumes,
        ]
    }
}
impl CategoryChanges {
    fn summary(&self) -> crate::filesystem_proto::CategorySummary {
        crate::filesystem_proto::CategorySummary {
            category: self.category as i32,
            counts: Some(crate::filesystem_proto::ChangeCounts {
                added: self.counts.added,
                modified: self.counts.modified,
                deleted: self.counts.deleted,
            }),
            // A nameless manifest yields a reference that export refuses to open.
            manifest: format!(
                "{FILESYSTEM_DIRECTORY}/{}",
                self.manifest
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
            coverage_complete: self.coverage_complete,
            coverage_note_count: self.warnings.len() as u64,
        }
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
    excluded: Vec<PathBuf>,
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
            excluded: Vec::new(),
        })
    }

    /// Yield one path and its metadata, or `None` after the entire tree is visited.
    pub async fn next_entry(&mut self) -> Result<Option<(PathBuf, FileMetadata)>> {
        let path = loop {
            if let Some(path) = self.next.take() {
                if self
                    .excluded
                    .iter()
                    .any(|excluded| path.starts_with(self.root.join(excluded)))
                {
                    continue;
                }
                break path;
            }
            let Some(dir) = self.stack.last_mut() else {
                return Ok(None);
            };
            if let Some(entry) = dir.next_entry().await? {
                if self
                    .excluded
                    .iter()
                    .any(|excluded| entry.path().starts_with(self.root.join(excluded)))
                {
                    continue;
                }
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

/// Resolve mount identities without traversing host folders or inspecting volume contents.
/// Existing report folders fail. Missing sources retain an explicit coverage reason.
pub async fn prepare_sources(input: SourcesInput) -> Result<FilesystemSources> {
    if input.excluded.iter().any(|path| {
        path.as_os_str().is_empty()
            || path
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
    }) {
        return Err(invalid(
            "excluded record paths must be nonempty and run-relative",
        ));
    }
    let artifacts = tokio::fs::canonicalize(input.artifacts).await?;
    let directory = artifacts.join(FILESYSTEM_DIRECTORY);
    super::files::create_private_dir(&directory).await?;
    let mut sources: BTreeMap<String, Source> = BTreeMap::new();
    for bind in input.bind_mounts {
        let path = tokio::fs::canonicalize(&bind.source)
            .await
            .unwrap_or_else(|_| PathBuf::from(&bind.source));
        let key = format!("host:{}", path.display());
        let source = sources.entry(key).or_insert_with(|| Source {
            category: StorageCategory::HostBind,
            identity: path.to_string_lossy().into(),
            path: Some(path),
            consumers: Vec::new(),
            read_only: true,
            reason: None,
        });
        source.read_only &= bind.read_only;
        source
            .consumers
            .push(format!("{}:{}", bind.consumer, bind.target));
    }
    for source in input.metadata_sources {
        if source.category != crate::filesystem_proto::StorageCategory::NamedVolume as i32 {
            return Err(invalid("unsupported backend metadata source category"));
        }
        let entry = sources
            .entry(format!("volume:{}", source.identity))
            .or_insert_with(|| Source {
                category: StorageCategory::NamedVolume,
                identity: source.identity,
                path: None,
                consumers: Vec::new(),
                read_only: true,
                reason: Some("not tracked: Docker named volume".into()),
            });
        entry.read_only &= source.read_only;
        entry.consumers.push(source.consumer);
    }
    for source in sources
        .values_mut()
        .filter(|s| s.category == StorageCategory::HostBind && !s.read_only)
    {
        source.reason =
            match overlay::source_reason(source.path.as_ref().unwrap(), &artifacts).await {
                Ok(reason) => reason,
                Err(error) => Some(format!("not tracked: source unavailable: {error}")),
            };
    }
    Ok(FilesystemSources {
        sources: sources.into_values().collect(),
        directory,
        artifacts,
        excluded: input.excluded,
    })
}
/// Review notes use the same source classification as execution; no report files are created.
pub async fn coverage_notes(
    bind_mounts: &[DirectoryMount],
    metadata_sources: &[MetadataSource],
    artifacts: &Path,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    for bind in bind_mounts.iter().filter(|b| !b.read_only) {
        let source = match tokio::fs::canonicalize(&bind.source).await {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if let Some(reason) = overlay::source_reason(&source, artifacts).await? {
            if source == artifacts {
                notes.push(format!(
                    "{}:{}: {} partly listed; Console records excluded",
                    bind.consumer,
                    bind.target,
                    source.display()
                ));
            } else if reason.starts_with("not tracked") {
                notes.push(format!(
                    "{}:{}: {}: {reason}",
                    bind.consumer,
                    bind.target,
                    source.display()
                ));
            }
        }
    }
    for source in metadata_sources.iter().filter(|s| !s.read_only) {
        notes.push(format!(
            "{}: {}: not tracked: Docker named volume",
            source.consumer, source.identity
        ));
    }
    Ok(notes)
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
        &input.sources.directory,
        StorageCategory::HostBind,
        "host-binds.pb",
    )
    .await?;
    let mut volumes = writer(
        &input.sources.directory,
        StorageCategory::NamedVolume,
        "named-volumes.pb",
    )
    .await?;
    let container_layers = if let Some(category) = input.container_layers {
        category
    } else {
        let (category, mut file) = writer(
            &input.sources.directory,
            StorageCategory::ContainerLayer,
            "container-layers.pb",
        )
        .await?;
        file.flush().await?;
        category
    };
    let sources = &input.sources;
    if let Some(overlays) = &input.overlays {
        for tree in &overlays.trees {
            for mut row in overlay::changes(tree).await? {
                let absolute = tree.source.join(&row.path);
                row.consumers = sources
                    .sources
                    .iter()
                    .filter(|source| {
                        source.category == StorageCategory::HostBind
                            && source
                                .path
                                .as_ref()
                                .is_some_and(|path| absolute.starts_with(path))
                    })
                    .flat_map(|source| source.consumers.clone())
                    .collect();
                emit(&mut host, row).await?;
            }
        }
    }
    for source in &sources.sources {
        if source.read_only {
            continue;
        }
        let out = if source.category == StorageCategory::HostBind {
            &mut host
        } else {
            &mut volumes
        };
        if source.category == StorageCategory::NamedVolume {
            out.0.coverage_complete = false;
            out.0.warnings.push(format!(
                "{}: not tracked: Docker named volume ({})",
                source.identity,
                source.consumers.join(", ")
            ));
            continue;
        }
        let path = source.path.as_ref().unwrap();
        if path.starts_with(&sources.artifacts) && tokio::fs::symlink_metadata(path).await?.is_dir()
        {
            // Collapse nested output aliases; their consumers remain associated with each entry.
            if sources.sources.iter().any(|other| {
                !other.read_only
                    && other.category == StorageCategory::HostBind
                    && other.path.as_ref().is_some_and(|parent| {
                        parent != path
                            && parent.starts_with(&sources.artifacts)
                            && path.starts_with(parent)
                    })
            }) {
                continue;
            }
            let mut walk = MetadataWalk::open(path).await?;
            let owned = [
                PathBuf::from(FILESYSTEM_DIRECTORY),
                PathBuf::from("overlays"),
            ];
            for relative in sources.excluded.iter().chain(&owned) {
                let record = sources.artifacts.join(relative);
                if record.starts_with(path) {
                    walk.excluded
                        .push(record.strip_prefix(path).unwrap().into());
                }
            }
            if path == &sources.artifacts {
                out.0.coverage_complete = false;
                out.0.warnings.push(format!(
                    "{}: partly listed; Console records excluded",
                    source.identity
                ));
            }
            while let Some((relative, metadata)) = walk.next_entry().await? {
                if relative.as_os_str().is_empty() {
                    continue;
                }
                let absolute = path.join(&relative);
                let consumers = sources
                    .sources
                    .iter()
                    .filter(|other| {
                        other.category == StorageCategory::HostBind
                            && other
                                .path
                                .as_ref()
                                .is_some_and(|parent| absolute.starts_with(parent))
                    })
                    .flat_map(|other| other.consumers.clone())
                    .collect();
                emit(
                    out,
                    FileChange {
                        category: StorageCategory::HostBind,
                        source: source.identity.clone(),
                        consumers,
                        path: relative,
                        kind: ChangeKind::Added,
                        before: None,
                        after: Some(metadata),
                    },
                )
                .await?;
            }
        } else if !input
            .overlays
            .as_ref()
            .is_some_and(|set| set.trees.iter().any(|tree| path.starts_with(&tree.source)))
        {
            out.0.coverage_complete = false;
            out.0.warnings.push(format!(
                "{}: {} ({})",
                source.identity,
                source
                    .reason
                    .as_deref()
                    .unwrap_or("not tracked: no overlay"),
                source.consumers.join(", ")
            ));
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
        &input.sources.directory.join("changes.json"),
        &serde_json::to_vec_pretty(&result)?,
    )
    .await?;
    publish_report(&input.sources.directory, &result).await?;
    Ok(result)
}

/// Sync every manifest and its folder, then atomically publish the completion checkpoint.
/// Refuses to replace an existing checkpoint; a crash before linking leaves no report.
async fn publish_report(directory: &Path, result: &FilesystemChanges) -> Result<()> {
    use prost::Message;
    for category in result.categories() {
        tokio::fs::File::open(&category.manifest)
            .await?
            .sync_all()
            .await?;
    }
    let folder = tokio::fs::File::open(directory).await?;
    folder.sync_all().await?;
    let report = crate::filesystem_proto::FileChangeReport {
        state: crate::filesystem_proto::ReportState::ReportComplete as i32,
        categories: result
            .categories()
            .into_iter()
            .map(|c| crate::filesystem_proto::CategoryReport {
                summary: Some(c.summary()),
                coverage_notes: c.warnings.clone(),
            })
            .collect(),
    };
    let pending = directory.join(PENDING_REPORT_FILE);
    private_write(&pending, &report.encode_to_vec()).await?;
    tokio::fs::File::open(&pending).await?.sync_all().await?;
    tokio::fs::hard_link(&pending, directory.join(REPORT_FILE)).await?;
    tokio::fs::remove_file(pending).await?;
    folder.sync_all().await?;
    let parent = directory
        .parent()
        .ok_or_else(|| invalid("filesystem record folder has no parent"))?;
    tokio::fs::File::open(parent).await?.sync_all().await?;
    Ok(())
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
    async fn records_preserve_unix_bytes_and_reject_truncated_frames() {
        use std::os::unix::ffi::OsStringExt;
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let name = std::ffi::OsString::from_vec(b"file-\xff".to_vec());
        let target = std::ffi::OsString::from_vec(b"target-\xfe".to_vec());
        std::os::unix::fs::symlink(&target, root.path().join(&name)).unwrap();
        let mut walk = MetadataWalk::open(root.path()).await.unwrap();
        walk.next_entry().await.unwrap();
        let (path, metadata) = walk.next_entry().await.unwrap().unwrap();
        let mut out = writer(root.path(), StorageCategory::HostBind, "changes.pb")
            .await
            .unwrap();
        // Act
        emit(
            &mut out,
            FileChange {
                category: StorageCategory::HostBind,
                source: "fixture".into(),
                consumers: vec![],
                path,
                kind: ChangeKind::Added,
                before: None,
                after: Some(metadata),
            },
        )
        .await
        .unwrap();
        out.1.flush().await.unwrap();
        let mut reader = FileChanges::open(&out.0.manifest).await.unwrap();
        let row = reader.next_change().await.unwrap().unwrap();
        // Assert
        assert_eq!(row.path, Path::new(&name));
        assert_eq!(row.after.unwrap().symlink.unwrap(), Path::new(&target));
        assert!(reader.next_change().await.unwrap().is_none());
        // Act
        out.1.write_all(&[0x80]).await.unwrap();
        out.1.flush().await.unwrap();
        // Assert
        assert!(reader.next_change().await.is_err());
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use crate::filesystem_proto::StorageCategory as WireCategory;
    use prost::Message;

    #[tokio::test]
    async fn output_files_are_added_without_baselines_and_private_records_are_excluded() {
        // Arrange: mount the whole run, plus nested and read-only aliases.
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output");
        std::fs::create_dir(&output).unwrap();
        let bind = |source: &Path, read_only| DirectoryMount {
            source: source.to_string_lossy().into(),
            consumer: "app".into(),
            target: if read_only { "/read" } else { "/write" }.into(),
            read_only,
        };
        let mounts = vec![
            bind(root.path(), false),
            bind(&output, false),
            bind(&output, true),
        ];
        let notes = coverage_notes(&mounts, &[], root.path()).await.unwrap();
        assert!(notes.iter().any(|note| note.contains("partly listed")));
        let sources = prepare_sources(SourcesInput {
            artifacts: root.path().into(),
            bind_mounts: mounts,
            metadata_sources: vec![],
            excluded: vec!["console-report.json".into(), "private".into()],
        })
        .await
        .unwrap();
        assert!(!root.path().join("filesystem/baseline.json").exists());
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::write(root.path().join("private/secret"), b"private").unwrap();
        std::fs::write(root.path().join("console-report.json"), b"private").unwrap();
        for i in 0..10_050 {
            std::fs::write(output.join(format!("{i}")), b"data").unwrap();
        }
        std::os::unix::fs::symlink("/proc", output.join("link")).unwrap();
        // Act
        let changes = collect(CollectInput {
            sources,
            overlays: None,
            container_layers: None,
        })
        .await
        .unwrap();
        let checkpoint = crate::filesystem_proto::FileChangeReport::decode(
            std::fs::read(root.path().join("filesystem/report.pb"))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let mut reader = FileChanges::open(&changes.host_binds.manifest)
            .await
            .unwrap();
        let mut count = 0;
        while let Some(row) = reader.next_change().await.unwrap() {
            assert_eq!(row.kind, ChangeKind::Added);
            assert!(row.before.is_none());
            assert!(row.path.starts_with("output"));
            if row.path != Path::new("output") {
                assert_eq!(row.consumers.len(), 3);
            }
            count += 1;
        }
        // Assert: the complete list exceeds preview limits and the checkpoint agrees.
        assert_eq!(count, 10_052);
        assert_eq!(changes.host_binds.counts.added, count);
        assert_eq!(
            checkpoint.categories[0]
                .summary
                .as_ref()
                .unwrap()
                .counts
                .as_ref()
                .unwrap()
                .added,
            count
        );
        assert!(!changes.host_binds.coverage_complete);
        assert!(output.join("10049").is_file());
    }

    #[tokio::test]
    async fn direct_sources_and_named_volumes_remain_explicitly_untracked() {
        // Arrange: a file mount, pseudo-filesystem and Docker volume; no content reads needed.
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("input");
        std::fs::write(&file, b"initial").unwrap();
        let artifacts = root.path().join("run");
        std::fs::create_dir(&artifacts).unwrap();
        let mounts = [&file, Path::new("/proc")]
            .into_iter()
            .map(|source| DirectoryMount {
                source: source.to_string_lossy().into(),
                consumer: "app".into(),
                target: source.to_string_lossy().into(),
                read_only: false,
            })
            .collect::<Vec<_>>();
        let volumes = vec![MetadataSource {
            category: WireCategory::NamedVolume as i32,
            identity: "data".into(),
            consumer: "app:/data".into(),
            ..Default::default()
        }];
        let notes = coverage_notes(&mounts, &volumes, &artifacts).await.unwrap();
        assert_eq!(notes.len(), 3);
        let sources = prepare_sources(SourcesInput {
            artifacts,
            bind_mounts: mounts,
            metadata_sources: volumes,
            excluded: vec![],
        })
        .await
        .unwrap();
        std::fs::write(&file, b"direct").unwrap();
        // Act
        let changes = collect(CollectInput {
            sources,
            overlays: None,
            container_layers: None,
        })
        .await
        .unwrap();
        // Assert
        assert_eq!(changes.host_binds.warnings.len(), 2);
        assert!(!changes.host_binds.coverage_complete && !changes.named_volumes.coverage_complete);
        assert!(changes.named_volumes.warnings[0].contains("not tracked: Docker named volume"));
        assert_eq!(
            changes.host_binds.counts.added
                + changes.host_binds.counts.modified
                + changes.host_binds.counts.deleted,
            0
        );
        assert_eq!(std::fs::read(file).unwrap(), b"direct");
    }
}
