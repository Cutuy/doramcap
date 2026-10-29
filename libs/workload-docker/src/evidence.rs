//! Descriptor-relative access to private workflow evidence and retained artifacts.
use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
use std::{fs::File, io, os::unix::fs::OpenOptionsExt, path::Path};

/// Open a directory without following its final component if it is a symlink.
/// The caller supplies a trusted parent path and owns the returned descriptor.
pub fn root(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(path)
}

/// Open a descendant beneath `root`, rejecting symlinks and path escapes.
/// Requires Linux openat2 support. Files must be regular; `directory` requests
/// a directory instead. The returned descriptor is independent of `root`.
pub fn open(root: &File, path: &str, directory: bool) -> io::Result<File> {
    let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK;
    if directory {
        flags |= OFlags::DIRECTORY;
    }
    let fd = openat2(
        root,
        path,
        flags,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let file = File::from(fd);
    if !directory && !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "artifact is not a regular file",
        ));
    }
    Ok(file)
}

/// Persist bounded JSON once, accepting an existing identical regular file.
/// The caller protects the parent directory; successful writes are synced.
pub fn write_private_once<T: serde::Serialize + ?Sized>(
    path: &Path,
    value: &T,
) -> Result<(), String> {
    use std::{
        io::{Read, Write},
        os::unix::fs::OpenOptionsExt,
    };
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("artifact metadata exceeds its size limit".into());
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)
                .map_err(|e| e.to_string())?;
            if !file.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err("index is not a regular file".into());
            }
            let mut existing = Vec::new();
            file.take(2 * 1024 * 1024)
                .read_to_end(&mut existing)
                .map_err(|e| e.to_string())?;
            if existing == bytes {
                Ok(())
            } else {
                Err("existing artifact index differs".into())
            }
        }
        Err(error) => Err(error.to_string()),
    }
}
