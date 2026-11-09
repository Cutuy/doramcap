//! Observe shared-memory metadata; a failed traversal never yields a partial snapshot.
use crate::{HostSharedMemory, PosixShmObject, SystemVShmObject};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

const POSIX_DIRECTORY: &str = "dev/shm";
const SYSTEM_V_TABLE: &str = "proc/sysvipc/shm";

pub(super) fn observe(root: &Path) -> Result<HostSharedMemory, String> {
    Ok(HostSharedMemory {
        posix: posix(&root.join(POSIX_DIRECTORY))?,
        system_v: system_v(&root.join(SYSTEM_V_TABLE))?,
    })
}

fn posix(directory: &Path) -> Result<Vec<PosixShmObject>, String> {
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
        let entry = entry.map_err(|e| format!("/{POSIX_DIRECTORY}{relative}: {e}"))?;
        let name = entry.file_name();
        if name == c"." || name == c".." {
            continue;
        }
        let name_text = name.to_str().map_err(|_| {
            format!("/{POSIX_DIRECTORY}{relative}: shared-memory name is not UTF-8")
        })?;
        let path = format!("{relative}/{name_text}");
        let error = |e: rustix::io::Errno| format!("/{POSIX_DIRECTORY}{path}: {e}");
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
        objects.push(PosixShmObject {
            path: format!("/{POSIX_DIRECTORY}{path}"),
            device: metadata.st_dev,
            inode: metadata.st_ino,
            mode: metadata.st_mode,
            uid: metadata.st_uid,
            gid: metadata.st_gid,
            size_bytes: metadata.st_size as u64,
            modified_seconds: metadata.st_mtime as i64,
            modified_nanos: metadata.st_mtime_nsec as u32,
            changed_seconds: metadata.st_ctime as i64,
            changed_nanos: metadata.st_ctime_nsec as u32,
        });
        if let Some(child) = child {
            let identity = (metadata.st_dev, metadata.st_ino);
            if !ancestors.insert(identity) {
                return Err(format!(
                    "/{POSIX_DIRECTORY}{path}: recursive directory mount"
                ));
            }
            directories.push((child, path, identity));
        }
    }
    Ok(objects)
}

