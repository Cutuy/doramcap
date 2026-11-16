use super::*;
use prost::Message;
use std::path::PathBuf;
use workload_lifecycle::cgroup::{Backend, Removal};

struct RemovalFixture {
    directory: PathBuf,
    cgroup: PathBuf,
    state: State,
}
impl RemovalFixture {
    fn new(native: bool) -> Self {
        let name = format!(
            "supervisor-removal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(&name);
        std::fs::create_dir(&directory).unwrap();
        let cgroup = if native {
            Path::new(workload_lifecycle::cgroup::CGROUP_ROOT).join(&name)
        } else {
            directory.join("group")
        };
        std::fs::create_dir(&cgroup).expect("native case requires writable cgroup v2");
        let metadata = std::fs::metadata(&cgroup).unwrap();
        let ledger = Ownership {
            run: Some(MonitorRun {
                output_directory: directory.to_str().unwrap().into(),
                ..Default::default()
            }),
            boot_id: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim()
                .into(),
            cgroup: cgroup.to_str().unwrap().into(),
            cgroup_device: metadata.dev(),
            cgroup_inode: metadata.ino(),
            ..Default::default()
        };
        ownership::save(&ledger).unwrap();
        let state = Rc::new(RefCell::new(Some(Runtime {
            child: None,
            kill: None,
            ledger,
            forced: false,
            daemon_reaped: true,
        })));
        Self {
            directory,
            cgroup,
            state,
        }
    }
    fn backend(&self) -> CgroupBackend {
        CgroupBackend {
            state: self.state.clone(),
        }
    }
    fn saved(&self) -> Ownership {
        Ownership::decode(
            std::fs::read(self.directory.join("ownership.pb"))
                .unwrap()
                .as_slice(),
        )
        .unwrap()
    }
}
impl Drop for RemovalFixture {
    fn drop(&mut self) {
        if self.cgroup.exists() {
            std::fs::remove_dir(&self.cgroup).expect("test left a populated or nonempty group");
        }
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn missing_group_removal_saves_completion_and_is_repeatable() {
    // Arrange: the retained runtime's group has already disappeared.
    let fixture = RemovalFixture::new(false);
    std::fs::remove_dir(&fixture.cgroup).unwrap();
    let backend = fixture.backend();
    // Act / Assert: absent resources and already-completed removal both succeed.
    for _ in 0..2 {
        assert!(matches!(backend.remove().await.unwrap(), Removal::Removed));
        assert!(fixture.saved().cleanup_verified);
        assert!(!fixture.cgroup.exists());
    }
}

#[tokio::test]
async fn missing_group_still_requires_the_direct_child_to_be_reaped_and_released() {
    // Arrange
    let fixture = RemovalFixture::new(false);
    std::fs::remove_dir(&fixture.cgroup).unwrap();
    fixture.state.borrow_mut().as_mut().unwrap().daemon_reaped = false;
    let backend = fixture.backend();
    // Act / Assert: missing resources do not bypass unconfirmed reaping.
    assert!(backend.remove().await.is_err());
    assert!(!fixture.saved().cleanup_verified);
    // Arrange: a retained child handle still prevents removal even if the flag says reaped.
    let child = tokio::process::Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    {
        let mut state = fixture.state.borrow_mut();
        let runtime = state.as_mut().unwrap();
        runtime.daemon_reaped = true;
        runtime.child = Some(child);
    }
    // Act / Assert
    assert!(backend.remove().await.is_err());
    assert!(!fixture.saved().cleanup_verified);
    let mut child = fixture
        .state
        .borrow_mut()
        .as_mut()
        .unwrap()
        .child
        .take()
        .unwrap();
    child.kill().await.unwrap();
}

#[tokio::test]
async fn completed_cleanup_preserves_a_replaced_group() {
    // Arrange: another group now occupies the recorded path after removal completed.
    let fixture = RemovalFixture::new(false);
    fixture
        .state
        .borrow_mut()
        .as_mut()
        .unwrap()
        .ledger
        .cleanup_verified = true;
    std::fs::rename(&fixture.cgroup, fixture.directory.join("original")).unwrap();
    std::fs::create_dir(&fixture.cgroup).unwrap();
    let replacement = std::fs::metadata(&fixture.cgroup).unwrap().ino();
    std::fs::write(fixture.cgroup.join("foreign"), b"retained").unwrap();
    // Act / Assert
    assert!(fixture.backend().remove().await.is_err());
    assert_eq!(
        std::fs::metadata(&fixture.cgroup).unwrap().ino(),
        replacement
    );
    assert_eq!(
        std::fs::read(fixture.cgroup.join("foreign")).unwrap(),
        b"retained"
    );
    std::fs::remove_file(fixture.cgroup.join("foreign")).unwrap();
}

#[tokio::test]
#[ignore = "requires writable cgroup v2; build unprivileged and elevate only this test executable"]
async fn removal_retries_persistence_after_the_real_cgroup_was_removed() {
    // Arrange: a pre-existing scratch file makes persistence fail after kernel removal.
    let fixture = RemovalFixture::new(true);
    let scratch = fixture.directory.join("ownership.next");
    std::fs::write(&scratch, b"retained scratch evidence").unwrap();
    let backend = fixture.backend();
    // Act
    assert!(backend.remove().await.is_err());
    // Assert: the cgroup was removed, while durable completion remains unconfirmed.
    assert!(!fixture.cgroup.exists());
    assert!(!fixture.saved().cleanup_verified);
    assert_eq!(
        std::fs::read(&scratch).unwrap(),
        b"retained scratch evidence"
    );
    // Arrange: the test-owned storage fault is cleared.
    std::fs::remove_file(scratch).unwrap();
    // Act / Assert: retry and repeated completed removal persist verified cleanup.
    for _ in 0..2 {
        assert!(matches!(backend.remove().await.unwrap(), Removal::Removed));
        assert!(!fixture.cgroup.exists());
        assert!(fixture.saved().cleanup_verified);
    }
}
