use super::*;
use std::{
    process::Command,
    sync::atomic::{AtomicU16, Ordering},
};

const STOP: Duration = Duration::from_secs(10);
static SEQUENCE: AtomicU16 = AtomicU16::new(0);

fn placement(slice: bool) -> CgroupPlacement {
    let name = format!(
        "{PARENT_PREFIX}-{:08x}{:04x}-system",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let parent = if slice {
        format!("{name}.slice")
    } else {
        format!("/{name}")
    };
    let path = if slice {
        slice_path(&parent)
    } else {
        cgroup::host_path(&parent).unwrap()
    };
    CgroupPlacement {
        parent,
        path: path.to_string_lossy().into_owned(),
    }
}

#[test]
fn systemd_parent_expands_to_the_driver_hierarchy() {
    // Arrange
    let parent = "doramcap-compose-abcdef-system.slice";
    // Act
    let path = slice_path(parent);
    // Assert
    assert_eq!(path, Path::new(cgroup::CGROUP_ROOT).join("doramcap.slice/doramcap-compose.slice/doramcap-compose-abcdef.slice/doramcap-compose-abcdef-system.slice"));
}

#[tokio::test]
async fn cleanup_rejects_foreign_names_and_mismatched_paths_before_side_effects() {
    for parent in [
        "system.slice",
        "doramcap-compose.slice",
        "doramcap-compose-123456789abc.slice",
        "/",
        "/foreign",
        "doramcap-compose-ABCDEF123456-system.slice",
    ] {
        // Arrange
        let group = DockerCgroup::group(
            CgroupPlacement {
                parent: parent.into(),
                path: "/sys/fs/cgroup".into(),
            },
            STOP,
        );
        // Act / Assert
        assert!(group.remove().await.is_err(), "accepted {parent}");
    }
    // Arrange
    let mut placement = placement(true);
    placement.path = "/sys/fs/cgroup/system.slice".into();
    let group = DockerCgroup::group(placement, STOP);
    // Act / Assert
    assert!(
        group
            .remove()
            .await
            .unwrap_err()
            .contains("host path differ")
    );
}

struct SliceFixture {
    placement: CgroupPlacement,
    service: String,
}
impl SliceFixture {
    fn new(sibling: bool) -> Self {
        let placement = placement(true);
        let service = placement.parent.replace(".slice", ".service");
        let fixture = Self { placement, service };
        let paths = cleanup_paths(&fixture.placement).unwrap();
        let slice = if sibling {
            paths[1].file_name().unwrap().to_str().unwrap()
        } else {
            &fixture.placement.parent
        };
        let output = Command::new("systemd-run")
            .args([
                "--quiet",
                "--collect",
                "--unit",
                &fixture.service,
                "--slice",
                slice,
                "--",
                "/bin/sleep",
                "infinity",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture
    }
    fn stop_service(&self) {
        let output = Command::new("systemctl")
            .args(["stop", &self.service])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
impl Drop for SliceFixture {
    fn drop(&mut self) {
        let paths = cleanup_paths(&self.placement).unwrap();
        let parent = paths[1].file_name().unwrap().to_str().unwrap();
        let _ = Command::new("systemctl")
            .args(["stop", &self.service, &self.placement.parent, parent])
            .output();
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn systemd_cleanup_preserves_live_descendants_and_removes_empty_run_slices() {
    // Arrange
    let fixture = SliceFixture::new(false);
    let group = DockerCgroup::group(fixture.placement.clone(), STOP);
    let paths = cleanup_paths(&fixture.placement).unwrap();
    // Act / Assert: an empty cgroup.procs in the slice cannot establish empty descendants.
    assert!(
        std::fs::read_to_string(paths[0].join("cgroup.procs"))
            .unwrap()
            .trim()
            .is_empty()
    );
    assert!(group.remove().await.unwrap_err().contains("live processes"));
    assert!(paths.iter().all(|path| path.exists()));
    // Act
    fixture.stop_service();
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(paths.iter().all(|path| !path.exists()));
    assert!(
        paths[1].parent().unwrap().exists(),
        "shared Docker slice removed"
    );
    // Act / Assert: completed cleanup is repeatable.
    assert!(group.remove().await.unwrap().is_none());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn systemd_cleanup_preserves_a_live_sibling_when_the_leaf_is_absent() {
    // Arrange
    let fixture = SliceFixture::new(true);
    let group = DockerCgroup::group(fixture.placement.clone(), STOP);
    let paths = cleanup_paths(&fixture.placement).unwrap();
    assert!(!paths[0].exists());
    // Act / Assert
    assert!(group.remove().await.unwrap_err().contains("live processes"));
    assert!(paths[1].exists());
    // Act / Assert: a missing leaf still permits cleanup of the empty per-run parent.
    fixture.stop_service();
    assert!(group.remove().await.unwrap().is_none());
    assert!(!paths[1].exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn cgroupfs_cleanup_removes_an_empty_run_parent_and_is_repeatable() {
    // Arrange
    let placement = placement(false);
    let path = PathBuf::from(&placement.path);
    std::fs::create_dir(&path).unwrap();
    let group = DockerCgroup::group(placement, STOP);
    // Act
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(!path.exists());
    // Act / Assert
    assert!(group.remove().await.unwrap().is_none());
}
