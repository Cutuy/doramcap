//! Single-manager ownership of an artifact root.
use std::{
    fs::{File, OpenOptions},
    io,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};

/// Holds an advisory exclusive lock until dropped. Managers sharing an artifact
/// root must acquire this before Docker discovery and keep it while draining.
pub struct ManagerLease {
    _file: File,
}
impl ManagerLease {
    /// Acquire the lock without waiting. The root must be an existing trusted
    /// absolute directory. Errors include an occupied lock, an unsafe symlink,
    /// or an inaccessible/non-regular lock file. No prior file is truncated.
    pub fn acquire(root: &Path) -> io::Result<Self> {
        if !root.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "artifact root must be absolute",
            ));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root.join(".manager.lock"))?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "manager lock is not a regular file",
            ));
        }
        // SAFETY: the live File owns this descriptor until the lease is dropped.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_one_manager_holds_a_root_and_drop_releases_it() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-lease-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let first = ManagerLease::acquire(&root).unwrap();

        // Act / Assert
        assert!(ManagerLease::acquire(&root).is_err());
        drop(first);
        let second = ManagerLease::acquire(&root).unwrap();
        drop(second);
        std::fs::remove_file(root.join(".manager.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
