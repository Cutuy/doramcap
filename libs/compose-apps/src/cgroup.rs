//! Docker-owned accounting parents, expressed using the daemon's configured driver.
use crate::preparation::PrepareInput;
// build.rs generates these protobuf types in Cargo's build output directory.
include!(concat!(env!("OUT_DIR"), "/doramcap.compose_plan.v1.rs"));
use crate::compose::{DockerAdapter, Result, error::invalid};
use futures_util::future::LocalBoxFuture;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
use workload_lifecycle::cgroup::{self, Backend, Removal};

const PARENT_PREFIX: &str = "doramcap-compose";
const PARENT_HASH_CHARS: usize = 12;

/// Resolve a unique per-run parent without creating containers or requiring host D-Bus access.
pub(crate) async fn placement(
    adapter: &DockerAdapter,
    input: &PrepareInput,
) -> Result<CgroupPlacement> {
    let info = tokio::time::timeout(
        Duration::from_secs(input.run_plan.readiness_seconds),
        adapter.engine.client().info(),
    )
    .await
    .map_err(|_| invalid("Docker cgroup capability deadline exceeded"))?
    .map_err(|e| invalid(format!("Docker cgroup capabilities: {e}")))?;
    if info.cgroup_version != Some(bollard::models::SystemInfoCgroupVersionEnum::_2) {
        return Err(invalid(
            "system-app accounting requires Docker cgroup v2 CPU and memory accounting",
        ));
    }
    let repository = std::fs::canonicalize(&input.run_plan.repository_root)?;
    let artifacts = std::fs::canonicalize(&input.run_plan.artifacts)?;
    let digest = Sha256::new()
        .chain_update(repository.as_os_str().as_encoded_bytes())
        .chain_update([0])
        .chain_update(input.run_plan.id.as_bytes())
        .chain_update([0])
        .chain_update(artifacts.as_os_str().as_encoded_bytes())
        .finalize();
    let run = format!(
        "{PARENT_PREFIX}-{}",
        &hex::encode(digest)[..PARENT_HASH_CHARS]
    );
    let parent = match info.cgroup_driver {
        Some(bollard::models::SystemInfoCgroupDriverEnum::SYSTEMD) => {
            format!("{run}-{SYSTEM}.slice")
        }
        Some(bollard::models::SystemInfoCgroupDriverEnum::CGROUPFS) => format!("/{run}/{SYSTEM}"),
        driver => {
            return Err(invalid(format!(
                "unsupported Docker cgroup driver: {driver:?}"
            )));
        }
    };
    let path = parent_path(&parent).map_err(invalid)?;
    Ok(CgroupPlacement {
        parent,
        path: path.to_string_lossy().into_owned(),
    })
}

/// Group below the run group holding one group per service.
#[derive(Clone, Copy)]
pub(crate) enum Role {
    System,
    User,
}
const SYSTEM: &str = "system";
const USER: &str = "user";
impl Role {
    fn name(self) -> &'static str {
        match self {
            Self::System => SYSTEM,
            Self::User => USER,
        }
    }
}

/// Generated run group name, `doramcap-compose-‹hash›`, of a placement's system group.
pub(crate) fn run_name(placement: &CgroupPlacement) -> std::result::Result<&str, String> {
    let run = placement
        .parent
        .strip_suffix(&format!("-{SYSTEM}.slice"))
        .or_else(|| {
            placement
                .parent
                .strip_prefix('/')
                .and_then(|parent| parent.strip_suffix(&format!("/{SYSTEM}")))
        })
        .ok_or("accounting parent is not a generated Compose system group")?;
    generated_run(run).ok_or("accounting parent is not a generated Compose run group")?;
    Ok(run)
}

/// Split a generated service parent into its run group name and the run-independent rest.
pub(crate) fn split_service_parent(parent: &str) -> Option<(&str, &str)> {
    let (slice, start) = match parent.strip_prefix('/') {
        Some(rest) => (false, rest),
        None => (true, parent),
    };
    let run = &start[..start.len().min(PARENT_PREFIX.len() + 1 + PARENT_HASH_CHARS)];
    generated_run(run)?;
    let rest = &start[run.len()..];
    let role = |separator: char| {
        [SYSTEM, USER].into_iter().any(|role| {
            rest.strip_prefix(separator)
                .and_then(|rest| rest.strip_prefix(role))
                .is_some_and(|rest| rest.starts_with(separator))
        })
    };
    let matches = if slice {
        role('-') && rest.ends_with(".slice")
    } else {
        role('/')
    };
    matches.then_some((run, rest))
}

