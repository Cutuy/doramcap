//! Bounded access to frozen app inputs.
use std::path::Path;
use tokio::io::AsyncReadExt;

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
