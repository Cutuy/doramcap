//! Per-run Linux OverlayFS views for Compose directory binds.
//!
//! Requires a cooperative local daemon in the caller's mount namespace and
//! CAP_SYS_ADMIN. Caller must stop all writers before finalization. Explicit
//! preparation/finalization own mounts; dropping serialized values does no cleanup.
mod direct;

use super::{
    ResolvedComposeProject, Result,
    adapter::{digest, private_write},
    error::invalid,
    filesystem::{MetadataWalk, Snapshot, read_snapshot, write_record, write_snapshot},
};
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

/// Require the local Docker socket peer to share this process's mount namespace.
/// Overlay binds must resolve there; proxies require separate namespace guarantees.
pub(crate) async fn verify_namespace(socket: &Path) -> Result<()> {
    use std::os::fd::AsRawFd;
    let stream = tokio::net::UnixStream::connect(socket).await?;
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: destination and length point to correctly sized live storage.
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let ours = tokio::fs::read_link("/proc/self/ns/mnt").await?;
    let theirs = tokio::fs::read_link(format!("/proc/{}/ns/mnt", cred.pid)).await?;
    if ours != theirs {
        return Err(invalid(
            "Docker peer and manager mount namespaces differ; host overlay visibility is unqualified",
        ));
    }
    Ok(())
}

