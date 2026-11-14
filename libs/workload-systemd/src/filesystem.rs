//! Native mount resolution, working-directory checks and overlay source replacement.
use crate::{
    Bind, ConfigurationSnapshot, Result, RunPlan, Scope, SystemdPlan, error,
    manager::{MS_REC, Manager, quote_path},
};
use host_filesystem::{
    changes::{self, FilesystemSources, SourcesInput},
    filesystem_proto::DirectoryMount,
    overlay::{self, OverlayPolicy, OverlaySet},
};
use std::{
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

const OVERLAY_DROP_IN: &str = "15-doramcap-overlays.conf";
const WORKSPACE: &str = "overlays";
type NativeBind = (String, String, bool, u64);

struct NativeUnit {
    name: String,
    writable: Vec<NativeBind>,
    read_only: Vec<NativeBind>,
}

pub(crate) struct Resolved {
    mounts: Vec<DirectoryMount>,
    native: Vec<NativeUnit>,
    pub notes: Vec<String>,
}

/// Live views and pinned source identities; owned by the running handle until finalization.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PreparedFilesystem {
    pub overlays: OverlaySet,
    pub sources: FilesystemSources,
}

pub(crate) fn workspace(scope: &Scope) -> PathBuf {
    scope.directory.join(WORKSPACE)
}

/// Check mounts and working directories using captured unit bytes interpreted by systemd.
/// Review starts no units or mounts; user app inputs need runtime linking privileges. Await cleanup.
pub async fn review(
    run: &RunPlan,
    plan: &SystemdPlan,
) -> Result<(ConfigurationSnapshot, Vec<String>)> {
    let snapshot = crate::inspect(run, plan).await?;
    if plan.unit_files.is_empty() {
        let resolved = resolve(
            None,
            &Scope::of(Path::new(&run.repository_root))?,
            run,
            plan,
            true,
        )
        .await?;
        return Ok((snapshot, review_notes(resolved, run).await?));
    }
    let _admission =
        workload_lifecycle::admission::Admission::acquire(Path::new(&run.repository_root))
            .map_err(error)?;
    let manager = Manager::connect().await?;
    let scope = Scope::of(Path::new(&run.repository_root))?;
    let remaining = manager.remaining(&scope).await?;
    if !remaining.is_empty() {
        return Err(format!(
            "prior resources remain: {remaining:?}; clean up before Review"
        ));
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&scope.directory)
        .map_err(error)?;
    let result = async {
        let capture_plan = plan.clone();
        let directory = scope.directory.clone();
        let captured = tokio::task::spawn_blocking(move || {
            crate::preparation::review_units(&capture_plan, &directory)
        })
        .await
        .map_err(error)??;
        crate::runtime::link_inputs(&manager, &scope, &captured).await?;
        resolve(Some(&manager), &scope, run, &captured, true).await
    }
    .await;
    // Admission protects these exact launch names; Review never starts them.
    let cleanup = async {
        for path in crate::manager::runtime_files(&scope).await? {
            crate::manager::remove_path(&path).await?;
        }
        crate::manager::remove_path(&scope.directory).await?;
        manager.manager.reload().await.map_err(error)
    }
    .await;
    match (result, cleanup) {
        (Ok(resolved), Ok(())) => Ok((snapshot, review_notes(resolved, run).await?)),
        (Err(cause), Ok(())) | (Ok(_), Err(cause)) => Err(cause),
        (Err(cause), Err(cleanup)) => Err(format!("{cause}; Review cleanup: {cleanup}")),
    }
}

