//! Per-run Linux OverlayFS views for resolved directory binds.
//!
//! Requires backend-qualified mount visibility and
//! CAP_SYS_ADMIN. Caller must stop all writers before finalization. Explicit
//! preparation/finalization own mounts; dropping serialized values does no cleanup.
mod direct;

use super::{Result, changes::write_record, error::invalid, files::private_write};
use crate::filesystem_proto::{DirectoryMount, SourceKind};

/// Run-relative directory owned by overlay preparation and finalization.
pub const RECORDS_DIRECTORY: &str = "overlays";
/// Run-relative root of retained copies.
pub const KEPT_DIRECTORY: &str = "kept";
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    os::fd::AsRawFd,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
    },
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

/// Disposition after all writers stop. Default reproduces persistent host output.
/// Keep retains copies in the run's `kept/<tree index>/` directories.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    #[default]
    WriteBack,
    Keep,
    Discard,
}
impl Disposition {
    /// Resolve the shared policy; omitted values preserve write-back and unknown values fail.
    pub fn from_policy(value: i32) -> Result<Self> {
        use crate::filesystem_proto::WritePolicy;
        match WritePolicy::try_from(value) {
            Ok(WritePolicy::Unspecified | WritePolicy::WriteBack) => Ok(Self::WriteBack),
            Ok(WritePolicy::Keep) => Ok(Self::Keep),
            Ok(WritePolicy::Discard) => Ok(Self::Discard),
            Err(_) => Err(invalid("unknown write_policy")),
        }
    }
    /// Concrete policy written into the immutable filesystem checkpoint.
    pub fn write_policy(&self) -> crate::filesystem_proto::WritePolicy {
        use crate::filesystem_proto::WritePolicy;
        match self {
            Self::WriteBack => WritePolicy::WriteBack,
            Self::Keep => WritePolicy::Keep,
            Self::Discard => WritePolicy::Discard,
        }
    }
}
/// Per-run filesystem policy. Directory binds are isolated by default.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayPolicy {
    #[serde(default)]
    /// Write back, keep in the run folder, or discard upper changes.
    pub disposition: Disposition,
    /// Host folders whose equal, ancestor or descendant mounts stay directly bound.
    #[serde(default)]
    pub passthrough_folders: Vec<String>,
}
/// One shared tree and its private run-owned mount/staging locations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayTree {
    /// Original host directory backing this view.
    pub source: PathBuf,
    /// Fresh copy-up layer owned by this run.
    pub upper: PathBuf,
    /// Fresh kernel scratch directory on the upper filesystem.
    pub work: PathBuf,
    /// Mountpoint exposed through the backend launch.
    pub merged: PathBuf,
    /// Private action journals and write-back diagnostics.
    pub records: PathBuf,
}
/// Explicit original/effective bind mapping, including read-only aliases.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountMapping {
    /// Backend-selected display name for the bind consumer.
    pub consumer: String,
    /// Unchanged application destination.
    pub target: String,
    /// Canonical user-specified source.
    pub original: PathBuf,
    /// Source used by the generated execution manifest.
    pub effective: PathBuf,
    /// Original access flag retained at the application bind.
    pub read_only: bool,
    /// Reason for redirecting or preserving this source.
    pub reason: String,
}
/// Prepared views; the caller finalizes them or discards them on the next reset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlaySet {
    /// Private artifact directory for ownership and result records.
    pub directory: PathBuf,
    /// Temporary OverlayFS root containing each tree's `upper`, `work` and `merged` directories.
    /// Reset discards it after stopping writers; retained artifacts live outside this root.
    pub workspace: PathBuf,
    /// Non-overlapping shared directory views.
    pub trees: Vec<OverlayTree>,
    /// Original and effective sources for every bind consumer.
    pub mappings: Vec<MountMapping>,
    /// Filesystem disposition selected for this run.
    pub policy: OverlayPolicy,
}
/// Resolved bind mounts in one serial run; sources must already be prepared.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareInput {
    /// Existing fresh run directory, outside all lower trees.
    pub artifacts: PathBuf,
    /// Temporary OverlayFS root for each tree's `upper`, `work` and `merged` directories.
    /// Must be fresh, outside source trees, and have an existing canonical parent.
    pub workspace: PathBuf,
    /// Bind sources, destinations and access settings supplied by the backend.
    pub bind_mounts: Vec<DirectoryMount>,
    /// Filesystem disposition selected for this run.
    pub policy: OverlayPolicy,
}
/// Final result locations; discard returns no paths.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayResults {
    /// Write back, keep in the run folder, or discard upper changes.
    pub disposition: Disposition,
    /// Host sources after write-back, kept folders after keep; empty after discard.
    pub paths: Vec<PathBuf>,
    /// True only after every owned mount has been strictly unmounted.
    pub unmounted: bool,
    /// Coverage limitations for this disposition.
    #[serde(default)]
    pub warnings: Vec<String>,
}
fn cstr(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("NUL in mount path"))
}
async fn mount(tree: &OverlayTree) -> Result<()> {
    let tree = tree.clone();
    tokio::task::spawn_blocking(move || {
        // Kernel path lookup through open directory FDs preserves literal option characters.
        let source = std::fs::File::open(&tree.source)?;
        let upper = std::fs::File::open(&tree.upper)?;
        let work = std::fs::File::open(&tree.work)?;
        let target = cstr(&tree.merged)?;
        let options = CString::new(format!(
            "lowerdir=/proc/self/fd/{},upperdir=/proc/self/fd/{},workdir=/proc/self/fd/{},index=on,metacopy=off,redirect_dir=off,xino=on",
            source.as_raw_fd(),
            upper.as_raw_fd(),
            work.as_raw_fd()
        ))
        .unwrap();
        // SAFETY: all pointers are live NUL-terminated strings; kernel copies them.
        if unsafe {
            libc::mount(
                c"overlay".as_ptr(),
                target.as_ptr(),
                c"overlay".as_ptr(),
                0,
                options.as_ptr().cast(),
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    })
    .await
    .map_err(|e| invalid(e.to_string()))?
}
/// Strictly unmount a view owned by the caller; a busy mount returns its OS error.
pub async fn unmount(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let target = cstr(&path)?;
        // SAFETY: valid path pointer; flags zero deliberately forbids lazy detach.
        if unsafe { libc::umount2(target.as_ptr(), 0) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    })
    .await
    .map_err(|e| invalid(e.to_string()))?
}
fn unescape_mount(s: &str) -> PathBuf {
    PathBuf::from(
        s.replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\"),
    )
}
async fn mounts() -> Result<Vec<PathBuf>> {
    Ok(tokio::fs::read_to_string("/proc/self/mountinfo")
        .await?
        .lines()
        .filter_map(|l| l.split_whitespace().nth(4))
        .map(unescape_mount)
        .collect())
}
/// Resolved sources awaiting backend validation and native mount preparation.
/// Owns no filesystem resources; dropping it needs no cleanup.
pub struct OverlayPlan {
    views: OverlaySet,
    roots: Vec<PathBuf>,
    names: BTreeMap<PathBuf, std::ffi::OsString>,
}
impl OverlayPlan {
    /// Canonical mappings in backend input order, before view redirection.
    pub fn mappings(&self) -> &[MountMapping] {
        &self.views.mappings
    }
    /// Non-overlapping roots selected for shared writable and read-only aliases.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }
    /// Name a source's view after its host folder; the name must be one normal path component.
    /// Call before preparation. Unknown roots and path traversal return errors.
    pub fn name_view(&mut self, source: &Path, name: &std::ffi::OsStr) -> Result<()> {
        let mut parts = Path::new(name).components();
        if !self.roots.iter().any(|root| root == source)
            || !matches!(parts.next(), Some(std::path::Component::Normal(_)))
            || parts.next().is_some()
        {
            return Err(invalid(
                "view name requires a known root and one normal path component",
            ));
        }
        self.names.insert(source.into(), name.into());
        Ok(())
    }
}

fn set_readonly_ancestors(mappings: &[MountMapping], writable: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    mappings
        .iter()
        .filter(|m| m.read_only && writable.iter().any(|p| p.starts_with(&m.original)))
        .map(|m| m.original.clone())
        .collect()
}

/// Resolve views without walking source folders or creating mounts.
/// Unsupported sources and run outputs stay directly mounted with a coverage reason.
/// Sources must exist; mutable workspace overlaps and unsafe alias sharing fail.
pub async fn resolve(input: PrepareInput) -> Result<OverlayPlan> {
    let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
    let directory = artifacts.join(RECORDS_DIRECTORY);
    let workspace = input.workspace;
    let parent = workspace
        .parent()
        .ok_or_else(|| invalid("workspace parent missing"))?;
    if !workspace.is_absolute()
        || tokio::fs::canonicalize(parent).await? != parent
        || workspace.starts_with(&directory)
        || artifacts.starts_with(&workspace)
    {
        return Err(invalid(
            "overlay workspace must be canonical and separate from records",
        ));
    }
    let (mappings, roots) = resolve_sources(
        &input.bind_mounts,
        &artifacts,
        &input.policy.passthrough_folders,
    )
    .await?;
    for mapping in &mappings {
        if mapping.original.starts_with(&workspace)
            || roots.iter().any(|root| workspace.starts_with(root))
        {
            return Err(invalid("overlay source overlaps the mutable workspace"));
        }
    }
    let set = OverlaySet {
        directory,
        workspace,
        trees: vec![],
        mappings,
        policy: input.policy,
    };
    Ok(OverlayPlan {
        views: set,
        roots,
        names: BTreeMap::new(),
    })
}

/// Resolve existing bind identities and reject aliases that cannot share one view.
/// Creates no files or mounts; Review and native preparation share these decisions.
pub async fn resolve_sources(
    bind_mounts: &[DirectoryMount],
    artifacts: &Path,
    passthrough_folders: &[String],
) -> Result<(Vec<MountMapping>, Vec<PathBuf>)> {
    let mountpoints = mounts().await?;
    let mut writable = BTreeSet::new();
    let mut mappings = Vec::new();
    for bind in bind_mounts {
        let source = tokio::fs::canonicalize(&bind.source)
            .await
            .map_err(|error| {
                invalid(format!(
                    "{}: bind mount source {}: {error}",
                    bind.consumer, bind.source
                ))
            })?;
        let (kind, reason) =
            classify_source(&source, artifacts, &mountpoints, passthrough_folders).await?;
        if !bind.read_only && kind == SourceKind::Overlay {
            writable.insert(source.clone());
        }
        mappings.push(MountMapping {
            consumer: bind.consumer.clone(),
            target: bind.target.clone(),
            original: source.clone(),
            effective: source,
            read_only: bind.read_only,
            reason: reason.unwrap_or_else(|| {
                if bind.read_only {
                    "read-only input"
                } else {
                    "OverlayFS"
                }
                .into()
            }),
        });
    }
    // A read-only ancestor must see mutations through a writable child alias.
    // Include it in the shared tree while retaining read_only on its bind target.
    let aliases: Vec<_> = set_readonly_ancestors(&mappings, &writable);
    for alias in aliases {
        let (kind, _) =
            classify_source(&alias, artifacts, &mountpoints, passthrough_folders).await?;
        if kind != SourceKind::Overlay {
            return Err(invalid(format!(
                "read-only ancestor cannot share isolated view: {}",
                alias.display()
            )));
        }
        writable.insert(alias);
    }
    let roots: Vec<_> = writable
        .iter()
        .filter(|p| !writable.iter().any(|q| q != *p && p.starts_with(q)))
        .cloned()
        .collect();
    // Every overlapping alias must use the same view before any mount is created.
    for mapping in &mappings {
        for root in &roots {
            if root.starts_with(&mapping.original) && root != &mapping.original {
                return Err(invalid(format!(
                    "{}:{}: direct ancestor {} cannot share isolated view {}",
                    mapping.consumer,
                    mapping.target,
                    mapping.original.display(),
                    root.display()
                )));
            }
            if !mapping.read_only && mapping.original.starts_with(root) {
                let (kind, _) = classify_source(
                    &mapping.original,
                    artifacts,
                    &mountpoints,
                    passthrough_folders,
                )
                .await?;
                if kind != SourceKind::Overlay {
                    return Err(invalid(format!(
                        "{}:{}: direct source {} cannot share isolated view {}",
                        mapping.consumer,
                        mapping.target,
                        mapping.original.display(),
                        root.display()
                    )));
                }
            }
        }
    }
    Ok((mappings, roots))
}

/// Validate native source constraints, then mount resolved views and save ownership.
/// Callers validate backend-specific options before this first mount attempt.
pub async fn prepare(plan: OverlayPlan) -> Result<OverlaySet> {
    let OverlayPlan {
        views: mut set,
        roots,
        names,
    } = plan;
    let directory = set.directory.clone();
    let workspace = set.workspace.clone();
    super::files::create_private_dir(&directory).await?;
    super::files::create_private_dir(&workspace).await?;
    for (i, source) in roots.into_iter().enumerate() {
        let base = directory.join(i.to_string());
        tokio::fs::create_dir(&base).await?;
        let view = workspace.join(i.to_string());
        tokio::fs::create_dir(&view).await?;
        let merged = names.get(&source).map_or_else(
            || view.join("merged"),
            |name| view.join("merged").join(name),
        );
        tokio::fs::create_dir_all(&merged).await?;
        let tree = OverlayTree {
            source,
            upper: view.join("upper"),
            work: view.join("work"),
            merged,
            records: base.clone(),
        };
        for path in [&tree.upper, &tree.work] {
            tokio::fs::create_dir(path).await?;
        }
        let root_list = base.join("root.paths0");
        list_file(&root_list, &BTreeSet::from([PathBuf::new()])).await?;
        rsync(
            &tree.source,
            &tree.upper,
            Some(&root_list),
            &base.join("root.log"),
            true,
            false,
        )
        .await?;
        set.trees.push(tree);
    }
    private_write(
        &directory.join("plan.json"),
        &serde_json::to_vec_pretty(&set)?,
    )
    .await?;
    let mut mounted = Vec::new();
    let result = async {
        for tree in &set.trees {
            mount(tree).await?;
            mounted.push(tree.merged.clone());
            if !mounts().await?.contains(&tree.merged) {
                return Err(invalid("overlay mount absent after mount syscall"));
            }
        }
        for mapping in &mut set.mappings {
            if let Some(tree) = set
                .trees
                .iter()
                .find(|t| mapping.original.starts_with(&t.source))
            {
                mapping.effective = tree
                    .merged
                    .join(mapping.original.strip_prefix(&tree.source).unwrap());
                mapping.reason = "shared OverlayFS view".into();
            }
        }
        private_write(
            &directory.join("prepared.json"),
            &serde_json::to_vec_pretty(&set)?,
        )
        .await?;
        Ok::<(), super::Error>(())
    }
    .await;
    if let Err(e) = result {
        let mut errors = vec![e.to_string()];
        for path in mounted.iter().rev() {
            if let Err(e) = unmount(path).await {
                errors.push(format!("rollback {}: {e}", path.display()));
            }
        }
        return Err(invalid(errors.join("; ")));
    }
    Ok(set)
}

// Stream rsync diagnostics into private artifacts; no shell, timeout or output cap.
async fn rsync(
    source: &Path,
    target: &Path,
    files: Option<&Path>,
    log: &Path,
    root_only: bool,
    basic: bool,
) -> Result<()> {
    let stdout = std::fs::OpenOptions::new()
        .mode(0o600)
        .create_new(true)
        .write(true)
        .open(log)?;
    let stderr = stdout.try_clone()?;
    let mut command = tokio::process::Command::new("rsync");
    command.args([
        if basic { "-aS" } else { "-aHAXS" },
        "--numeric-ids",
        "--super",
        "--ignore-times",
        "--itemize-changes",
        "--stats",
    ]);
    if root_only {
        command.arg("--exclude=/*");
    }
    if let Some(files) = files {
        command
            .args(["--no-recursive", "--dirs", "--from0"])
            .arg("--files-from")
            .arg(files);
    }
    command
        .arg("--")
        .arg(format!("{}/", source.display()))
        .arg(format!("{}/", target.display()))
        .stdout(stdout)
        .stderr(stderr);
    let (mut child, group) = super::files::ProcessGroup::spawn(&mut command)?;
    let status = child.wait().await?;
    drop(group);
    if !status.success() {
        return Err(invalid(format!(
            "rsync {status}; diagnostics {}",
            log.display()
        )));
    }
    Ok(())
}
async fn list_file(path: &Path, paths: &BTreeSet<PathBuf>) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    for p in paths {
        file.write_all(if p.as_os_str().is_empty() {
            b"."
        } else {
            p.as_os_str().as_bytes()
        })
        .await?;
        file.write_all(&[0]).await?;
    }
    file.flush().await?;
    Ok(())
}
/// Unmount and discard the owned mutable workspace; an absent workspace succeeds.
/// Writers must be stopped. Per-run records belong outside this directory.
/// Unexpected mounts, symlinks and I/O errors fail.
pub async fn discard(directory: &Path) -> Result<()> {
    let metadata = match tokio::fs::symlink_metadata(directory).await {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if !metadata.is_dir() || tokio::fs::canonicalize(directory).await? != directory {
        return Err(invalid("overlay cleanup directory must be canonical"));
    }
    let mut trees = Vec::new();
    let mut entries = tokio::fs::read_dir(directory).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            if entry
                .file_name()
                .to_str()
                .is_none_or(|name| name.is_empty() || !name.bytes().all(|c| c.is_ascii_digit()))
            {
                return Err(invalid("unexpected directory in overlay cleanup"));
            }
            trees.push(entry.path());
        } else if entry.file_type().await?.is_symlink() {
            return Err(invalid("linked overlay cleanup metadata"));
        }
    }
    // Inspect every mount before changing any filesystem state.
    let mounted = mounts().await?;
    let views: Vec<_> = mounted
        .iter()
        .filter(|path| {
            trees.iter().any(|tree| {
                let merged = tree.join("merged");
                **path == merged || path.parent() == Some(merged.as_path())
            })
        })
        .cloned()
        .collect();
    for mount in mounted.iter().filter(|mount| mount.starts_with(directory)) {
        if !views.contains(mount) {
            return Err(invalid("unexpected mount in overlay cleanup"));
        }
    }
    for view in views.iter().rev().filter(|view| mounted.contains(view)) {
        unmount(view).await?;
    }
    if mounts()
        .await?
        .iter()
        .any(|mount| mount.starts_with(directory))
    {
        return Err(invalid("overlay cleanup left a mounted view"));
    }
    let disposable: Vec<_> = trees
        .iter()
        .flat_map(|tree| ["upper", "work", "merged"].map(|name| tree.join(name)))
        .collect();
    for path in &disposable {
        match tokio::fs::symlink_metadata(path).await {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                tokio::fs::remove_dir_all(path).await?
            }
            Ok(_) => return Err(invalid("invalid overlay cleanup tree")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    // Absence is the success condition, including retries after partial deletion.
    for path in &disposable {
        match tokio::fs::symlink_metadata(path).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
            Ok(_) => return Err(invalid("overlay data remains after cleanup")),
        }
    }
    if mounts()
        .await?
        .iter()
        .any(|mount| mount.starts_with(directory))
    {
        return Err(invalid("mounted view remains after overlay cleanup"));
    }
    tokio::fs::remove_dir_all(directory).await?;
    if tokio::fs::try_exists(directory).await? {
        return Err(invalid("overlay workspace remains after reset"));
    }
    Ok(())
}

