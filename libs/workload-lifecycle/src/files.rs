//! Shared filesystem observations and recording/process readiness notifications.
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{io, os::unix::fs::MetadataExt, path::Path};
use tokio::sync::watch;

/// Conventional application mount path; native declarations can select other paths or backing.
pub const SHM_PATH: &str = "/dev/shm";

/// Identify live host shared memory, including bind aliases and hugetlbfs mounts at any path.
/// Requires an absolute source; follows symlinks and reads filesystem metadata only.
pub fn is_host_shm_source(source: &Path) -> io::Result<bool> {
    absolute_source(source)?;
    let source_metadata = std::fs::metadata(source)?;
    let host = Path::new(SHM_PATH);
    let path = std::ffi::CString::new(source.as_os_str().as_encoded_bytes())?;
    let mut filesystem = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: path is NUL-terminated and statfs initializes the output on success.
    let filesystem = unsafe {
        if libc::statfs(path.as_ptr(), filesystem.as_mut_ptr()) != 0 {
            return Err(io::Error::last_os_error());
        }
        filesystem.assume_init()
    };
    Ok(filesystem.f_type as u32 == libc::HUGETLBFS_MAGIC as u32
        || source_metadata.dev() == std::fs::metadata(host)?.dev()
        || source.canonicalize()?.starts_with(host.canonicalize()?))
}

/// Whether an absolute recursive bind source exposes the default host shm tree or an ancestor alias.
/// Resolves symlinks and compares native directory identities; reads no contents.
pub fn exposes_host_shm_tree(source: &Path) -> io::Result<bool> {
    absolute_source(source)?;
    let source = source.canonicalize()?;
    let metadata = std::fs::metadata(&source)?;
    let host = Path::new(SHM_PATH).canonicalize()?;
    if host.starts_with(&source) {
        return Ok(true);
    }
    for ancestor in host.ancestors() {
        let ancestor = std::fs::metadata(ancestor)?;
        if (metadata.dev(), metadata.ino()) == (ancestor.dev(), ancestor.ino()) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn absolute_source(source: &Path) -> io::Result<()> {
    if source.is_absolute() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "shared-memory source must be absolute: {}",
                source.display()
            ),
        ))
    }
}

/// Subscribe before inspecting paths. Keep the watcher alive while waiting.
/// Notifications coalesce; the first watcher error remains available to every wait.
/// Read-only access does not wake waiters.
pub fn watch_changes(
    paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<(RecommendedWatcher, watch::Receiver<notify::Result<()>>)> {
    let (changed, receiver) = watch::channel(Ok(()));
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.as_ref().is_ok_and(|event| event.kind.is_access()) {
            return;
        }
        changed.send_modify(|status| {
            if status.is_ok() {
                *status = event.map(|_| ());
            }
        });
    })
    .map_err(io::Error::other)?;
    for path in paths {
        watcher
            .watch(path.as_ref(), RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
    }
    Ok((watcher, receiver))
}

/// Await a change; return watcher failures and unexpected channel closure.
pub async fn wait_for_change(changes: &mut watch::Receiver<notify::Result<()>>) -> io::Result<()> {
    changes.changed().await.map_err(io::Error::other)?;
    changes
        .borrow_and_update()
        .as_ref()
        .map(|_| ())
        .map_err(|error| io::Error::other(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    const EVENT_DEADLINE: Duration = Duration::from_secs(2);
    const READ_QUIET_INTERVAL: Duration = Duration::from_millis(50);

    #[test]
    fn shared_memory_sources_must_be_absolute() {
        // Arrange
        let source = Path::new("relative/shared-memory");
        // Act
        let live = is_host_shm_source(source).unwrap_err();
        let exposed = exposes_host_shm_tree(source).unwrap_err();
        // Assert
        for error in [live, exposed] {
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("must be absolute"));
        }
    }

    #[test]
    #[ignore = "requires an existing hugetlbfs mount at /dev/hugepages; reads metadata only"]
    fn native_hugepage_mount_is_live_outside_the_default_shm_path() {
        // Arrange
        let source = Path::new("/dev/hugepages");
        // Act
        let live = is_host_shm_source(source).unwrap();
        // Assert
        assert!(live);
    }

    #[test]
    fn host_shm_detection_follows_symlinks_without_reading_payloads() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        // Arrange
        let name = format!(
            "doramcap-shm-source-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let source = Path::new("/dev/shm").join(&name);
        let alias = std::env::temp_dir().join(&name);
        std::fs::write(&source, b"live").unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o0)).unwrap();
        symlink(&source, &alias).unwrap();

        // Act
        let direct = is_host_shm_source(&source);
        let linked = is_host_shm_source(&alias);
        let ordinary =
            is_host_shm_source(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
        std::fs::remove_file(&alias).unwrap();
        std::fs::remove_file(&source).unwrap();
        let missing = is_host_shm_source(&source);

        // Assert
        assert!(direct.unwrap());
        assert!(linked.unwrap());
        assert!(!ordinary.unwrap());
        assert_eq!(missing.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn creation_wakes_waiter_and_inspection_does_not() {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "file-changes-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("recording");
        let (_directory_watcher, mut created) = watch_changes([&root]).unwrap();

        // Act
        std::fs::write(&file, b"record").unwrap();
        tokio::time::timeout(EVENT_DEADLINE, wait_for_change(&mut created))
            .await
            .unwrap()
            .unwrap();

        // Assert
        assert!(file.exists());

        // Act: inspections must not feed another readiness wake.
        let (_file_watcher, mut changes) = watch_changes([&file]).unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"record");

        // Assert
        assert!(
            tokio::time::timeout(READ_QUIET_INTERVAL, wait_for_change(&mut changes))
                .await
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
