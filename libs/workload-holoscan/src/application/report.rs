//! One authoritative run report, published before removal and refreshed with final cleanup facts.
use crate::HoloscanRunReport;
use prost::Message;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub(super) const FILE: &str = "holoscan-run.pb";
pub(super) const PENDING: &str = "holoscan-run.pending.pb";

pub(super) fn excluded(run: &workload_lifecycle::launch::RunPlan) -> Vec<std::path::PathBuf> {
    let mut excluded: Vec<_> = run
        .private_records
        .iter()
        .map(std::path::PathBuf::from)
        .collect();
    excluded.extend(
        [
            super::launch::CIDFILE,
            super::observation::STDOUT,
            super::observation::STDERR,
            super::observation::EVENTS,
            super::observation::RUNTIME,
            FILE,
            PENDING,
        ]
        .map(std::path::PathBuf::from),
    );
    excluded
}

/// Sync a complete private report, replace the current file, then acknowledge its directory.
/// The caller holds admission and has stopped writers; cancellation preserves previous/pending evidence.
/// Replacement has no cancellable await; errors never grant permission to discard resources.
pub(super) async fn save(artifacts: &Path, report: &HoloscanRunReport) -> Result<(), String> {
    let pending = artifacts.join(PENDING);
    match std::fs::remove_file(&pending) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&pending)
        .await
        .map_err(error)?;
    file.write_all(&report.encode_to_vec())
        .await
        .map_err(error)?;
    file.sync_all().await.map_err(error)?;
    drop(file);
    std::fs::rename(&pending, artifacts.join(FILE)).map_err(error)?;
    tokio::fs::File::open(artifacts)
        .await
        .map_err(error)?
        .sync_all()
        .await
        .map_err(error)
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    use workload_lifecycle::launch::CleanupRecord;

    #[tokio::test]
    async fn cleanup_refresh_replaces_one_complete_report_and_leaves_prior_evidence_on_failure() {
        // Arrange
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "holoscan-report-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let mut report = HoloscanRunReport {
            id: "run".into(),
            ..Default::default()
        };
        // Act
        save(&directory, &report).await.unwrap();
        report.cleanup = Some(CleanupRecord { verified: true });
        save(&directory, &report).await.unwrap();
        // Assert
        let published =
            HoloscanRunReport::decode(fs::read(directory.join(FILE)).unwrap().as_slice()).unwrap();
        assert_eq!(published, report);
        assert!(!directory.join(PENDING).exists());
        // Act
        fs::create_dir(directory.join(PENDING)).unwrap();
        report.id = "unpublished".into();
        let failure = save(&directory, &report).await;
        // Assert
        assert!(failure.is_err());
        let retained =
            HoloscanRunReport::decode(fs::read(directory.join(FILE)).unwrap().as_slice()).unwrap();
        assert_eq!(retained.id, "run");
        assert!(retained.cleanup.unwrap().verified);
        fs::remove_dir_all(directory).unwrap();
    }
}
