//! Descriptor-relative access to private workflow evidence and retained artifacts.
use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
use std::{fs::File, io, os::unix::fs::OpenOptionsExt, path::Path};
use tokio::io::AsyncReadExt;

/// Create a fresh owner-only directory; existing paths and missing parents fail.
pub(crate) async fn create_private_dir(path: &Path) -> io::Result<()> {
    tokio::fs::DirBuilder::new().mode(0o700).create(path).await
}

/// Read bounded private metadata without following its final component.
/// Rejects non-regular files and growth beyond `limit`; parents are caller-owned.
pub(crate) async fn read_regular(path: &Path, limit: u64) -> crate::compose::Result<Vec<u8>> {
    use crate::compose::error::invalid;
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .await?;
    let metadata = file.metadata().await?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(invalid("workflow metadata is not a bounded regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > limit {
        return Err(invalid("workflow metadata exceeds its size limit"));
    }
    Ok(bytes)
}

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