/// Finalize stopped views after capture: write back, keep, or discard changes.
/// Write-back replaces conflicting host changes; errors preserve upper data until reset.
/// Application is non-atomic and is never replayed by pre-run cleanup.
pub async fn finalize(set: OverlaySet) -> Result<OverlayResults> {
    if set.policy.disposition == Disposition::Discard {
        discard(&set.workspace).await?;
    } else {
        let mounted = mounts().await?;
        for tree in set
            .trees
            .iter()
            .rev()
            .filter(|tree| mounted.contains(&tree.merged))
        {
            unmount(&tree.merged).await?;
        }
        if mounts()
            .await?
            .iter()
            .any(|path| path.starts_with(&set.workspace))
        {
            return Err(invalid("overlay remains mounted after unmount"));
        }
        private_write(&set.directory.join("unmounted.json"), b"true\n").await?;
    }
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    if set.policy.disposition != Disposition::Discard {
        // Validate every source before mutating any. Upper path bytes are stored
        // as arrays so filesystem names need not be UTF-8.
        let mut plans = Vec::new();
        for tree in &set.trees {
            let tree = tree.clone();
            let plan = tokio::task::spawn_blocking(move || direct::inspect(&tree, |_| Ok(())))
                .await
                .map_err(|e| invalid(e.to_string()))??;
            plans.push(plan);
        }
        for (tree, plan) in set.trees.iter().zip(&plans) {
            let base = &tree.records;
            let actions = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(base.join("apply-plan.pb"))
                .await?;
            let mut actions = tokio::io::BufWriter::new(actions);
            for path in &plan.remove {
                write_record(
                    &mut actions,
                    &crate::filesystem_proto::FileAction {
                        kind: 1,
                        path: path.clone(),
                    },
                )
                .await?;
            }
            for path in &plan.transfer {
                write_record(
                    &mut actions,
                    &crate::filesystem_proto::FileAction {
                        kind: 2,
                        path: path.clone(),
                    },
                )
                .await?;
            }
            actions.flush().await?;
            let selected: BTreeSet<PathBuf> = plan
                .transfer
                .iter()
                .map(|p| PathBuf::from(std::ffi::OsString::from_vec(p.clone())))
                .collect();
            list_file(&base.join("changed.paths0"), &selected).await?;
        }
        let warning = "Filesystem copies preserve basic rsync archive attributes; ACL, xattr, capability and hardlink fidelity are not qualified";
        eprintln!("warning: {warning}");
        warnings.push(warning.to_string());
        private_write(&set.directory.join("metadata-coverage.txt"), b"Basic rsync archive attributes; ACLs, xattrs, capabilities and hardlinks are not qualified.\n").await?;
        let keep = set.policy.disposition == Disposition::Keep;
        let kept = set
            .directory
            .parent()
            .ok_or_else(|| invalid("overlay records have no run folder"))?
            .join(KEPT_DIRECTORY);
        if keep {
            super::files::create_private_dir(&kept).await?;
        }
        for (index, (tree, plan)) in set.trees.iter().zip(plans).enumerate() {
            warnings.extend(plan.warnings.iter().cloned());
            let target = if keep {
                let target = kept.join(index.to_string());
                super::files::create_private_dir(&target).await?;
                target
            } else {
                tree.source.clone()
            };
            let base = &tree.records;
            let mut journal = tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(base.join("apply.pb"))
                .await?;
            for bytes in plan.remove.iter().filter(|_| !keep) {
                let relative = PathBuf::from(std::ffi::OsString::from_vec(bytes.clone()));
                let path = tree.source.join(relative);
                match tokio::fs::symlink_metadata(&path).await {
                    Ok(meta) if meta.is_dir() => tokio::fs::remove_dir_all(&path).await?,
                    Ok(_) => tokio::fs::remove_file(&path).await?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(e.into()),
                }
                write_record(
                    &mut journal,
                    &crate::filesystem_proto::FileAction {
                        kind: 1,
                        path: bytes.clone(),
                    },
                )
                .await?;
            }
            journal.flush().await?;
            rsync(
                &tree.upper,
                &target,
                Some(&base.join("changed.paths0")),
                &base.join("apply.log"),
                false,
                true,
            )
            .await?;
            // Copy root attributes separately so selecting it cannot include whiteouts or special children.
            rsync(
                &tree.upper,
                &target,
                Some(&base.join("root.paths0")),
                &base.join("apply-root.log"),
                true,
                true,
            )
            .await?;
            if keep {
                let target = target.clone();
                tokio::task::spawn_blocking(move || sync_kept(&target, &plan.transfer))
                    .await
                    .map_err(|e| invalid(e.to_string()))??;
            }
            paths.push(target);
        }
        if keep {
            tokio::fs::File::open(&kept).await?.sync_all().await?;
            tokio::fs::File::open(kept.parent().unwrap())
                .await?
                .sync_all()
                .await?;
            discard(&set.workspace).await?;
        }
    }
    let result = OverlayResults {
        disposition: set.policy.disposition,
        paths,
        unmounted: true,
        warnings,
    };
    private_write(
        &set.directory.join("results.json"),
        &serde_json::to_vec_pretty(&result)?,
    )
    .await?;
    Ok(result)
}

