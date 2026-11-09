//! Observe shared-memory metadata; a failed traversal never yields a partial snapshot.
use crate::FileMetadata;
use std::{collections::HashSet, path::Path};

const SHM_DIRECTORY: &str = "dev/shm";
const NANOS_PER_SECOND: i128 = 1_000_000_000;

pub(super) fn observe(root: &Path) -> Result<Vec<FileMetadata>, String> {
    entries(&root.join(SHM_DIRECTORY))
}

fn entries(directory: &Path) -> Result<Vec<FileMetadata>, String> {
    use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, fstat, open, openat, statat};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let opened = open(directory, flags, Mode::empty())
        .map_err(|e| format!("{}: {e}", directory.display()))?;
    let metadata = fstat(&opened).map_err(|e| format!("{}: {e}", directory.display()))?;
    let identity = (metadata.st_dev, metadata.st_ino);
    let opened = Dir::new(opened).map_err(|e| format!("{}: {e}", directory.display()))?;
    let mut ancestors = HashSet::from([identity]);
    let mut directories = vec![(opened, String::new(), identity)];
    let mut objects = Vec::new();
    while let Some((parent, relative, identity)) = directories.last_mut() {
        let Some(entry) = parent.next() else {
            ancestors.remove(identity);
            directories.pop();
            continue;
        };
        let entry = entry.map_err(|e| format!("/{SHM_DIRECTORY}{relative}: {e}"))?;
        let name = entry.file_name();
        if name == c"." || name == c".." {
            continue;
        }
        let name_text = name
            .to_str()
            .map_err(|_| format!("/{SHM_DIRECTORY}{relative}: shared-memory name is not UTF-8"))?;
        let path = format!("{relative}/{name_text}");
        let error = |e: rustix::io::Errno| format!("/{SHM_DIRECTORY}{path}: {e}");
        let fd = parent.fd().map_err(error)?;
        let mut metadata = statat(fd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(error)?;
        // Anchor traversal to directory handles; replacement symlinks cannot escape shm.
        let child = if FileType::from_raw_mode(metadata.st_mode) == FileType::Directory {
            let opened =
                openat(fd, name, flags | OFlags::NOFOLLOW, Mode::empty()).map_err(error)?;
            metadata = fstat(&opened).map_err(error)?;
            Some(Dir::new(opened).map_err(error)?)
        } else {
            None
        };
        let unix_ns = |seconds: i64, nanos: u32| {
            i64::try_from(i128::from(seconds) * NANOS_PER_SECOND + i128::from(nanos)).map_err(
                |_| format!("/{SHM_DIRECTORY}{path}: timestamp exceeds signed Unix nanoseconds"),
            )
        };
        objects.push(FileMetadata {
            path: format!("/{SHM_DIRECTORY}{path}"),
            mode: metadata.st_mode,
            uid: metadata.st_uid,
            gid: metadata.st_gid,
            size_bytes: metadata.st_size as u64,
            modified_unix_ns: unix_ns(metadata.st_mtime as i64, metadata.st_mtime_nsec as u32)?,
            changed_unix_ns: unix_ns(metadata.st_ctime as i64, metadata.st_ctime_nsec as u32)?,
        });
        if let Some(child) = child {
            let identity = (metadata.st_dev, metadata.st_ino);
            if !ancestors.insert(identity) {
                return Err(format!("/{SHM_DIRECTORY}{path}: recursive directory mount"));
            }
            directories.push((child, path, identity));
        }
    }
    Ok(objects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    use std::{
        fs::{self, File},
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    };

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(SHM_DIRECTORY)).unwrap();
        root
    }

    #[test]
    fn observes_nested_objects_without_reading_payloads_or_following_links() {
        // Arrange
        let root = fixture();
        let directory = root.path().join(SHM_DIRECTORY);
        fs::create_dir(directory.join("nested")).unwrap();
        let path = directory.join("nested/data");
        let file = File::create(&path).unwrap();
        file.set_len(1 << 40).unwrap();
        file.set_permissions(fs::Permissions::from_mode(0o0))
            .unwrap();
        symlink("missing-target", directory.join("link")).unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(
            std::time::UNIX_EPOCH + std::time::Duration::new(1_700_000_000, 123_456_789),
        ))
        .unwrap();
        let expected = file.metadata().unwrap();

        // Act
        let snapshot = observe(root.path()).unwrap();

        // Assert
        assert_eq!(snapshot.len(), 3);
        let link = snapshot
            .iter()
            .find(|object| object.path == "/dev/shm/link")
            .unwrap();
        assert_eq!(link.mode & 0o170000, 0o120000);
        let object = snapshot
            .iter()
            .find(|object| object.path == "/dev/shm/nested/data")
            .unwrap();
        assert_eq!(object.size_bytes, 1 << 40);
        assert_eq!((object.uid, object.gid), (expected.uid(), expected.gid()));
        assert_eq!(object.mode & 0o777, 0);
        assert_eq!(object.modified_unix_ns, 1_700_000_000_123_456_789);
        assert_eq!(
            object.changed_unix_ns,
            expected.ctime() * 1_000_000_000 + expected.ctime_nsec()
        );

        // Act / Assert: signed epoch timestamps retain subsecond precision before 1970.
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH - std::time::Duration::from_nanos(1)),
        )
        .unwrap();
        let snapshot = observe(root.path()).unwrap();
        assert_eq!(
            snapshot
                .iter()
                .find(|object| object.path == "/dev/shm/nested/data")
                .unwrap()
                .modified_unix_ns,
            -1
        );
    }

    #[test]
    fn failed_capture_returns_an_error_instead_of_partial_state() {
        // Arrange
        let root = fixture();
        let directory = root.path().join(SHM_DIRECTORY);
        fs::write(directory.join("present"), "data").unwrap();
        let invalid_name = std::ffi::OsString::from_vec(vec![0xff]);
        fs::write(directory.join(&invalid_name), "data").unwrap();

        // Act / Assert
        let error = crate::observe_root(root.path(), root.path()).unwrap_err();
        assert!(error.contains("not UTF-8"));

        // Act / Assert: an empty capture remains a successful observation.
        fs::remove_file(directory.join(invalid_name)).unwrap();
        fs::remove_file(directory.join("present")).unwrap();
        let state = crate::observe_root(root.path(), root.path()).unwrap();
        assert!(state.host_shm.is_empty());
    }

    #[test]
    fn a_missing_directory_fails_capture() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        // Act / Assert
        assert!(
            crate::observe_root(root.path(), root.path())
                .unwrap_err()
                .contains("dev/shm")
        );
    }

    #[test]
    #[ignore = "requires CAP_SYS_ADMIN and util-linux for an isolated mount namespace"]
    fn native_bind_aliases_are_retained_and_cycles_fail_capture() {
        use std::{env, process::Command};
        const CHILD: &str = "SYSTEM_CONFIG_SHM_MOUNT_TEST_CHILD";
        if env::var_os(CHILD).is_none() {
            // Re-exec only this test inside a private namespace before creating mounts.
            let status = Command::new("unshare")
                .args(["--mount", "--propagation", "private"])
                .arg(env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "shm::tests::native_bind_aliases_are_retained_and_cycles_fail_capture",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        struct Mounts(Vec<std::path::PathBuf>);
        impl Drop for Mounts {
            fn drop(&mut self) {
                for path in self.0.iter().rev() {
                    let _ = Command::new("umount").arg(path).status();
                }
            }
        }
        // Arrange: two independent paths bind the same metadata directory.
        let root = fixture();
        let directory = root.path().join(SHM_DIRECTORY);
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("payload"), "unread payload").unwrap();
        let mut mounts = Mounts(Vec::new());
        for name in ["one", "two", "cycle"] {
            let target = directory.join(name);
            fs::create_dir(&target).unwrap();
            let source = if name == "cycle" { &directory } else { &source };
            let status = Command::new("mount")
                .arg("--bind")
                .arg(source)
                .arg(&target)
                .status()
                .unwrap();
            assert!(status.success());
            mounts.0.push(target);
            // Act / Assert
            let state = crate::observe_root(root.path(), root.path());
            if name == "cycle" {
                assert!(state.unwrap_err().contains("recursive directory mount"));
            } else {
                let snapshot = state.unwrap().host_shm;
                for alias in if name == "two" {
                    &["one", "two"][..]
                } else {
                    &["one"][..]
                } {
                    assert!(
                        snapshot
                            .iter()
                            .any(|object| { object.path == format!("/dev/shm/{alias}/payload") })
                    );
                }
            }
        }
    }
}
