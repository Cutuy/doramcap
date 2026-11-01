//! Recording readiness and completion checks shared by workload backends.
use futures_util::future::join_all;
use orchestrator::Step;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::other(message)
}

/// Recording paths; an empty stream requires an armed regular recording only.
/// A supplied stream must grow before readiness. Finalization requires an MCAP footer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recording {
    pub stream: PathBuf,
    pub recording: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadinessInput {
    pub recordings: Vec<Recording>,
    pub timeout_seconds: u64,
}
/// Await independent application-output readiness probes on the caller's task.
pub struct WaitForRecordings;
impl Step for WaitForRecordings {
    type Input = ReadinessInput;
    type Output = ();
    async fn run(&mut self, input: ReadinessInput) -> orchestrator::Result<()> {
        let results = join_all(
            input
                .recordings
                .iter()
                .map(|output| wait_ready(output, input.timeout_seconds)),
        )
        .await;
        for result in results {
            result.map_err(|e| orchestrator::Error::message(e.to_string()))?;
        }
        Ok(())
    }
}
/// Verify recordings after their writers stop. Probes run concurrently and never
/// stop containers or publish artifacts; those remain the enclosing workflow's job.
pub struct FinalizeRecordings;
impl Step for FinalizeRecordings {
    type Input = Vec<Recording>;
    type Output = ();
    async fn run(&mut self, outputs: Vec<Recording>) -> orchestrator::Result<()> {
        let results = join_all(
            outputs
                .iter()
                .map(|output| finalized_recording(&output.recording)),
        )
        .await;
        for result in results {
            result.map_err(|e| orchestrator::Error::message(e.to_string()))?;
        }
        Ok(())
    }
}

async fn wait_ready(output: &Recording, timeout: u64) -> std::io::Result<()> {
    let start = tokio::time::Instant::now();
    let mut previous = None;
    loop {
        if output.stream.as_os_str().is_empty() {
            match tokio::fs::symlink_metadata(&output.recording).await {
                Ok(metadata) if metadata.is_file() => return Ok(()),
                Ok(_) => return Err(invalid("recording must be a regular file")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        } else if let Ok(metadata) = tokio::fs::metadata(&output.stream).await {
            if previous.is_some_and(|n| n > 0 && metadata.len() > n)
                && tokio::fs::try_exists(&output.recording).await?
            {
                return Ok(());
            }
            previous = Some(metadata.len());
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            return Err(invalid(
                "system app did not advance its stream/arm MCAP recording",
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
/// Verify the terminal MCAP footer and stable length after its writer exits.
/// Reject symlinks; the caller retains ownership of the recording path.
pub async fn finalized_recording(path: &Path) -> std::io::Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .await?;
    let len = file.metadata().await?.len();
    if len < 45 {
        return Err(invalid("MCAP is truncated"));
    }
    let mut header = [0u8; 8];
    file.read_exact(&mut header).await?;
    file.seek(std::io::SeekFrom::End(-37)).await?;
    let mut tail = [0u8; 37];
    file.read_exact(&mut tail).await?;
    if &header != b"\x89MCAP0\r\n"
        || tail[29..] != header
        || tail[0] != 2
        || u64::from_le_bytes(tail[1..9].try_into().unwrap()) != 20
    {
        return Err(invalid("MCAP terminal footer/magic absent"));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    if tokio::fs::metadata(path).await?.len() != len {
        return Err(invalid("recording changed after producer exit"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn subscriber_recording_can_be_armed_before_its_workload_produces_data() {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "recording-arm-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        tokio::fs::create_dir(&root).await.unwrap();
        let path = root.join("workload.mcap");
        tokio::fs::write(&path, []).await.unwrap();
        let output = Recording {
            stream: PathBuf::new(),
            recording: path.clone(),
        };
        // Act / Assert
        assert!(wait_ready(&output, 0).await.is_ok());
        tokio::fs::remove_file(&path).await.unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &path).unwrap();
        assert!(wait_ready(&output, 0).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[tokio::test]
    async fn rejects_truncation_and_requires_stream_advancement() {
        // Arrange
        let dir = std::env::temp_dir().join(format!(
            "recording-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        tokio::fs::create_dir(&dir).await.unwrap();
        let output = Recording {
            stream: dir.join("cpu.pb"),
            recording: dir.join("cpu.mcap"),
        };
        tokio::fs::write(&output.stream, b"stalled").await.unwrap();
        tokio::fs::write(&output.recording, b"\x89MCAP0\r\n")
            .await
            .unwrap();
        // Act / Assert
        assert!(finalized_recording(&output.recording).await.is_err());
        assert!(wait_ready(&output, 0).await.is_err());
        // Act: recording exists and an independent producer advances the stream.
        let stream = output.stream.clone();
        let write = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            tokio::fs::write(stream, b"advancing stream").await.unwrap();
        };
        let (result, ()) = tokio::join!(wait_ready(&output, 1), write);
        // Assert
        assert!(result.is_ok());
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
}
