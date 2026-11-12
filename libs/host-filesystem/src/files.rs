//! Private filesystem records shared by workload backends.
use super::Result;
use sha2::{Digest, Sha256};
use std::{io, os::unix::process::CommandExt, path::Path};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};

/// Lowercase SHA-256 identity for pinned manifests and private record names.
pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
/// Create an owner-only record once and flush its bytes; existing files fail.
pub async fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    Ok(())
}

/// Create a fresh owner-only directory; existing paths and missing parents fail.
pub async fn create_private_dir(path: &Path) -> std::io::Result<()> {
    tokio::fs::DirBuilder::new().mode(0o700).create(path).await
}

/// Kill an owned child process group when this guard is dropped.
pub struct ProcessGroup(i32);
impl ProcessGroup {
    /// Spawn a child in a fresh process group and return the child and its cleanup guard.
    /// Spawn and PID lookup errors are returned as I/O errors.
    pub fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
        command.kill_on_drop(true);
        command.as_std_mut().process_group(0);
        let child = command.spawn()?;
        let group = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .filter(|&id| id > 1)
            .ok_or_else(|| io::Error::other("spawned child has no valid process group ID"))?;
        Ok((child, Self(group)))
    }
}
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: spawn created this group and excluded special kill targets.
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}