// Sync rsync-completed copies before releasing any upper.
fn sync_kept(target: &Path, paths: &BTreeSet<Vec<u8>>) -> Result<()> {
    for bytes in paths {
        let path = target.join(std::ffi::OsStr::from_bytes(bytes));
        let actual = std::fs::symlink_metadata(&path)?;
        if !actual.file_type().is_symlink() {
            std::fs::File::open(&path)?.sync_all()?;
        }
    }
    std::fs::File::open(target)?.sync_all()?;
    Ok(())
}

/// Classify an existing canonical source without traversing folders or reading payloads.
/// Host shared memory retains native passthrough semantics at every mount path.
pub async fn source_kind(
    source: &Path,
    artifacts: &Path,
    passthrough_folders: &[String],
) -> Result<(SourceKind, Option<String>)> {
    classify_source(source, artifacts, &mounts().await?, passthrough_folders).await
}
async fn classify_source(
    source: &Path,
    artifacts: &Path,
    mountpoints: &[PathBuf],
    passthrough_folders: &[String],
) -> Result<(SourceKind, Option<String>)> {
    for folder in passthrough_folders {
        let folder = Path::new(folder);
        if !folder.is_absolute() {
            return Err(invalid(
                "passthrough_folders must contain absolute host folders",
            ));
        }
        let folder = match tokio::fs::canonicalize(folder).await {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => folder.to_path_buf(),
            Err(error) => return Err(error.into()),
        };
        if source.starts_with(&folder) || folder.starts_with(source) {
            return Ok((
                SourceKind::Passthrough,
                Some("not tracked: passed through by passthrough_folders".into()),
            ));
        }
    }

    if crate::is_host_shm_source(source)? {
        return Ok((
            SourceKind::HostSharedMemory,
            Some("live host shared memory".into()),
        ));
    }
    let meta = tokio::fs::symlink_metadata(source).await?;
    let reason = if !meta.is_dir() {
        "not tracked: single file or special mount source".into()
    } else if ["/proc", "/sys", "/dev"]
        .iter()
        .any(|root| source.starts_with(root))
    {
        "not tracked: pseudo-filesystem or device mount".into()
    } else if source.starts_with(artifacts) {
        return Ok((
            SourceKind::RunOutput,
            Some("run output folder; files listed as added".into()),
        ));
    } else if artifacts.starts_with(source) {
        "not tracked: source contains the run folder".into()
    } else if let Some(nested) = mountpoints
        .iter()
        .find(|point| *point != source && point.starts_with(source))
    {
        format!("not tracked: contains mount point {}", nested.display())
    } else {
        return Ok((SourceKind::Overlay, None));
    };
    Ok((SourceKind::Untracked, Some(reason)))
}