fn system_v(path: &Path) -> Result<Vec<SystemVShmObject>, String> {
    let capture = || -> Result<Vec<SystemVShmObject>, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut lines = BufReader::new(file).lines();
        let header = lines
            .next()
            .ok_or("missing shared-memory table header")?
            .map_err(|e| e.to_string())?;
        let columns: Vec<_> = header.split_whitespace().collect();
        let index = |name| {
            columns
                .iter()
                .position(|column| *column == name)
                .ok_or_else(|| format!("missing {name} column"))
        };
        let required = [
            "shmid", "key", "perms", "uid", "gid", "size", "atime", "dtime", "ctime",
        ];
        let indices = required
            .map(index)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut objects = Vec::new();
        for (line_number, line) in lines.enumerate() {
            let line = line.map_err(|e| e.to_string())?;
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() != columns.len() {
                return Err(format!(
                    "invalid shared-memory table row {}",
                    line_number + 2
                ));
            }
            let value = |position| fields[indices[position]];
            let invalid =
                |position| format!("invalid {} in row {}", required[position], line_number + 2);
            objects.push(SystemVShmObject {
                id: value(0).parse().map_err(|_| invalid(0))?,
                key: value(1).parse().map_err(|_| invalid(1))?,
                mode: u32::from_str_radix(value(2), 8).map_err(|_| invalid(2))?,
                uid: value(3).parse().map_err(|_| invalid(3))?,
                gid: value(4).parse().map_err(|_| invalid(4))?,
                size_bytes: value(5).parse().map_err(|_| invalid(5))?,
                attached_seconds: value(6).parse().map_err(|_| invalid(6))?,
                detached_seconds: value(7).parse().map_err(|_| invalid(7))?,
                changed_seconds: value(8).parse().map_err(|_| invalid(8))?,
            });
        }
        Ok(objects)
    };
    capture().map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    };

    const EMPTY_TABLE: &str =
        "key shmid perms size cpid lpid nattch uid gid cuid cgid atime dtime ctime rss swap\n";

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(POSIX_DIRECTORY)).unwrap();
        fs::create_dir_all(root.path().join("proc/sysvipc")).unwrap();
        fs::write(root.path().join(SYSTEM_V_TABLE), EMPTY_TABLE).unwrap();
        root
    }

    #[test]
    fn observes_nested_objects_without_reading_payloads_or_following_links() {
        // Arrange
        let root = fixture();
        let directory = root.path().join(POSIX_DIRECTORY);
        fs::create_dir(directory.join("nested")).unwrap();
        let path = directory.join("nested/data");
        let file = File::create(&path).unwrap();
        file.set_len(1 << 40).unwrap();
        file.set_permissions(fs::Permissions::from_mode(0o0)).unwrap();
        symlink("missing-target", directory.join("link")).unwrap();
        let expected = file.metadata().unwrap();

        // Act
        let snapshot = observe(root.path()).unwrap();

        // Assert
        assert!(snapshot.system_v.is_empty());
        assert_eq!(snapshot.posix.len(), 3);
        let link = snapshot
            .posix
            .iter()
            .find(|object| object.path == "/dev/shm/link")
            .unwrap();
        assert_eq!(link.mode & 0o170000, 0o120000);
        let object = snapshot
            .posix
            .iter()
            .find(|object| object.path == "/dev/shm/nested/data")
            .unwrap();
        assert_eq!(object.size_bytes, 1 << 40);
        assert_eq!(object.inode, expected.ino());
        assert_eq!(object.device, expected.dev());
        assert_eq!((object.uid, object.gid), (expected.uid(), expected.gid()));
        assert_eq!(object.mode & 0o777, 0);
        assert_eq!(object.modified_seconds, expected.mtime());
        assert_eq!(object.changed_nanos, expected.ctime_nsec() as u32);
    }

    #[test]
    fn system_v_metadata_uses_kernel_column_names_and_octal_permissions() {
        // Arrange
        let root = fixture();
        fs::write(
            root.path().join(SYSTEM_V_TABLE),
            concat!(
                "shmid key perms uid gid size atime dtime ctime extra\n",
                "20 -8 1600 1000 1001 8589934592 10 11 12 ignored\n",
                "3 0 644 2 4 4096 0 0 13 ignored\n",
            ),
        )
        .unwrap();

        // Act
        let objects = observe(root.path()).unwrap().system_v;

        // Assert
        assert_eq!(objects.len(), 2);
        assert_eq!(
            objects.iter().find(|object| object.id == 20).unwrap(),
            &SystemVShmObject {
                id: 20,
                key: -8,
                mode: 0o1600,
                uid: 1000,
                gid: 1001,
                size_bytes: 8_589_934_592,
                attached_seconds: 10,
                detached_seconds: 11,
                changed_seconds: 12,
            }
        );
    }

    #[test]
    fn any_capture_error_leaves_the_whole_snapshot_absent() {
        // Arrange
        let root = fixture();
        fs::write(root.path().join(POSIX_DIRECTORY).join("present"), "data").unwrap();
        fs::write(root.path().join(SYSTEM_V_TABLE), "bad header\n").unwrap();

        // Act
        let state = crate::observe_root(root.path(), root.path());

        // Assert
        assert!(state.host_shm.is_none());
        assert!(state.host_shm_error.contains("proc/sysvipc/shm"));
        assert!(state.host_shm_error.contains("missing shmid column"));
        assert!(state.configuration.is_some());

        // Act / Assert: empty success remains different from unavailable capture.
        fs::write(root.path().join(SYSTEM_V_TABLE), EMPTY_TABLE).unwrap();
        fs::remove_file(root.path().join(POSIX_DIRECTORY).join("present")).unwrap();
        let state = crate::observe_root(root.path(), root.path());
        assert_eq!(state.host_shm, Some(HostSharedMemory::default()));
        assert!(state.host_shm_error.is_empty());
    }

    #[test]
    fn malformed_or_unreadable_sources_fail_instead_of_omitting_entries() {
        // Arrange
        let root = fixture();
        for table in [
            "",
            "key shmid perms size uid gid atime dtime ctime\n0 1 600\n",
            "key shmid perms size uid gid atime dtime ctime\n0 1 nope 10 1 1 0 0 0\n",
        ] {
            fs::write(root.path().join(SYSTEM_V_TABLE), table).unwrap();
            // Act / Assert
            assert!(observe(root.path()).is_err());
        }
        fs::remove_dir(root.path().join(POSIX_DIRECTORY)).unwrap();
        assert!(observe(root.path()).unwrap_err().contains("dev/shm"));
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
        let directory = root.path().join(POSIX_DIRECTORY);
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
                assert!(state.host_shm.is_none());
                assert!(state.host_shm_error.contains("recursive directory mount"));
            } else {
                let snapshot = state.host_shm.unwrap();
                assert!(state.host_shm_error.is_empty());
                for alias in if name == "two" {
                    &["one", "two"][..]
                } else {
                    &["one"][..]
                } {
                    assert!(
                        snapshot
                            .posix
                            .iter()
                            .any(|object| { object.path == format!("/dev/shm/{alias}/payload") })
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "uses host /dev/shm and util-linux ipcmk/ipcrm"]
    fn native_posix_and_system_v_metadata_without_payload_access() {
        use std::{process::Command, time::Instant};
        struct Segment(String);
        impl Drop for Segment {
            fn drop(&mut self) {
                let _ = Command::new("ipcrm").args(["-m", &self.0]).status();
            }
        }
        // Arrange
        let directory = tempfile::tempdir_in("/dev/shm").unwrap();
        let path = directory.path().join("unreadable-payload");
        let file = File::create(&path).unwrap();
        file.set_permissions(fs::Permissions::from_mode(0o0)).unwrap();
        let created = Command::new("ipcmk").args(["-M", "8192"]).output().unwrap();
        assert!(created.status.success());
        let output = String::from_utf8(created.stdout).unwrap();
        let segment = Segment(output.split_whitespace().last().unwrap().to_owned());
        let id = segment.0.parse::<u64>().unwrap();

        for bytes in [4096, 1 << 40] {
            file.set_len(bytes).unwrap();
            // Act
            let started = Instant::now();
            let snapshot = observe(Path::new("/")).unwrap();
            println!(
                "host shm capture with {bytes} byte object: {:?}",
                started.elapsed()
            );
            // Assert
            let posix = snapshot
                .posix
                .iter()
                .find(|object| object.path == path.to_str().unwrap())
                .unwrap();
            assert_eq!(posix.size_bytes, bytes);
            assert_eq!(posix.mode & 0o777, 0);
            let system_v = snapshot
                .system_v
                .iter()
                .find(|object| object.id == id)
                .unwrap();
            assert_eq!(system_v.size_bytes, 8192);
        }
    }
}
