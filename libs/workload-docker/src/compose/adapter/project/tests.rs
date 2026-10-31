use super::*;
use serde_json::json;
use std::{io::Read, os::unix::fs::PermissionsExt};

const SNAPSHOT: &str = "experiment-container.stopped.json";
const BEFORE: &[u8] = br#"{"State":{"ExitCode":1}}"#;
const AFTER: &[u8] = br#"{"State":{"ExitCode":0}}"#;

fn entries(directory: &Path) -> Vec<std::ffi::OsString> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

#[tokio::test]
async fn replacement_keeps_one_private_snapshot_and_preserves_existing_readers() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    save_stopped_inspection(root.path(), "experiment", "container", BEFORE.into())
        .await
        .unwrap();
    let destination = root.path().join(SNAPSHOT);
    let mut previous_reader = std::fs::File::open(&destination).unwrap();
    // Act
    save_stopped_inspection(root.path(), "experiment", "container", AFTER.into())
        .await
        .unwrap();
    // Assert: replacement preserves the old inode for existing readers.
    let mut previous = Vec::new();
    previous_reader.read_to_end(&mut previous).unwrap();
    assert_eq!(previous, BEFORE);
    assert_eq!(std::fs::read(&destination).unwrap(), AFTER);
    assert_eq!(entries(root.path()), [std::ffi::OsString::from(SNAPSHOT)]);
    assert_eq!(
        std::fs::metadata(destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[tokio::test]
async fn replacement_does_not_follow_a_destination_symlink() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("artifacts");
    std::fs::create_dir(&directory).unwrap();
    let outside = root.path().join("outside");
    std::fs::write(&outside, b"keep").unwrap();
    let destination = directory.join(SNAPSHOT);
    std::os::unix::fs::symlink(&outside, &destination).unwrap();
    // Act
    save_stopped_inspection(&directory, "experiment", "container", AFTER.into())
        .await
        .unwrap();
    // Assert
    assert_eq!(std::fs::read(outside).unwrap(), b"keep");
    assert_eq!(std::fs::read(&destination).unwrap(), AFTER);
    assert!(
        std::fs::symlink_metadata(destination)
            .unwrap()
            .file_type()
            .is_file()
    );
}

#[tokio::test]
async fn failed_publish_preserves_existing_entry_and_removes_temporary_file() {
    // Arrange: a directory blocks publishing a file at the requested destination.
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join(SNAPSHOT);
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("sentinel"), b"keep").unwrap();
    // Act
    let result =
        save_stopped_inspection(root.path(), "experiment", "container", AFTER.into()).await;
    // Assert
    assert!(matches!(result, Err(crate::compose::Error::Io(_))));
    assert_eq!(
        std::fs::read(destination.join("sentinel")).unwrap(),
        b"keep"
    );
    assert_eq!(entries(root.path()), [std::ffi::OsString::from(SNAPSHOT)]);
}

struct Mounted(std::ffi::CString);
impl Drop for Mounted {
    fn drop(&mut self) {
        // SAFETY: emergency cleanup detaches only this test's own tmpfs.
        unsafe {
            libc::umount2(self.0.as_ptr(), libc::MNT_DETACH);
        }
    }
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN; fills a private 64 KiB tmpfs"]
async fn disk_full_preserves_previous_snapshot_and_removes_temporary_file() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let mountpoint = std::ffi::CString::new(root.path().as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: live C strings and a test-owned temporary mountpoint.
    assert_eq!(
        unsafe {
            libc::mount(
                c"tmpfs".as_ptr(),
                mountpoint.as_ptr(),
                c"tmpfs".as_ptr(),
                0,
                c"size=64k,mode=0700".as_ptr().cast(),
            )
        },
        0
    );
    let mounted = Mounted(mountpoint);
    save_stopped_inspection(root.path(), "experiment", "container", BEFORE.into())
        .await
        .unwrap();
    let replacement = serde_json::to_vec(&json!({"large": "x".repeat(1024 * 1024)})).unwrap();
    // Act
    let result = save_stopped_inspection(root.path(), "experiment", "container", replacement).await;
    // Assert
    assert!(
        matches!(result, Err(crate::compose::Error::Io(ref error)) if error.kind() == std::io::ErrorKind::StorageFull),
        "{result:?}"
    );
    assert_eq!(std::fs::read(root.path().join(SNAPSHOT)).unwrap(), BEFORE);
    assert_eq!(entries(root.path()), [std::ffi::OsString::from(SNAPSHOT)]);
    // SAFETY: strict unmount after the writer and all inspection handles have closed.
    assert_eq!(unsafe { libc::umount2(mounted.0.as_ptr(), 0) }, 0);
}