// Accept only `doramcap-compose-‹lowercase hex hash›`.
fn generated_run(run: &str) -> Option<&str> {
    run.strip_prefix(&format!("{PARENT_PREFIX}-"))
        .filter(|hash| {
            hash.len() == PARENT_HASH_CHARS
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
}

/// Docker `cgroup_parent` giving `service` its own group below the run's `role` group.
/// Systemd names escape `-` so service names add no slice levels.
pub(crate) fn service_parent(
    placement: &CgroupPlacement,
    role: Role,
    service: &str,
) -> std::result::Result<String, String> {
    let run = run_name(placement)?;
    let role = role.name();
    if placement.parent.ends_with(".slice") {
        Ok(format!(
            "{run}-{role}-{}.slice",
            service.replace('-', SLICE_DASH)
        ))
    } else {
        Ok(format!("/{run}/{role}/{service}"))
    }
}

/// Systemd's escape for `-` inside one slice name component.
const SLICE_DASH: &str = "\\x2d";

// Host path of a generated systemd slice or cgroupfs parent.
fn parent_path(parent: &str) -> std::result::Result<PathBuf, String> {
    if parent.ends_with(".slice") {
        Ok(slice_path(parent))
    } else {
        cgroup::host_path(parent)
    }
}

fn slice_path(slice: &str) -> PathBuf {
    let stem = slice
        .strip_suffix(".slice")
        .expect("generated slice suffix");
    let mut path = PathBuf::from(cgroup::CGROUP_ROOT);
    for (index, _) in stem.match_indices('-') {
        path.push(format!("{}.slice", &stem[..index]));
    }
    path.join(slice)
}

/// Adopt Docker's accounting parent and remove its empty per-run hierarchy at cleanup.
pub struct DockerCgroup {
    placement: CgroupPlacement,
    stop: Duration,
}
impl DockerCgroup {
    /// Bind the prepared parent and cleanup budget; Docker creates the directory.
    /// Systemd cleanup requires access to the local host's system bus and permission to stop units.
    pub fn group(placement: CgroupPlacement, stop: Duration) -> Rc<cgroup::Group> {
        Rc::new(cgroup::Group::new(Rc::new(Self { placement, stop })))
    }
}

// Limit removal to the generated run group; shared ancestor slices remain owned by Docker.
fn run_path(placement: &CgroupPlacement) -> std::result::Result<PathBuf, String> {
    let run = run_name(placement)?;
    if parent_path(&placement.parent)? != Path::new(&placement.path) {
        return Err("Compose accounting parent and host path differ".into());
    }
    if placement.parent.ends_with(".slice") {
        Ok(slice_path(&format!("{run}.slice")))
    } else {
        cgroup::host_path(&format!("/{run}"))
    }
}

fn verify_empty_if_present(path: &Path) -> std::result::Result<(), String> {
    if path.try_exists().map_err(|e| e.to_string())? {
        cgroup::verify_empty(path)?;
    }
    Ok(())
}

// Remove an empty cgroupfs tree deepest first; the kernel refuses any group that gains a process.
fn remove_tree(path: &Path) -> std::result::Result<(), String> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            remove_tree(&entry.path())?;
        }
    }
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

impl Backend for DockerCgroup {
    fn create(&self) -> LocalBoxFuture<'_, std::result::Result<PathBuf, String>> {
        // Docker creates the parent before container exec; adopt only that prepared location.
        Box::pin(async { Ok(PathBuf::from(&self.placement.path)) })
    }
    fn remove(&self) -> LocalBoxFuture<'_, std::result::Result<Removal, String>> {
        Box::pin(async {
            let path = run_path(&self.placement)?;
            tokio::time::timeout(self.stop, async {
                // Check the whole run before removing anything, including user services.
                verify_empty_if_present(&path)?;
                if self.placement.parent.ends_with(".slice") {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or("Compose slice name is not UTF-8")?;
                    // Stopping the run slice stops and removes every slice below it.
                    cgroup::slice::stop(&cgroup::slice::connect().await?, name).await?;
                } else {
                    remove_tree(&path)?;
                }
                if path.try_exists().map_err(|e| e.to_string())? {
                    return Err(format!(
                        "{}: Compose run cgroup remains after removal",
                        path.display()
                    ));
                }
                Ok(Removal::Removed)
            })
            .await
            .map_err(|_| format!("Compose accounting cleanup exceeded {:?}", self.stop))?
        })
    }
}

#[cfg(test)]
mod tests;
