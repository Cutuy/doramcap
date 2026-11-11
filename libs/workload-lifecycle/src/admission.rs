//! Non-blocking, repository-wide admission for runs, builds and operator cleanup.
use std::{
    fmt,
    fs::{File, TryLockError},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
};

/// Exclusive ownership of a canonical repository directory on a local filesystem.
/// Writes no files; aliases of the same directory share admission across users.
/// Drop or process exit releases ownership after every inherited holder exits.
#[derive(Debug)]
pub struct Admission {
    repository: PathBuf,
    directory: File,
}

/// Why repository admission was refused.
#[derive(Debug)]
pub enum Refusal {
    /// Another run, build or cleanup holds this repository.
    Busy,
    /// Invalid, inaccessible or unsupported repository; includes the input and cause.
    Invalid(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => f.write_str("another run owns this repository"),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for Refusal {}

impl Admission {
    /// Lock an existing absolute directory, resolving aliases before admission.
    /// Returns `Busy` immediately on contention; path, access and locking errors are `Invalid`.
    /// Children do not retain admission across exec unless `keep_across_exec` is called.
    pub fn acquire(repository: &Path) -> Result<Self, Refusal> {
        let invalid = |cause: &dyn fmt::Display| {
            Refusal::Invalid(format!("repository {}: {cause}", repository.display()))
        };
        if !repository.is_absolute() {
            return Err(invalid(&"root must be absolute"));
        }
        let canonical = repository.canonicalize().map_err(|e| invalid(&e))?;
        let directory = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(&canonical)
            .map_err(|e| invalid(&e))?;
        match directory.try_lock() {
            Ok(()) => Ok(Self {
                repository: canonical,
                directory,
            }),
            Err(TryLockError::WouldBlock) => Err(Refusal::Busy),
            Err(TryLockError::Error(e)) => Err(invalid(&e)),
        }
    }

    /// Canonical repository root covered by this admission.
    pub fn repository(&self) -> &Path {
        &self.repository
    }

    /// Let subsequently spawned children retain admission across exec until they exit.
    /// Applies process-wide to child launches; dropping this holder does not unlock theirs.
    /// Returns the OS error if descriptor flags cannot be read or changed.
    pub fn keep_across_exec(&self) -> std::io::Result<()> {
        let fd = self.directory.as_raw_fd();
        // Preserve every descriptor flag except close-on-exec.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } == -1
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
