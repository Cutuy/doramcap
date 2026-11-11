//! Private filesystem records used by Docker workflows.
use super::Result;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::AsyncWriteExt;

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
