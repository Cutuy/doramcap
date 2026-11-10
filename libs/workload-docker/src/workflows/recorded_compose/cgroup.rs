//! Docker-owned accounting parents, expressed using the daemon's configured driver.
use super::{ComposeRun, plan::CgroupPlacement};
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
pub(super) async fn placement(
    adapter: &DockerAdapter,
    run: &ComposeRun,
) -> Result<CgroupPlacement> {
    let info = tokio::time::timeout(
        Duration::from_secs(run.readiness_seconds),
        adapter.docker.info(),
    )
    .await
    .map_err(|_| invalid("Docker cgroup capability deadline exceeded"))?
    .map_err(|e| invalid(format!("Docker cgroup capabilities: {e}")))?;
    if info.cgroup_version != Some(bollard::models::SystemInfoCgroupVersionEnum::_2) {
        return Err(invalid(
            "system-app accounting requires Docker cgroup v2 CPU and memory accounting",
        ));
    }
    let repository = std::fs::canonicalize(&run.repository_root)?;
    let digest = Sha256::new()
        .chain_update(repository.as_os_str().as_encoded_bytes())
        .chain_update([0])
        .chain_update(run.id.as_bytes())
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

pub(super) struct DockerCgroup {
    placement: CgroupPlacement,
}
impl DockerCgroup {
    pub fn group(placement: CgroupPlacement) -> Rc<cgroup::Group> {
        Rc::new(cgroup::Group::new(Rc::new(Self { placement })))
    }
}
impl Backend for DockerCgroup {
    fn create(&self) -> LocalBoxFuture<'_, std::result::Result<PathBuf, String>> {
        // Docker creates the parent before container exec; adopt only that prepared location.
        Box::pin(async { Ok(PathBuf::from(&self.placement.path)) })
    }
    fn remove(&self) -> LocalBoxFuture<'_, std::result::Result<Removal, String>> {
        Box::pin(async {
            let path = Path::new(&self.placement.path);
            if path.try_exists().map_err(|e| e.to_string())? {
                cgroup::verify_empty(path)?;
            }
            Ok(Removal::RetainedEmpty(path.to_owned()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn systemd_parent_expands_to_the_driver_hierarchy() {
        // Arrange
        let parent = "doramcap-compose-abcdef-system.slice";
        // Act
        let path = slice_path(parent);
        // Assert
        assert_eq!(path, Path::new(cgroup::CGROUP_ROOT).join("doramcap.slice/doramcap-compose.slice/doramcap-compose-abcdef.slice/doramcap-compose-abcdef-system.slice"));
    }
}
