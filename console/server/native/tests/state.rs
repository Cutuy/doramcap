use console_native::{
    protocol::{Marker, Outcome, Phase},
    state::{Restored, StateStore},
};

#[test]
fn lock_is_exclusive_and_unfinished_marker_blocks_restart() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (store, restored) = StateStore::open(root.path()).unwrap();
    assert!(matches!(restored, Restored::Idle));
    // Act / Assert
    assert!(StateStore::open(root.path()).is_err());
    store
        .write(&Marker {
            version: 1,
            phase: Phase::Starting as i32,
            run_id: "run".into(),
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let (_store, restored) = StateStore::open(root.path()).unwrap();
    assert!(matches!(restored, Restored::Blocked(_)));
}

#[test]
fn restores_only_proven_clean_completion_and_blocks_corruption() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (store, _) = StateStore::open(root.path()).unwrap();
    let marker = Marker {
        version: 1,
        phase: Phase::Finished as i32,
        outcome: Some(Outcome {
            boundary_clean: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    // Act
    store.write(&marker).unwrap();
    drop(store);
    // Assert
    let (store, restored) = StateStore::open(root.path()).unwrap();
    assert!(matches!(restored, Restored::Finished(_)));
    drop(store);
    std::fs::write(root.path().join("current.pb"), [0xff]).unwrap();
    let (_store, restored) = StateStore::open(root.path()).unwrap();
    assert!(matches!(restored, Restored::Blocked(_)));
}

#[test]
fn crash_before_rename_keeps_prior_marker_and_after_rename_blocks() {
    use std::os::unix::fs::PermissionsExt;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (store, _) = StateStore::open(root.path()).unwrap();
    let clean = Marker {
        version: 1,
        phase: Phase::Finished as i32,
        run_id: "old".into(),
        outcome: Some(Outcome {
            boundary_clean: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    store.write(&clean).unwrap();
    // Act: simulate loss before the replacement marker is renamed.
    std::fs::write(root.path().join(".current-interrupted"), b"incomplete").unwrap();
    drop(store);
    let (store, restored) = StateStore::open(root.path()).unwrap();
    // Assert
    assert!(matches!(restored, Restored::Finished(marker) if marker.run_id == "old"));
    assert_eq!(
        std::fs::metadata(root.path().join("current.pb"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    // Act: persist Starting, then simulate loss before dispatch or completion.
    store
        .write(&Marker {
            version: 1,
            phase: Phase::Starting as i32,
            run_id: "new".into(),
            ..Default::default()
        })
        .unwrap();
    drop(store);
    let (_store, restored) = StateStore::open(root.path()).unwrap();
    // Assert
    assert!(matches!(restored, Restored::Blocked(Some(marker)) if marker.run_id == "new"));
}
