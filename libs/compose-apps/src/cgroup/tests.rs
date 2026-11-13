use super::*;
use std::{
    process::Command,
    sync::atomic::{AtomicU16, Ordering},
};

const STOP: Duration = Duration::from_secs(10);
static SEQUENCE: AtomicU16 = AtomicU16::new(0);

/// A run placement in a new repository group.
fn placement(slice: bool) -> CgroupPlacement {
    let repository = format!(
        "{PARENT_PREFIX}-{:08x}{:04x}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    in_repository(&repository, slice)
}

/// Another run placement in `placement`'s repository group.
fn sibling(placement: &CgroupPlacement) -> CgroupPlacement {
    let run = run_name(placement).unwrap();
    in_repository(repository_name(run), placement.parent.ends_with(".slice"))
}

fn in_repository(repository: &str, slice: bool) -> CgroupPlacement {
    let run = format!("{:012x}", SEQUENCE.fetch_add(1, Ordering::Relaxed));
    let parent = if slice {
        format!("{repository}-{run}-{SYSTEM}.slice")
    } else {
        format!("/{repository}/{run}/{SYSTEM}")
    };
    CgroupPlacement {
        path: parent_path(&parent).unwrap().to_string_lossy().into_owned(),
        parent,
    }
}

fn run_path(placement: &CgroupPlacement) -> PathBuf {
    run_paths(placement).unwrap().0
}

fn repository_path(placement: &CgroupPlacement) -> PathBuf {
    run_paths(placement).unwrap().1
}

/// Host path of the group a service's containers are placed below.
fn service_path(placement: &CgroupPlacement, role: Role, service: &str) -> PathBuf {
    parent_path(&service_parent(placement, role, service).unwrap()).unwrap()
}

#[test]
fn service_parents_nest_below_the_run_role_group_for_both_drivers() {
    // Arrange
    let slice = CgroupPlacement {
        parent: "doramcap-compose-0123456789ab-ba9876543210-system.slice".into(),
        path: String::new(),
    };
    let directory = CgroupPlacement {
        parent: "/doramcap-compose-0123456789ab/ba9876543210/system".into(),
        path: String::new(),
    };
    // Act
    let systemd = service_parent(&slice, Role::User, "my-camera").unwrap();
    let cgroupfs = service_parent(&directory, Role::System, "my-camera").unwrap();
    // Assert: escaped dashes keep the service one level below its role group.
    let run = "doramcap-compose-0123456789ab-ba9876543210";
    assert_eq!(systemd, format!(r"{run}-user-my\x2dcamera.slice"));
    assert_eq!(
        cgroup::slice::path(&systemd),
        Path::new(cgroup::CGROUP_ROOT).join(format!(
            "doramcap.slice/doramcap-compose.slice/doramcap-compose-0123456789ab.slice/\
             {run}.slice/{run}-user.slice/{systemd}"
        ))
    );
    assert_eq!(
        cgroupfs,
        "/doramcap-compose-0123456789ab/ba9876543210/system/my-camera"
    );
}

#[tokio::test]
async fn cleanup_rejects_foreign_names_and_mismatched_paths_before_side_effects() {
    for parent in [
        "system.slice",
        "doramcap-compose.slice",
        "doramcap-compose-0123456789ab.slice",
        "doramcap-compose-0123456789ab-system.slice",
        "doramcap-compose-0123456789ab-ba9876543210.slice",
        "doramcap-compose-0123456789ab-ba9876543210-user.slice",
        "/",
        "/foreign/system",
        "/doramcap-compose-0123456789ab/system",
        "/doramcap-compose-0123456789ab/ba9876543210",
        "doramcap-compose-0123456789ab-ABCDEF123456-system.slice",
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

/// A run's systemd groups, with an optional service unit standing in for a container.
struct SliceFixture {
    placement: CgroupPlacement,
    service: String,
}
impl SliceFixture {
    /// Run a long-lived service in `service`'s per-service slice below `role`.
    fn running(placement: CgroupPlacement, role: Role, service: &str) -> Self {
        let slice = service_parent(&placement, role, service).unwrap();
        let fixture = Self {
            service: placement.parent.replace(".slice", ".service"),
            placement,
        };
        systemd(&[
            "systemd-run",
            "--quiet",
            "--collect",
            "--unit",
            &fixture.service,
            "--slice",
            &slice,
            "--",
            "/bin/sleep",
            "infinity",
        ]);
        fixture
    }
    /// Keep per-service slices started after their last unit exits, as Docker does.
    fn emptied(placement: CgroupPlacement, services: &[(Role, &str)]) -> Self {
        for (role, service) in services {
            let slice = service_parent(&placement, *role, service).unwrap();
            systemd(&["systemctl", "start", &slice]);
            // Systemd creates a slice's cgroup only once a unit has run inside it.
            systemd(&[
                "systemd-run",
                "--quiet",
                "--wait",
                "--collect",
                "--slice",
                &slice,
                "/bin/true",
            ]);
        }
        Self {
            service: placement.parent.replace(".slice", ".service"),
            placement,
        }
    }
    fn stop_service(&self) {
        systemd(&["systemctl", "stop", &self.service]);
    }
}
impl Drop for SliceFixture {
    fn drop(&mut self) {
        let repository = format!(
            "{}.slice",
            repository_name(run_name(&self.placement).unwrap())
        );
        let _ = Command::new("systemctl")
            .args(["stop", &self.service, &repository])
            .output();
    }
}

fn systemd(command: &[&str]) {
    let output = Command::new(command[0])
        .args(&command[1..])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn systemd_cleanup_preserves_a_live_user_app_then_removes_the_repository_slice() {
    // Arrange: the system group is absent; only a user service runs.
    let fixture = SliceFixture::running(placement(true), Role::User, "camera");
    let group = DockerCgroup::group(fixture.placement.clone(), STOP);
    let repository = repository_path(&fixture.placement);
    let user = service_path(&fixture.placement, Role::User, "camera");
    assert!(!Path::new(&fixture.placement.path).exists());
    // Act / Assert
    assert!(group.remove().await.unwrap_err().contains("live processes"));
    assert!(user.exists());
    // Act
    fixture.stop_service();
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(!repository.exists());
    assert!(
        repository.parent().unwrap().exists(),
        "shared Docker slice removed"
    );
    // Act / Assert: completed cleanup is repeatable.
    assert!(group.remove().await.unwrap().is_none());
}

/// Empty cgroup directories below `root`, as a system app or container runtime may leave them.
fn nested_tree(root: &Path) -> Vec<PathBuf> {
    let tree = vec![
        root.join("container-a"),
        root.join("container-a/inner"),
        root.join("container-a/inner/deepest"),
        root.join("container-b"),
    ];
    for path in &tree {
        std::fs::create_dir_all(path).unwrap();
    }
    tree
}

/// A process held in one cgroup until dropped.
struct Member(std::process::Child);
impl Member {
    fn join(path: &Path) -> Self {
        let child = Command::new("/bin/sleep").arg("infinity").spawn().unwrap();
        std::fs::write(path.join("cgroup.procs"), child.id().to_string()).unwrap();
        Self(child)
    }
}
impl Drop for Member {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn systemd_cleanup_removes_every_service_slice_and_nested_empty_group() {
    // Arrange
    let services = [(Role::System, "dora"), (Role::User, "my-camera")];
    let fixture = SliceFixture::emptied(placement(true), &services);
    let group = DockerCgroup::group(fixture.placement.clone(), STOP);
    let mut tree = Vec::new();
    for (role, service) in services {
        tree.extend(nested_tree(&service_path(
            &fixture.placement,
            role,
            service,
        )));
    }
    let repository = repository_path(&fixture.placement);
    // Act
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(tree.iter().chain([&repository]).all(|path| !path.exists()));
    assert!(
        repository.parent().unwrap().exists(),
        "shared Docker slice removed"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn cgroupfs_cleanup_removes_the_entire_empty_run_tree_and_is_repeatable() {
    // Arrange
    let placement = placement(false);
    let repository = repository_path(&placement);
    let mut tree = nested_tree(&service_path(&placement, Role::System, "dora"));
    tree.extend(nested_tree(&service_path(&placement, Role::User, "camera")));
    let group = DockerCgroup::group(placement, STOP);
    // Act
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(tree.iter().chain([&repository]).all(|path| !path.exists()));
    // Act / Assert
    assert!(group.remove().await.unwrap().is_none());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn cgroupfs_cleanup_preserves_the_whole_tree_while_a_user_app_runs() {
    // Arrange
    let placement = placement(false);
    let repository = repository_path(&placement);
    let mut tree = nested_tree(&service_path(&placement, Role::System, "dora"));
    tree.extend(nested_tree(&service_path(&placement, Role::User, "camera")));
    let group = DockerCgroup::group(placement, STOP);
    let member = Member::join(tree.last().unwrap());
    // Act / Assert
    assert!(group.remove().await.unwrap_err().contains("live processes"));
    assert!(tree.iter().chain([&repository]).all(|path| path.exists()));
    // Act
    drop(member);
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(!repository.exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn systemd_cleanup_sweeps_earlier_runs_of_its_repository_only() {
    // Arrange: a crashed run left a live app; another repository has its own run.
    let current = placement(true);
    let earlier = SliceFixture::running(sibling(&current), Role::User, "camera");
    let other = SliceFixture::emptied(placement(true), &[(Role::System, "dora")]);
    let current = SliceFixture::emptied(current, &[(Role::System, "dora")]);
    let group = DockerCgroup::group(current.placement.clone(), STOP);
    // Act / Assert: this run is removed and the live leftover is reported.
    let error = group.remove().await.unwrap_err();
    assert!(error.contains("earlier Compose run"), "{error}");
    assert!(!run_path(&current.placement).exists());
    assert!(service_path(&earlier.placement, Role::User, "camera").exists());
    // Act
    earlier.stop_service();
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(!repository_path(&current.placement).exists());
    assert!(run_path(&other.placement).exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn cgroupfs_cleanup_sweeps_earlier_runs_of_its_repository_only() {
    // Arrange: a crashed run left a live app; another repository has its own run.
    let current = placement(false);
    let earlier = sibling(&current);
    let other = placement(false);
    nested_tree(&service_path(&current, Role::System, "dora"));
    nested_tree(&service_path(&other, Role::System, "dora"));
    let stale = nested_tree(&service_path(&earlier, Role::User, "camera"));
    let member = Member::join(stale.last().unwrap());
    let group = DockerCgroup::group(current.clone(), STOP);
    // Act / Assert: this run is removed and the live leftover is reported.
    let error = group.remove().await.unwrap_err();
    assert!(error.contains("earlier Compose run"), "{error}");
    assert!(!run_path(&current).exists());
    assert!(stale.iter().all(|path| path.exists()));
    // Act
    drop(member);
    assert!(group.remove().await.unwrap().is_none());
    // Assert
    assert!(!repository_path(&current).exists());
    assert!(run_path(&other).exists());
    remove_tree(&repository_path(&other)).unwrap();
}

/// Per-app paths the Console frontend names apps from; its tests read the same file.
const APP_PATHS: &str =
    include_str!("../../../../console/frontend/src/fixtures/app-cgroup-paths.json");
/// Docker names a container's group `docker-‹id›.scope` (systemd driver) or `‹id›` (cgroupfs).
const CONTAINER: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

/// Path as the CPU monitor reports it: below the cgroup root, with a leading `/`.
fn reported(path: &Path) -> String {
    format!(
        "/{}",
        path.strip_prefix(cgroup::CGROUP_ROOT).unwrap().display()
    )
}

#[test]
fn service_paths_match_the_console_app_name_fixture() {
    // Arrange
    let run = "doramcap-compose-0123456789ab";
    let slice = CgroupPlacement {
        parent: format!("{run}-fedcba987654-system.slice"),
        path: String::new(),
    };
    let directory = CgroupPlacement {
        parent: format!("/{run}/fedcba987654/system"),
        path: String::new(),
    };
    // Act
    let camera = service_path(&slice, Role::User, "camera-driver");
    let paths = serde_json::json!([
        { "path": reported(&camera.join(format!("docker-{CONTAINER}.scope"))), "name": "camera-driver" },
        { "path": reported(&service_path(&slice, Role::System, "cpu-monitor")), "name": "cpu-monitor" },
        { "path": reported(&service_path(&directory, Role::System, "live-bridge").join(CONTAINER)), "name": "live-bridge" },
    ]);
    // Assert
    let fixture: serde_json::Value = serde_json::from_str(APP_PATHS).unwrap();
    assert_eq!(paths, fixture["compose"]);
}
