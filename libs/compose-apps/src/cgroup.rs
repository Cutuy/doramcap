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

mod systemd;

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
    let hash = &hex::encode(digest)[..PARENT_HASH_CHARS];
    let name = format!("{PARENT_PREFIX}-{hash}-system");
    match info.cgroup_driver {
        Some(bollard::models::SystemInfoCgroupDriverEnum::SYSTEMD) => {
            let parent = format!("{name}.slice");
            let path = slice_path(&parent);
            Ok(CgroupPlacement {
                parent,
                path: path.to_string_lossy().into_owned(),
            })
        }
        Some(bollard::models::SystemInfoCgroupDriverEnum::CGROUPFS) => {
            let parent = format!("/{name}");
            let path = cgroup::host_path(&parent).map_err(invalid)?;
            Ok(CgroupPlacement {
                parent,
                path: path.to_string_lossy().into_owned(),
            })
        }
        driver => Err(invalid(format!(
            "unsupported Docker cgroup driver: {driver:?}"
        ))),
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

// Limit removal to generated per-run paths; shared ancestor slices remain owned by Docker.
fn cleanup_paths(placement: &CgroupPlacement) -> std::result::Result<Vec<PathBuf>, String> {
    let slice = placement.parent.strip_suffix(".slice");
    let name = slice
        .or_else(|| placement.parent.strip_prefix('/'))
        .ok_or("invalid Compose accounting parent")?;
    let hash = name
        .strip_prefix(&format!("{PARENT_PREFIX}-"))
        .and_then(|name| name.strip_suffix("-system"))
        .filter(|hash| {
            hash.len() == PARENT_HASH_CHARS
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
        .ok_or("accounting parent is not a generated Compose run parent")?;
    let path = if slice.is_some() {
        slice_path(&placement.parent)
    } else {
        cgroup::host_path(&placement.parent)?
    };
    if path != Path::new(&placement.path) {
        return Err("Compose accounting parent and host path differ".into());
    }
    let mut paths = vec![path];
    if slice.is_some() {
        paths.push(slice_path(&format!("{PARENT_PREFIX}-{hash}.slice")));
    }
    Ok(paths)
}

fn verify_empty_if_present(path: &Path) -> std::result::Result<(), String> {
    if path.try_exists().map_err(|e| e.to_string())? {
        cgroup::verify_empty(path)?;
    }
    Ok(())
}

impl Backend for DockerCgroup {
    fn create(&self) -> LocalBoxFuture<'_, std::result::Result<PathBuf, String>> {
        // Docker creates the parent before container exec; adopt only that prepared location.
        Box::pin(async { Ok(PathBuf::from(&self.placement.path)) })
    }
    fn remove(&self) -> LocalBoxFuture<'_, std::result::Result<Removal, String>> {
        Box::pin(async {
            let paths = cleanup_paths(&self.placement)?;
            tokio::time::timeout(self.stop, async {
                // Check the whole run before stopping either slice, including sibling descendants.
                for path in &paths {
                    verify_empty_if_present(path)?;
                }
                if self.placement.parent.ends_with(".slice") {
                    systemd::remove(&paths).await?;
                } else {
                    match std::fs::remove_dir(&paths[0]) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error.to_string()),
                    }
                }
                for path in &paths {
                    if path.try_exists().map_err(|e| e.to_string())? {
                        return Err(format!(
                            "{}: Compose accounting cgroup remains after removal",
                            path.display()
                        ));
                    }
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
