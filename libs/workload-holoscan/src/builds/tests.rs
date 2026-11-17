use super::*;
use crate::Selection;

const HANDSHAKE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
use std::{
    fs as sync_fs,
    os::unix::fs::DirBuilderExt,
    sync::atomic::{AtomicU64, Ordering},
};

struct FailAck;
impl FailAck {
    fn new(path: &Path) -> Self {
        FAIL_DIRECTORY_ACK.with(|target| *target.borrow_mut() = Some(path.to_owned()));
        Self
    }
}
impl Drop for FailAck {
    fn drop(&mut self) {
        FAIL_DIRECTORY_ACK.with(|target| *target.borrow_mut() = None);
    }
}

#[tokio::test]
async fn post_rename_failure_requires_admission_and_durability_ack_before_evidence() {
    // Arrange
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "holoscan-build-ack-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    sync_fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let repository = root.join("repository");
    sync_fs::create_dir(&repository).unwrap();
    let selection = Selection {
        application: "sample".into(),
        ..Default::default()
    };
    let store = BuildStoreRequest {
        root: root.join("builds").to_str().unwrap().into(),
        key: Some(BuildKey {
            repository_root: repository.to_str().unwrap().into(),
            selection: Some(selection.clone()),
        }),
    };
    let begin = |id: &str| BuildAttemptRequest {
        store: Some(store.clone()),
        id: id.into(),
        source: None,
    };
    let success = |image: &str| BuildResult {
        termination: Some(build_result::Termination::ExitCode(0)),
        cleanup_verified: true,
        record: Some(BuildRecord {
            selection: Some(selection.clone()),
            image_id: image.into(),
            source: None,
        }),
        ..Default::default()
    };
    let mut first = Attempt::begin(begin("first")).await.unwrap();
    first.finish(success("sha256:first")).await.unwrap();
    let mut next = Attempt::begin(begin("next")).await.unwrap();
    let failure = FailAck::new(&next.directory);
    // Act: the rename really completed, then its directory acknowledgement failed.
    assert!(next.finish(success("sha256:next")).await.is_err());
    let visible = BuildAttempt::decode(
        sync_fs::read(next.directory.join(LATEST))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    // Assert: visible terminal bytes are not eligible while the writer retains admission.
    assert_eq!(visible.id, "next");
    assert!(visible.result.unwrap().record.is_some());
    assert!(lookup(store.clone()).await.is_err());
    assert!(Admission::acquire(&repository).is_err());
    drop(failure);
    // Act / Assert: an identical retry acknowledges the visible publication and releases.
    next.finish(success("sha256:next")).await.unwrap();
    assert_eq!(
        lookup(store.clone())
            .await
            .unwrap()
            .record
            .unwrap()
            .image_id,
        "sha256:next"
    );

    // Arrange another completed replacement whose writer disappears before acknowledgement.
    let mut last = Attempt::begin(begin("last")).await.unwrap();
    let failure = FailAck::new(&last.directory);
    assert!(last.finish(success("sha256:last")).await.is_err());
    drop(last);
    // Act / Assert: recovery must itself acknowledge durability; an error returns no evidence.
    let error = lookup(store.clone()).await.unwrap_err().to_string();
    assert!(error.contains("durability acknowledgement"));
    drop(failure);
    let recovered = lookup(store.clone()).await.unwrap();
    assert_eq!(recovered.latest.unwrap().id, "last");
    assert_eq!(recovered.record.unwrap().image_id, "sha256:last");
    let admission = Admission::acquire(&repository).unwrap();
    assert!(
        lookup_admitted(store, &admission)
            .await
            .unwrap()
            .record
            .is_some()
    );
    drop(admission);
    sync_fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn directory_creation_retry_reacknowledges_its_parent_entry() {
    // Arrange
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "holoscan-build-parent-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    sync_fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let repository = root.join("repository");
    sync_fs::create_dir(&repository).unwrap();
    let store_root = root.join("builds");
    let selection = Selection {
        application: "sample".into(),
        ..Default::default()
    };
    let request = BuildAttemptRequest {
        store: Some(BuildStoreRequest {
            root: store_root.to_str().unwrap().into(),
            key: Some(BuildKey {
                repository_root: repository.to_str().unwrap().into(),
                selection: Some(selection),
            }),
        }),
        id: "retry".into(),
        source: None,
    };
    let failure = FailAck::new(&root);
    // Act / Assert: only the store-root's parent acknowledgement fails.
    assert!(Attempt::begin(request.clone()).await.is_err());
    assert!(store_root.is_dir());
    assert!(sync_fs::read_dir(&store_root).unwrap().next().is_none());
    assert!(Attempt::begin(request.clone()).await.is_err());
    assert!(sync_fs::read_dir(&store_root).unwrap().next().is_none());
    assert!(Admission::acquire(&repository).is_ok());
    drop(failure);
    // Act / Assert: acknowledging that same already-existing entry permits a later attempt.
    let attempt = Attempt::begin(request).await.unwrap();
    drop(attempt);
    sync_fs::remove_dir_all(root).unwrap();
}

struct RenameGate {
    path: PathBuf,
    entered: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}
static RENAME_GATE: std::sync::Mutex<Option<RenameGate>> = std::sync::Mutex::new(None);
pub(super) fn before_replace(path: &Path) {
    let gate = RENAME_GATE
        .lock()
        .unwrap()
        .take_if(|gate| gate.path == path);
    if let Some(gate) = gate {
        let _ = gate.entered.send(());
        let _ = gate.release.recv_timeout(HANDSHAKE_DEADLINE);
    }
}

#[test]
fn cancelling_begin_or_finish_cannot_detach_a_latest_replacement() {
    for cancel_finish in [false, true] {
        // Arrange a controlled synchronous replacement on its own executor thread.
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-build-cancel-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        sync_fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let repository = root.join("repository");
        sync_fs::create_dir(&repository).unwrap();
        let selection = Selection {
            application: "sample".into(),
            ..Default::default()
        };
        let store = BuildStoreRequest {
            root: root.join("builds").to_str().unwrap().into(),
            key: Some(BuildKey {
                repository_root: repository.to_str().unwrap().into(),
                selection: Some(selection.clone()),
            }),
        };
        let request = BuildAttemptRequest {
            store: Some(store.clone()),
            id: "cancelled".into(),
            source: None,
        };
        let result = BuildResult {
            termination: Some(build_result::Termination::ExitCode(0)),
            cleanup_verified: true,
            record: Some(BuildRecord {
                selection: Some(selection),
                image_id: "sha256:cancelled".into(),
                source: None,
            }),
            ..Default::default()
        };
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (abort_tx, abort_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let mut existing = if cancel_finish {
                    Some(Attempt::begin(request.clone()).await.unwrap())
                } else {
                    None
                };
                let key = request.store.as_ref().unwrap().key.as_ref().unwrap();
                let path = Path::new(&request.store.as_ref().unwrap().root)
                    .join(slot(key))
                    .join(LATEST);
                *RENAME_GATE.lock().unwrap() = Some(RenameGate {
                    path,
                    entered: entered_tx,
                    release: release_rx,
                });
                let task = tokio::spawn(async move {
                    if let Some(attempt) = &mut existing {
                        attempt.finish(result).await.map(|_| ())
                    } else {
                        Attempt::begin(request).await.map(drop)
                    }
                });
                abort_tx.send(task.abort_handle()).unwrap();
                match task.await {
                    Err(error) => assert!(error.is_cancelled()),
                    Ok(_) => panic!("publication task escaped cancellation"),
                }
            });
        });
        let abort = abort_rx.recv_timeout(HANDSHAKE_DEADLINE).unwrap();
        entered_rx.recv_timeout(HANDSHAKE_DEADLINE).unwrap();
        // Act: abort while the shared-name replacement is still unsettled.
        abort.abort();
        // Assert: admission cannot pass to a newer publisher yet.
        assert!(Admission::acquire(&repository).is_err());
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        // Act / Assert: after settlement a new attempt remains the latest.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let newer = Attempt::begin(BuildAttemptRequest {
                store: Some(store.clone()),
                id: "newer".into(),
                source: None,
            })
            .await
            .unwrap();
            drop(newer);
            let latest = lookup(store).await.unwrap().latest.unwrap();
            assert_eq!(latest.id, "newer");
            assert!(latest.result.is_none());
        });
        sync_fs::remove_dir_all(root).unwrap();
    }
}