/// Disposition after all writers stop. Default reproduces persistent host output.
/// Retaining results uses `WriteBack` to the configured host source.
/// Prefer a per-experiment source directory for isolated retained results.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    #[default]
    WriteBack,
    Discard,
}
/// Per-run filesystem policy. Directory binds are isolated by default.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayPolicy {
    #[serde(default)]
    /// Apply upper changes to their source, or discard them.
    pub disposition: Disposition,
}
/// One shared tree and its private run-owned mount/staging locations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayTree {
    /// Original directory; immutable while mounted.
    pub source: PathBuf,
    /// Fresh copy-up layer owned by this run.
    pub upper: PathBuf,
    /// Fresh kernel scratch directory on the upper filesystem.
    pub work: PathBuf,
    /// Mountpoint bound into containers.
    pub merged: PathBuf,
    /// Original metadata inventory used for conflict detection.
    pub baseline: PathBuf,
}
/// Explicit original/effective bind mapping, including read-only aliases.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountMapping {
    /// Compose project key.
    pub project: String,
    /// Service consuming this bind.
    pub service: String,
    /// Unchanged container destination.
    pub target: String,
    /// Canonical user-specified source.
    pub original: PathBuf,
    /// Source used by the generated execution manifest.
    pub effective: PathBuf,
    /// Original access flag retained at the container bind.
    pub read_only: bool,
    /// Reason for redirecting or preserving this source.
    pub reason: String,
}
/// Prepared views; the caller finalizes them or discards them on the next reset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlaySet {
    /// Private artifact directory for ownership and result records.
    pub directory: PathBuf,
    /// Fixed mutable workspace; reset discards its contents after stopping writers.
    pub workspace: PathBuf,
    /// Non-overlapping shared directory views.
    pub trees: Vec<OverlayTree>,
    /// Pinned Compose manifests, rewritten in the preparation result.
    pub projects: Vec<ResolvedComposeProject>,
    /// Original and effective sources for every bind consumer.
    pub mappings: Vec<MountMapping>,
    /// Filesystem disposition selected for this run.
    pub policy: OverlayPolicy,
}
/// All projects in one serial run; inputs must already be resolved and pinned.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareInput {
    /// Existing fresh run directory, outside all lower trees.
    pub artifacts: PathBuf,
    /// Fresh mutable workspace with an existing canonical parent, outside all source trees.
    pub workspace: PathBuf,
    /// Pinned Compose manifests, rewritten in the preparation result.
    pub projects: Vec<ResolvedComposeProject>,
    /// Filesystem disposition selected for this run.
    pub policy: OverlayPolicy,
}
/// Final result locations; discard returns no source paths.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayResults {
    /// Apply upper changes to their source, or discard them.
    pub disposition: Disposition,
    /// Original paths after write-back; empty after discard.
    pub paths: Vec<PathBuf>,
    /// True only after every owned mount has been strictly unmounted.
    pub unmounted: bool,
    /// Coverage limitations for this disposition.
    #[serde(default)]
    pub warnings: Vec<String>,
}
/// Prepare all directory views and immutable execution manifests before launch.
/// Errors unwind mounts already created; failed rollback is included in the error.
/// No upstream Compose file is edited. Async operations are sequential.
pub struct PrepareOverlays {
    /// Local daemon used to validate all rewritten manifests before launch.
    pub adapter: std::rc::Rc<super::DockerAdapter>,
}
impl Step for PrepareOverlays {
    const ID: Option<&'static str> = Some("compose.filesystem.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare filesystem");
    type Input = PrepareInput;
    type Output = OverlaySet;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        let set = prepare(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))?;
        for project in &set.projects {
            if let Err(error) = self.adapter.project(project).validate_execution().await {
                let mut errors = vec![error.to_string()];
                for tree in set.trees.iter().rev() {
                    if let Err(e) = unmount(&tree.merged).await {
                        errors.push(format!("rollback {}: {e}", tree.merged.display()));
                    }
                }
                return Err(orchestrator::Error::message(errors.join("; ")));
            }
        }
        Ok(set)
    }
}
/// Strictly unmount, then apply upper changes or discard them; writers must stop first.
/// Interrupted write-back leaves upper data for the user to salvage before reset.
/// Pre-run reset never replays writes.
pub struct FinalizeOverlays;
impl Step for FinalizeOverlays {
    type Input = OverlaySet;
    type Output = OverlayResults;
    async fn run(&mut self, input: Self::Input) -> orchestrator::Result<Self::Output> {
        finalize(input)
            .await
            .map_err(|e| orchestrator::Error::message(e.to_string()))
    }
}
fn cstr(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("NUL in mount path"))
}
async fn mount(tree: &OverlayTree) -> Result<()> {
    let tree = tree.clone();
    tokio::task::spawn_blocking(move || {
        for p in [&tree.source, &tree.upper, &tree.work] {
            if p.to_str().is_none() {
                return Err(invalid("non-UTF8 overlay source path"));
            }
            if p.as_os_str()
                .as_bytes()
                .iter()
                .any(|b| b",:\\\n".contains(b))
            {
                return Err(invalid(format!(
                    "unsupported OverlayFS option characters: {}",
                    p.display()
                )));
            }
        }
        let target = cstr(&tree.merged)?;
        let options = CString::new(format!(
            "lowerdir={},upperdir={},workdir={},index=on,metacopy=off,redirect_dir=off,xino=on",
            tree.source.display(),
            tree.upper.display(),
            tree.work.display()
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
async fn unmount(path: &Path) -> Result<()> {
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
async fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, super::filesystem::FileMetadata>> {
    let mut walk = MetadataWalk::open(root).await?;
    let mut entries = BTreeMap::new();
    while let Some((p, m)) = walk.next_entry().await? {
        entries.insert(p, m);
    }
    Ok(entries)
}
fn set_readonly_ancestors(mappings: &[MountMapping], writable: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    mappings
        .iter()
        .filter(|m| m.read_only && writable.iter().any(|p| p.starts_with(&m.original)))
        .map(|m| m.original.clone())
        .collect()
}

/// Plan and mount views. Sources must exist and be ordinary directories. Read-only
/// binds outside shared writable trees and dedicated run artifacts stay unchanged.
/// Writable files, special nodes and nested mounts fail closed with their paths.
pub async fn prepare(input: PrepareInput) -> Result<OverlaySet> {
    let artifacts = tokio::fs::canonicalize(&input.artifacts).await?;
    let directory = artifacts.join("overlays");
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
    let mut models = Vec::new();
    let mut writable = BTreeSet::new();
    let mut mappings = Vec::new();
    for project in &input.projects {
        let bytes = tokio::fs::read(&project.manifest).await?;
        if digest(&bytes) != project.sha256 {
            return Err(invalid("overlay input manifest changed"));
        }
        let model: Value = serde_json::from_slice(&bytes)?;
        for (service, v) in model["services"]
            .as_object()
            .ok_or_else(|| invalid("missing services"))?
        {
            for bind in v["volumes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|v| v["type"] == "bind")
            {
                let source = tokio::fs::canonicalize(
                    bind["source"]
                        .as_str()
                        .ok_or_else(|| invalid("bind source missing"))?,
                )
                .await?;
                if source.starts_with(&workspace) || workspace.starts_with(&source) {
                    return Err(invalid("overlay source overlaps the mutable workspace"));
                }
                let ro = bind["read_only"] == true;
                let meta = tokio::fs::symlink_metadata(&source).await?;
                let owned = source.starts_with(&artifacts);
                if !ro && !owned {
                    if !meta.is_dir()
                        || ["/proc", "/sys", "/dev"]
                            .iter()
                            .any(|p| source.starts_with(p))
                    {
                        return Err(invalid(format!(
                            "writable bind requires explicit unsupported-source handling: {}",
                            source.display()
                        )));
                    }
                    if artifacts.starts_with(&source) {
                        return Err(invalid(format!(
                            "artifact directory overlaps lower tree: {}",
                            source.display()
                        )));
                    }
                    if bind["bind"]["propagation"]
                        .as_str()
                        .is_some_and(|p| p != "rprivate" && p != "private")
                        || bind["bind"].get("selinux").is_some()
                    {
                        return Err(invalid(format!(
                            "unqualified bind propagation/SELinux option: {}",
                            source.display()
                        )));
                    }
                    writable.insert(source.clone());
                }
                mappings.push(MountMapping {
                    project: project.key.clone(),
                    service: service.clone(),
                    target: bind["target"].as_str().unwrap_or("").into(),
                    original: source.clone(),
                    effective: source,
                    read_only: ro,
                    reason: if owned {
                        "fresh run artifact"
                    } else if ro {
                        "read-only input"
                    } else {
                        "OverlayFS"
                    }
                    .into(),
                });
            }
        }
        models.push(model);
    }
    // A read-only ancestor must see mutations through a writable child alias.
    // Include it in the shared tree while retaining read_only on its bind target.
    let aliases: Vec<_> = set_readonly_ancestors(&mappings, &writable);
    for alias in aliases {
        if artifacts.starts_with(&alias)
            || ["/proc", "/sys", "/dev"]
                .iter()
                .any(|p| alias.starts_with(p))
        {
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
    let mountpoints = mounts().await?;
    for root in &roots {
        let mut links: BTreeMap<(u64, u64), (u64, u64)> = BTreeMap::new();
        let mut walk = MetadataWalk::open(root).await?;
        while let Some((p, _)) = walk.next_entry().await? {
            let m = tokio::fs::symlink_metadata(root.join(p)).await?;
            if m.is_file() && m.nlink() > 1 {
                let group = links.entry((m.dev(), m.ino())).or_insert((0, m.nlink()));
                group.0 += 1;
            }
        }
        if links.values().any(|(seen, total)| seen != total) {
            return Err(invalid(format!(
                "hardlink group escapes overlay source: {}",
                root.display()
            )));
        }
    }
    for root in &roots {
        if let Some(nested) = mountpoints
            .iter()
            .find(|p| *p != root && p.starts_with(root))
        {
            return Err(invalid(format!(
                "nested mount {} inside overlay source {}",
                nested.display(),
                root.display()
            )));
        }
    }
    crate::evidence::create_private_dir(&directory).await?;
    crate::evidence::create_private_dir(&workspace).await?;
    let mut set = OverlaySet {
        directory: directory.clone(),
        workspace: workspace.clone(),
        trees: vec![],
        projects: input.projects,
        mappings,
        policy: input.policy,
    };
    for (i, source) in roots.into_iter().enumerate() {
        let base = directory.join(i.to_string());
        tokio::fs::create_dir(&base).await?;
        let view = workspace.join(i.to_string());
        tokio::fs::create_dir(&view).await?;
        let tree = OverlayTree {
            source,
            upper: view.join("upper"),
            work: view.join("work"),
            merged: view.join("merged"),
            baseline: base.join("baseline.pb"),
        };
        for path in [&tree.upper, &tree.work, &tree.merged] {
            tokio::fs::create_dir(path).await?;
        }
        // Capture baselines before any mounts so failure here owns no kernel resources.
        write_snapshot(
            &tree.baseline,
            &Snapshot {
                entries: snapshot(&tree.source).await?,
                complete: true,
                reason: None,
            },
        )
        .await?;
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
        for (project, model) in set.projects.iter_mut().zip(&mut models) {
            for (service, v) in model["services"].as_object_mut().unwrap() {
                for bind in v
                    .get_mut("volumes")
                    .and_then(Value::as_array_mut)
                    .into_iter()
                    .flatten()
                    .filter(|v| v["type"] == "bind")
                {
                    let mapping = set
                        .mappings
                        .iter()
                        .find(|m| {
                            m.project == project.key
                                && m.service == *service
                                && m.target == bind["target"].as_str().unwrap_or("")
                        })
                        .unwrap();
                    if mapping.reason == "shared OverlayFS view"
                        && (bind["bind"]["propagation"]
                            .as_str()
                            .is_some_and(|p| p != "rprivate" && p != "private")
                            || bind["bind"].get("selinux").is_some())
                    {
                        return Err(invalid(format!(
                            "unqualified bind propagation/SELinux option: {}",
                            mapping.original.display()
                        )));
                    }
                    bind["source"] = serde_json::json!(mapping.effective);
                }
            }
            let bytes = serde_json::to_vec_pretty(model)?;
            project.manifest = artifacts.join(format!("{}.execution.json", project.key));
            project.sha256 = digest(&bytes);
            private_write(&project.manifest, &bytes).await?;
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
        .stderr(stderr)
        .kill_on_drop(true);
    command.as_std_mut().process_group(0);
    let mut child = command.spawn()?;
    let group =
        super::command::ProcessGroup(child.id().ok_or_else(|| invalid("rsync PID missing"))?);
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
async fn check_lower(tree: &OverlayTree) -> Result<()> {
    let expected = read_snapshot(&tree.baseline).await?;
    if !expected.complete || snapshot(&tree.source).await? != expected.entries {
        return Err(invalid(format!(
            "lower tree changed outside isolated view: {}",
            tree.source.display()
        )));
    }
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
    let views: Vec<_> = trees.iter().map(|tree| tree.join("merged")).collect();
    let mounted = mounts().await?;
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

/// Finalize stopped views: apply supported upper changes, or discard them.
/// Write-back validates stable lower trees; errors preserve upper data until reset.
/// Application is non-atomic and is never replayed by pre-run cleanup.
pub async fn finalize(set: OverlaySet) -> Result<OverlayResults> {
    if set.policy.disposition == Disposition::Discard {
        discard(&set.workspace).await?;
    } else {
        for tree in &set.trees {
            check_lower(tree).await?;
        }
        for tree in set.trees.iter().rev() {
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
    if set.policy.disposition == Disposition::WriteBack {
        // Validate every source before mutating any. Upper path bytes are stored
        // as arrays so filesystem names need not be UTF-8.
        let mut plans = Vec::new();
        for tree in &set.trees {
            check_lower(tree).await?;
            let tree = tree.clone();
            let plan = tokio::task::spawn_blocking(move || direct::inspect(&tree))
                .await
                .map_err(|e| invalid(e.to_string()))??;
            plans.push(plan);
        }
        for (tree, plan) in set.trees.iter().zip(&plans) {
            let base = tree.baseline.parent().unwrap();
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
        let warning = "Host write-back preserves basic rsync archive attributes; ACL, xattr, capability and hardlink fidelity are not qualified";
        eprintln!("warning: {warning}");
        warnings.push(warning.to_string());
        private_write(&set.directory.join("metadata-coverage.txt"), b"Basic rsync archive attributes; ACLs, xattrs, capabilities and hardlinks are not qualified.\n").await?;
        for (tree, plan) in set.trees.iter().zip(plans) {
            check_lower(tree).await?;
            let base = tree.baseline.parent().unwrap();
            let mut journal = tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(base.join("apply.pb"))
                .await?;
            for bytes in plan.remove {
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
                        path: bytes,
                    },
                )
                .await?;
            }
            journal.flush().await?;
            rsync(
                &tree.upper,
                &tree.source,
                Some(&base.join("changed.paths0")),
                &base.join("apply.log"),
                false,
                true,
            )
            .await?;
            paths.push(tree.source.clone());
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