/// Resolve every supported bind before the first mount; Review permits fresh output directories.
pub(crate) async fn resolve(
    manager: Option<&Manager>,
    scope: &Scope,
    run: &RunPlan,
    plan: &SystemdPlan,
    reviewing: bool,
) -> Result<Resolved> {
    let mut resolved = Resolved {
        mounts: Vec::new(),
        native: Vec::new(),
        notes: Vec::new(),
    };
    for program in &plan.programs {
        check_directory(
            &program.id,
            &program.working_directory,
            &program.binds,
            None,
            true,
            run,
            reviewing,
        )?;
        add_binds(&mut resolved.mounts, &program.id, &program.binds);
    }
    for file in &plan.unit_files {
        let id = crate::unit_stem(file)?;
        let name = scope.unit(id);
        let unit = manager
            .ok_or("user app unit inspection requires systemd")?
            .watch(&name)
            .await?;
        let writable = unit.service.bind_paths().await.map_err(error)?;
        let read_only = unit.service.bind_read_only_paths().await.map_err(error)?;
        let mut binds = Vec::new();
        for ((source, target, optional, _), read_only) in writable
            .iter()
            .map(|entry| (entry, false))
            .chain(read_only.iter().map(|entry| (entry, true)))
        {
            match std::fs::metadata(source) {
                Err(error) if *optional && error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("{id}: bind {source}:{target}: {error}")),
                Ok(_) => binds.push(Bind {
                    source: source.clone(),
                    target: target.clone(),
                    read_only,
                }),
            }
        }
        for (_, _, _, flags) in writable.iter().chain(&read_only) {
            if *flags != 0 && *flags != MS_REC {
                return Err(format!("{id}: unsupported native bind flags {flags}"));
            }
        }
        let directory = unit.service.working_directory().await.map_err(error)?;
        let root = unit.service.root_directory().await.map_err(error)?;
        if !unit.service.root_image().await.map_err(error)?.is_empty()
            && !directory.is_empty()
            && !missing_directory_ok(&directory)
        {
            resolved.notes.push(format!(
                "{id}: working directory {directory} is inside RootImage and was not checked"
            ));
        } else {
            check_directory(
                id,
                &directory,
                &binds,
                (!root.is_empty()).then(|| Path::new(&root)),
                false,
                run,
                reviewing,
            )?;
        }
        if !directory.is_empty()
            && !missing_directory_ok(&directory)
            && !binds
                .iter()
                .any(|bind| !bind.read_only && Path::new(&directory).starts_with(&bind.target))
        {
            resolved.notes.push(format!("{id}: working directory {directory}: not tracked outside writable mounts; declare a BindPaths= entry"));
        }
        resolved.notes.push(format!(
            "{id}: writes outside declared writable mounts are not tracked"
        ));
        add_binds(&mut resolved.mounts, id, &binds);
        resolved.native.push(NativeUnit {
            name,
            writable,
            read_only,
        });
    }
    // Review accepts only the exact output directories preparation creates.
    let mut existing = Vec::new();
    for bind in &resolved.mounts {
        match std::fs::metadata(&bind.source) {
            Ok(_) => existing.push(bind.clone()),
            Err(error)
                if reviewing
                    && error.kind() == std::io::ErrorKind::NotFound
                    && (Path::new(&bind.source) == Path::new(&run.artifacts)
                        || Path::new(&bind.source)
                            == Path::new(&run.artifacts).join("dora/runtime")) => {}
            Err(error) => {
                return Err(format!(
                    "{}: bind {}:{}: {error}",
                    bind.consumer, bind.source, bind.target
                ));
            }
        }
    }
    overlay::resolve_sources(&existing, Path::new(&run.artifacts))
        .await
        .map_err(error)?;
    Ok(resolved)
}

async fn review_notes(mut resolved: Resolved, run: &RunPlan) -> Result<Vec<String>> {
    resolved.notes.extend(
        changes::coverage_notes(&resolved.mounts, &[], Path::new(&run.artifacts), true)
            .await
            .map_err(error)?,
    );
    Ok(resolved.notes)
}

fn add_binds(mounts: &mut Vec<DirectoryMount>, id: &str, binds: &[Bind]) {
    mounts.extend(binds.iter().map(|bind| DirectoryMount {
        consumer: id.into(),
        source: bind.source.clone(),
        target: bind.target.clone(),
        read_only: bind.read_only,
    }));
}

// systemd's D-Bus getter prefixes native WorkingDirectory=- paths with `!`.
fn missing_directory_ok(directory: &str) -> bool {
    directory.starts_with(['-', '!'])
}

fn check_directory(
    id: &str,
    directory: &str,
    binds: &[Bind],
    root: Option<&Path>,
    private: bool,
    run: &RunPlan,
    reviewing: bool,
) -> Result<()> {
    if directory.is_empty() || missing_directory_ok(directory) {
        return Ok(());
    }
    let path = Path::new(directory);
    if !path.is_absolute() {
        return Err(format!(
            "{id}: working directory {directory} must resolve to an absolute path"
        ));
    }
    let host = binds
        .iter()
        .filter(|bind| path.starts_with(&bind.target))
        .max_by_key(|bind| bind.target.len())
        .map(|bind| Path::new(&bind.source).join(path.strip_prefix(&bind.target).unwrap()))
        .or_else(|| root.map(|root| root.join(path.strip_prefix("/").unwrap_or(path))))
        .or_else(|| {
            (!private
                || crate::manager::SYSTEM_PATHS
                    .iter()
                    .any(|root| path.starts_with(root)))
            .then(|| path.to_path_buf())
        });
    let valid = host.as_ref().is_some_and(|host| {
        host.is_dir()
            || (reviewing
                && (host == Path::new(&run.artifacts)
                    || host == &Path::new(&run.artifacts).join("dora/runtime")))
    });
    if !valid {
        return Err(format!(
            "{id}: working directory {directory} is missing in the unit filesystem"
        ));
    }
    Ok(())
}