/// Read upper metadata transitions after writers stop; no mount or payload reads are required.
/// Results allocate with the upper and removed lower subtrees. Collection errors retain the upper.
pub async fn changes(tree: &OverlayTree) -> Result<Vec<crate::filesystem_proto::FileChange>> {
    let tree = tree.clone();
    tokio::task::spawn_blocking(move || {
        let mut changes = Vec::new();
        direct::inspect(&tree, |row| {
            changes.push(row);
            Ok(())
        })?;
        Ok(changes)
    })
    .await
    .map_err(|error| invalid(error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn prepare(input: PrepareInput) -> Result<OverlaySet> {
        super::prepare(resolve(input).await?).await
    }

    #[tokio::test]
    async fn unsafe_direct_aliases_fail_before_creating_views() {
        // Arrange: the parent contains the run folder, or a writable file aliases a folder view.
        for shape in ["writable-parent", "readonly-parent", "writable-file"] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("source");
            let child = source.join("child");
            std::fs::create_dir_all(&child).unwrap();
            let file = child.join("file");
            std::fs::write(&file, b"original").unwrap();
            let artifacts = if shape == "writable-file" {
                root.path().join("run")
            } else {
                source.join("run")
            };
            std::fs::create_dir(&artifacts).unwrap();
            let workspace = root.path().join("views");
            let bind = |path: &Path, target: &str, read_only| DirectoryMount {
                source: path.to_string_lossy().into(),
                consumer: "app".into(),
                target: target.into(),
                read_only,
            };
            let bind_mounts = if shape == "writable-file" {
                vec![bind(&child, "/folder", false), bind(&file, "/file", false)]
            } else {
                vec![
                    bind(&source, "/parent", shape == "readonly-parent"),
                    bind(&child, "/child", false),
                ]
            };
            // Act: Review and launch reject the same unsupported alias shape.
            let review_error =
                crate::changes::review_sources(&bind_mounts, &[], &artifacts, true, &[])
                    .await
                    .unwrap_err();
            assert!(
                review_error
                    .to_string()
                    .contains("cannot share isolated view"),
                "{shape}: {review_error}"
            );
            let error = resolve(PrepareInput {
                artifacts: artifacts.clone(),
                workspace: workspace.clone(),
                bind_mounts,
                policy: Default::default(),
            })
            .await
            .err()
            .unwrap();
            // Assert: both source data and mount ownership are untouched.
            assert!(
                error.to_string().contains("cannot share isolated view"),
                "{shape}: {error}"
            );
            assert!(!workspace.exists());
            assert!(!artifacts.join(RECORDS_DIRECTORY).exists());
            assert_eq!(std::fs::read(file).unwrap(), b"original");
        }
    }

    #[tokio::test]
    async fn host_shm_binds_bypass_overlay_copy_and_writeback() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let artifacts = root.path().join("artifacts");
        std::fs::create_dir(&artifacts).unwrap();
        let shared = tempfile::tempdir_in("/dev/shm").unwrap();
        let file = shared.path().join("segment");
        std::fs::write(&file, b"initial").unwrap();
        let bind_mounts = vec![
            DirectoryMount {
                consumer: "project/app".into(),
                source: shared.path().to_string_lossy().into(),
                target: "/input/shm".into(),
                ..Default::default()
            },
            DirectoryMount {
                consumer: "project/app".into(),
                source: file.to_string_lossy().into(),
                target: "/input/camera".into(),
                ..Default::default()
            },
            DirectoryMount {
                consumer: "project/app".into(),
                source: file.to_string_lossy().into(),
                target: "/read-only".into(),
                read_only: true,
            },
        ];

        // Act
        let set = prepare(PrepareInput {
            artifacts,
            workspace: root.path().join("workspace"),
            bind_mounts,
            policy: OverlayPolicy {
                passthrough_folders: Vec::new(),
                disposition: Disposition::Discard,
            },
        })
        .await
        .unwrap();
        std::fs::write(&file, b"live").unwrap();

        // Assert
        assert!(set.trees.is_empty());
        assert_eq!(set.mappings.len(), 3);
        assert!(
            set.mappings
                .iter()
                .all(|mapping| mapping.original == mapping.effective)
        );

        // Act: discard the prepared run resources.
        finalize(set).await.unwrap();
        // Assert
        assert_eq!(std::fs::read(&file).unwrap(), b"live");
    }
}
