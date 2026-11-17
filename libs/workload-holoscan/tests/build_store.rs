use prost::Message;
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use workload_holoscan::{
    BuildAttempt, BuildAttemptRequest, BuildKey, BuildRecord, BuildResult, BuildStoreRequest,
    Selection, build_result,
    builds::{Attempt, lookup},
};
use workload_lifecycle::{
    admission::Admission,
    launch::{SourceRecord, SourceState},
};

struct Fixture {
    root: PathBuf,
    request: BuildStoreRequest,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-build-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let repository = root.join("repository");
        fs::create_dir(&repository).unwrap();
        let request = BuildStoreRequest {
            root: root.join("builds").to_str().unwrap().into(),
            key: Some(BuildKey {
                repository_root: repository.to_str().unwrap().into(),
                selection: Some(Selection {
                    application: "a native/name with spaces".into(),
                    language: "python".into(),
                    mode: "".into(),
                }),
            }),
        };
        Self { root, request }
    }
    async fn begin(&self, id: &str) -> Attempt {
        Attempt::begin(BuildAttemptRequest {
            store: Some(self.request.clone()),
            id: id.into(),
            source: Some(source()),
        })
        .await
        .unwrap()
    }
    fn repository(&self) -> &Path {
        Path::new(&self.request.key.as_ref().unwrap().repository_root)
    }
    fn directory(&self) -> PathBuf {
        fs::read_dir(&self.request.root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    }
    fn success(&self) -> BuildResult {
        BuildResult {
            termination: Some(build_result::Termination::ExitCode(0)),
            cleanup_verified: true,
            record: Some(BuildRecord {
                selection: self.request.key.as_ref().unwrap().selection.clone(),
                image_id: "sha256:built-image".into(),
                source: Some(source()),
            }),
            ..Default::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn source() -> SourceRecord {
    SourceRecord {
        source_commit: "current-head".into(),
        source_state: SourceState::Dirty as i32,
    }
}

#[tokio::test]
async fn latest_incomplete_and_failed_attempts_invalidate_an_older_success() {
    // Arrange
    let fixture = Fixture::new();
    assert!(
        lookup(fixture.request.clone())
            .await
            .unwrap()
            .latest
            .is_none()
    );
    let mut first = fixture.begin("first").await;
    first.finish(fixture.success()).await.unwrap();
    assert!(
        lookup(fixture.request.clone())
            .await
            .unwrap()
            .record
            .is_some()
    );

    // Act: the next attempt is durably incomplete before its caller can launch.
    let mut next = fixture.begin("next").await;
    fs::write(next.stdout_path(), b"preserved output\n").unwrap();
    let latest = BuildAttempt::decode(
        fs::read(fixture.directory().join("latest.pb"))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    // Assert
    assert_eq!(latest.id, "next");
    assert!(latest.result.is_none());
    assert!(lookup(fixture.request.clone()).await.is_err());
    assert!(Admission::acquire(fixture.repository()).is_err());

    // Act: a failed process, output and cleanup remain independent observations.
    let failed = BuildResult {
        termination: Some(build_result::Termination::ExitCode(7)),
        output_error: "stdout sync failed".into(),
        cleanup_error: "writer remains".into(),
        record: fixture.success().record,
        ..Default::default()
    };
    let saved = next.finish(failed).await.unwrap();
    // Assert
    let result = saved.result.unwrap();
    assert_eq!(
        result.termination,
        Some(build_result::Termination::ExitCode(7))
    );
    assert_eq!(result.output_error, "stdout sync failed");
    assert_eq!(result.cleanup_error, "writer remains");
    assert!(result.record.is_none());
    assert!(
        lookup(fixture.request.clone())
            .await
            .unwrap()
            .record
            .is_none()
    );
    assert_eq!(fs::read(next.stdout_path()).unwrap(), b"preserved output\n");
    assert!(Admission::acquire(fixture.repository()).is_ok());
}

#[tokio::test]
async fn interrupted_attempt_and_canonical_alias_do_not_reveal_previous_evidence() {
    // Arrange
    let fixture = Fixture::new();
    let mut first = fixture.begin("completed").await;
    first.finish(fixture.success()).await.unwrap();
    let interrupted = fixture.begin("interrupted").await;
    drop(interrupted);
    let alias = fixture.root.join("alias");
    symlink(fixture.repository(), &alias).unwrap();
    let mut request = fixture.request.clone();
    request.key.as_mut().unwrap().repository_root = alias.to_str().unwrap().into();
    // Act
    let latest = lookup(request).await.unwrap();
    // Assert
    assert_eq!(latest.latest.unwrap().id, "interrupted");
    assert!(latest.record.is_none());
    assert!(Admission::acquire(fixture.repository()).is_ok());
}

#[tokio::test]
async fn failed_publication_retains_admission_and_retries_without_older_success() {
    // Arrange
    let fixture = Fixture::new();
    let mut attempt = fixture.begin("retry").await;
    let obstruction = fixture.directory().join("latest.retry.next");
    fs::write(&obstruction, b"unrelated existing file").unwrap();
    // Act
    assert!(attempt.finish(fixture.success()).await.is_err());
    // Assert: prior state is incomplete, and the foreign obstruction was preserved.
    assert!(lookup(fixture.request.clone()).await.is_err());
    let incomplete = BuildAttempt::decode(
        fs::read(fixture.directory().join("latest.pb"))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    assert!(incomplete.result.is_none());
    assert!(Admission::acquire(fixture.repository()).is_err());
    assert_eq!(fs::read(&obstruction).unwrap(), b"unrelated existing file");
    fs::remove_file(&obstruction).unwrap();
    // Act
    let completed = attempt.finish(fixture.success()).await.unwrap();
    let repeated = attempt.finish(fixture.success()).await.unwrap();
    // Assert
    assert_eq!(completed, repeated);
    assert!(
        lookup(fixture.request.clone())
            .await
            .unwrap()
            .record
            .is_some()
    );
    assert!(Admission::acquire(fixture.repository()).is_ok());
}

#[tokio::test]
async fn selection_components_never_become_paths_and_records_do_not_cross_keys() {
    // Arrange
    let fixture = Fixture::new();
    let mut attempt = fixture.begin("safe-id").await;
    let artifacts = attempt.artifacts().to_owned();
    attempt.finish(fixture.success()).await.unwrap();
    // Assert
    let slot = artifacts
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(slot.len(), 64);
    assert!(slot.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        fs::metadata(&artifacts).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(fixture.directory().join("latest.pb"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    // Act: an omitted versus explicit native mode stays an exact selection distinction.
    let mut other = fixture.request.clone();
    other.key.as_mut().unwrap().selection.as_mut().unwrap().mode = "default".into();
    assert!(lookup(other).await.unwrap().latest.is_none());
    // Act: invalid attempt paths fail before launch/store changes.
    for id in ["", ".", "..", "a/b", "/absolute"] {
        assert!(
            Attempt::begin(BuildAttemptRequest {
                store: Some(fixture.request.clone()),
                id: id.into(),
                source: None
            })
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn malformed_or_replaced_latest_is_an_error_instead_of_not_built() {
    // Arrange
    let fixture = Fixture::new();
    let mut attempt = fixture.begin("complete").await;
    attempt.finish(fixture.success()).await.unwrap();
    let path = fixture.directory().join("latest.pb");
    // Act / Assert
    fs::write(&path, b"not a protobuf").unwrap();
    assert!(lookup(fixture.request.clone()).await.is_err());
    let foreign = BuildAttempt {
        version: 1,
        key: Some(BuildKey {
            repository_root: "/other".into(),
            selection: fixture.request.key.as_ref().unwrap().selection.clone(),
        }),
        id: "foreign".into(),
        ..Default::default()
    };
    fs::write(&path, foreign.encode_to_vec()).unwrap();
    assert!(lookup(fixture.request.clone()).await.is_err());
    fs::remove_file(&path).unwrap();
    symlink(fixture.repository(), &path).unwrap();
    assert!(lookup(fixture.request.clone()).await.is_err());
}

#[tokio::test]
async fn terminal_retry_is_per_attempt_and_does_not_republish_over_a_newer_attempt() {
    // Arrange
    let fixture = Fixture::new();
    let mut first = fixture.begin("first").await;
    let receipt = first.finish(fixture.success()).await.unwrap();
    let next = fixture.begin("next").await;
    // Act
    let repeated = first.finish(fixture.success()).await.unwrap();
    let latest = BuildAttempt::decode(
        fs::read(fixture.directory().join("latest.pb"))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    // Assert
    assert_eq!(repeated, receipt);
    assert_eq!(latest.id, "next");
    assert!(latest.result.is_none());
    assert!(lookup(fixture.request.clone()).await.is_err());
    assert!(Admission::acquire(fixture.repository()).is_err());
    drop(next);
}