pub(crate) async fn prepare(
    manager: &Manager,
    scope: &Scope,
    run: &RunPlan,
    plan: &SystemdPlan,
    resolved: Resolved,
) -> Result<PreparedFilesystem> {
    let overlay_plan = overlay::resolve(overlay::PrepareInput {
        artifacts: run.artifacts.clone().into(),
        workspace: workspace(scope),
        bind_mounts: resolved.mounts.clone(),
        policy: OverlayPolicy {
            disposition: if plan.discard_writes {
                overlay::Disposition::Discard
            } else {
                overlay::Disposition::WriteBack
            },
        },
    })
    .await
    .map_err(error)?;
    let mut sources = changes::prepare_sources(SourcesInput {
        overlays: true,
        artifacts: run.artifacts.clone().into(),
        bind_mounts: resolved.mounts,
        metadata_sources: vec![],
        excluded: ["inputs", "report.json", "filesystem", "overlays"]
            .into_iter()
            .map(PathBuf::from)
            .chain(run.private_records.iter().map(PathBuf::from))
            .collect(),
    })
    .await
    .map_err(error)?;
    sources.add_coverage_notes(resolved.notes);
    let overlays = overlay::prepare(overlay_plan).await.map_err(error)?;
    for native in resolved.native {
        let path = Path::new(crate::SYSTEMD_RUNTIME_UNITS).join(format!("{}.d", native.name));
        tokio::fs::create_dir_all(&path).await.map_err(error)?;
        tokio::fs::write(
            path.join(OVERLAY_DROP_IN),
            native_drop_in(&native, &overlays)?,
        )
        .await
        .map_err(error)?;
    }
    manager.manager.reload().await.map_err(error)?;
    Ok(PreparedFilesystem { overlays, sources })
}

fn effective(source: &str, target: &str, consumer: &str, overlays: &OverlaySet) -> Result<String> {
    let original = match std::fs::canonicalize(source) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(source.into()),
        Err(e) => return Err(error(e)),
    };
    Ok(overlays
        .mappings
        .iter()
        .find(|mapping| {
            mapping.consumer == consumer && mapping.target == target && mapping.original == original
        })
        .map_or_else(
            || source.into(),
            |mapping| mapping.effective.to_string_lossy().into_owned(),
        ))
}

/// Redirect typed binds without changing destination or access mode.
pub(crate) fn redirect(
    run: &mut RunPlan,
    plan: &mut SystemdPlan,
    overlays: &OverlaySet,
) -> Result<()> {
    if let Some(runtime) = &mut run.dora_runtime {
        let directory = std::fs::canonicalize(&runtime.runtime_directory).map_err(error)?;
        if let Some(tree) = overlays
            .trees
            .iter()
            .find(|tree| directory.starts_with(&tree.source))
        {
            runtime.runtime_directory = tree
                .merged
                .join(directory.strip_prefix(&tree.source).unwrap())
                .to_string_lossy()
                .into_owned();
        }
    }
    for (id, binds) in plan
        .programs
        .iter_mut()
        .map(|program| (&program.id, &mut program.binds))
    {
        for bind in binds {
            bind.source = effective(&bind.source, &bind.target, id, overlays)?;
        }
    }
    Ok(())
}

// Empty BindPaths= clears both native lists, so one writer always restores both.
fn native_drop_in(unit: &NativeUnit, overlays: &OverlaySet) -> Result<String> {
    let id = crate::runner::unit_id(&unit.name);
    let mut lines = "[Service]\nBindPaths=\n".to_owned();
    for (property, binds) in [
        ("BindPaths", &unit.writable),
        ("BindReadOnlyPaths", &unit.read_only),
    ] {
        for (source, target, optional, flags) in binds {
            let source = effective(source, target, id, overlays)?;
            lines.push_str(&format!(
                "{property}={}{source}:{target}:{}\n",
                if *optional { "-" } else { "" },
                if *flags == MS_REC { "rbind" } else { "norbind" },
                source = quote_path(&source),
                target = quote_path(target)
            ));
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_dora_runtime_uses_the_canonical_overlay() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("systemd-runtime-link-{}", std::process::id()));
        let source = root.join("source");
        std::fs::create_dir_all(source.join("runtime")).unwrap();
        std::os::unix::fs::symlink(&source, root.join("link")).unwrap();
        let merged = root.join("merged");
        let mut run = RunPlan {
            dora_runtime: Some(workload_lifecycle::launch::DoraRuntime {
                runtime_directory: root.join("link/runtime").to_string_lossy().into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let overlays = OverlaySet {
            directory: root.join("records"),
            workspace: root.join("views"),
            trees: vec![overlay::OverlayTree {
                source: std::fs::canonicalize(&source).unwrap(),
                merged: merged.clone(),
                upper: root.join("upper"),
                work: root.join("work"),
                records: root.join("records/0"),
            }],
            mappings: vec![],
            policy: Default::default(),
        };
        // Act
        redirect(&mut run, &mut SystemdPlan::default(), &overlays).unwrap();
        // Assert
        assert_eq!(
            Path::new(&run.dora_runtime.unwrap().runtime_directory),
            merged.join("runtime")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
