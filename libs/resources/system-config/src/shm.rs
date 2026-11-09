//! Observe shared-memory metadata; a failed traversal never yields a partial snapshot.
use crate::{HostSharedMemory, PosixShmObject, SystemVShmObject};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    os::unix::fs::MetadataExt,
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
    let mut objects = Vec::new();
    let mut directories = vec![directory.to_owned()];
    while let Some(parent) = directories.pop() {
        for entry in fs::read_dir(&parent).map_err(|e| format!("{}: {e}", parent.display()))? {
            let path = entry
                .map_err(|e| format!("{}: {e}", parent.display()))?
                .path();
            // lstat observes links without following them outside shm or into a cycle.
            let metadata =
                fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let relative = path
                .strip_prefix(directory)
                .expect("shm entry beneath root");
            let name = relative
                .to_str()
                .ok_or_else(|| format!("{}: shared-memory path is not UTF-8", path.display()))?;
            objects.push(PosixShmObject {
                path: format!("/{POSIX_DIRECTORY}/{name}"),
                device: metadata.dev(),
                inode: metadata.ino(),
                mode: metadata.mode(),
                uid: metadata.uid(),
                gid: metadata.gid(),
                size_bytes: metadata.size(),
                modified_seconds: metadata.mtime(),
                modified_nanos: metadata.mtime_nsec() as u32,
                changed_seconds: metadata.ctime(),
                changed_nanos: metadata.ctime_nsec() as u32,
            });
            if metadata.is_dir() {
                directories.push(path);
            }
        }
    }
    objects.sort_by(|a, b| a.path.cmp(&b.path));
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
        objects.sort_by_key(|object| object.id);
        Ok(objects)
    };
    capture().map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

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
        file.set_permissions(fs::Permissions::from_mode(0)).unwrap();
        symlink("missing-target", directory.join("link")).unwrap();
        let expected = file.metadata().unwrap();

        // Act
        let snapshot = observe(root.path()).unwrap();

        // Assert
        assert!(snapshot.system_v.is_empty());
        assert_eq!(snapshot.posix.len(), 3);
        assert_eq!(snapshot.posix[0].path, "/dev/shm/link");
        assert_eq!(snapshot.posix[0].mode & 0o170000, 0o120000);
        let object = &snapshot.posix[2];
        assert_eq!(object.path, "/dev/shm/nested/data");
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
        assert_eq!(
            objects.iter().map(|object| object.id).collect::<Vec<_>>(),
            [3, 20]
        );
        assert_eq!(
            objects[1],
            SystemVShmObject {
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
}
