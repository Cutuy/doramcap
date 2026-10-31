use super::*;
use crate::compose::adapter::tests::test_engine;

fn plan(directory: &Path, project: &str) -> ResolvedComposeProject {
    ResolvedComposeProject {
        run: "previous".into(),
        key: "experiment".into(),
        name: project.into(),
        manifest: directory.join("experiment.compose.json"),
        sha256: String::new(),
        services: Default::default(),
        diagnostics: vec![],
        remove_volumes: false,
    }
}

#[test]
fn one_controller_excludes_different_repositories_and_releases_on_drop() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("controller");
    let first = Controller::in_directory(root.path().join("repo-a"), directory.clone()).unwrap();
    // Act / Assert
    assert!(Controller::in_directory(root.path().join("repo-b"), directory.clone()).is_err());
    drop(first);
    // Act / Assert
    assert!(Controller::in_directory(root.path().join("repo-b"), directory).is_ok());
}

#[tokio::test]
async fn published_identity_survives_a_partial_new_record_and_clear_is_repeatable() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller =
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap();
    let expected = plan(root.path(), &controller.project());
    controller.save(&expected).await.unwrap();
    // Act: model a launcher exiting before its next atomic publication.
    std::fs::write(controller.directory.join("pending.json"), b"partial").unwrap();
    // Assert
    assert_eq!(
        controller.previous().await.unwrap().unwrap().name,
        expected.name
    );
    // Act / Assert
    controller.clear().await.unwrap();
    controller.clear().await.unwrap();
    assert!(controller.previous().await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_previous_identity_blocks_reset_and_preserves_upper_data() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap(),
    );
    std::fs::write(controller.directory.join("current.json"), b"invalid").unwrap();
    let upper = root.path().join("upper");
    std::fs::write(&upper, b"untouched").unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    let mut reset = ResetPreviousProject {
        adapter: Rc::new(adapter),
        controller,
        stop_seconds: 3,
    };
    // Act
    let result = reset.run(()).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(std::fs::read(upper).unwrap(), b"untouched");
    server.await.unwrap();
}

#[test]
fn repository_projects_are_distinct() {
    // Arrange
    let first = Path::new("/repositories/first");
    let second = Path::new("/repositories/second");
    // Act / Assert
    assert_ne!(
        project_name("experiment", first),
        project_name("experiment", second)
    );
}

#[tokio::test]
async fn engine_inspection_failure_preserves_previous_state_and_upper_data() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap(),
    );
    let previous = plan(root.path(), &controller.project());
    controller.save(&previous).await.unwrap();
    let upper = root.path().join("overlays/0/upper");
    std::fs::create_dir_all(&upper).unwrap();
    std::fs::write(upper.join("data"), b"unfinished").unwrap();
    let (adapter, server) = test_engine(
        root.path(),
        vec![
            (
                "/containers/doramcap-experiment/json",
                serde_json::json!({"Config":{"Labels":{"com.docker.compose.project":previous.name}}}),
            ),
            (
                "/containers/json",
                serde_json::json!({"invalid":"container list"}),
            ),
        ],
    );
    let mut reset = ResetPreviousProject {
        adapter: Rc::new(adapter),
        controller: controller.clone(),
        stop_seconds: 3,
    };
    // Act
    let result = reset.run(()).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(std::fs::read(upper.join("data")).unwrap(), b"unfinished");
    assert!(controller.previous().await.unwrap().is_some());
    server.await.unwrap();
}

#[tokio::test]
async fn discard_handles_dirty_clean_and_skipped_teardown_without_changing_host_files() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let overlays = root.path().join("overlays");
    let source = root.path().join("source");
    std::fs::write(&source, b"partially copied back").unwrap();
    for tree in ["0", "1"] {
        for name in ["upper", "work", "merged", "staged"] {
            std::fs::create_dir_all(overlays.join(tree).join(name)).unwrap();
        }
    }
    std::fs::write(overlays.join("0/upper/data"), b"unfinished").unwrap();
    // Act: reset a partially removed tree, as after an interrupted cleanup.
    std::fs::remove_dir(overlays.join("1/work")).unwrap();
    overlay::discard(&overlays).await.unwrap();
    // Assert
    assert!(!overlays.join("0/upper").exists());
    assert!(!overlays.join("1/upper").exists());
    assert_eq!(std::fs::read(&source).unwrap(), b"partially copied back");
    // Act / Assert: clean and entirely absent previous state are already done.
    overlay::discard(&overlays).await.unwrap();
    overlay::discard(&root.path().join("absent")).await.unwrap();
}

#[tokio::test]
async fn discard_rejects_unknown_directories_before_deleting_any_upper() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let overlays = root.path().join("overlays");
    std::fs::create_dir_all(overlays.join("0/upper")).unwrap();
    std::fs::create_dir_all(overlays.join("foreign")).unwrap();
    std::fs::write(overlays.join("0/upper/data"), b"keep").unwrap();
    // Act
    let result = overlay::discard(&overlays).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(overlays.join("0/upper/data")).unwrap(),
        b"keep"
    );
}

#[test]
fn in_flight_child_keeps_admission_after_controller_is_dropped() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("controller");
    let controller = Controller::in_directory(root.path().to_owned(), directory.clone()).unwrap();
    let mut child = std::process::Command::new("/bin/cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // Act / Assert: the original launcher is gone, but its child can still make changes.
    drop(controller);
    assert!(Controller::in_directory(root.path().to_owned(), directory.clone()).is_err());
    // Act / Assert: admission releases when the child exits.
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    assert!(Controller::in_directory(root.path().to_owned(), directory).is_ok());
}

#[tokio::test]
async fn discard_rejects_linked_upper_without_deleting_the_link_target() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let outside = root.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("data"), b"keep").unwrap();
    let overlays = root.path().join("overlays");
    std::fs::create_dir_all(overlays.join("0")).unwrap();
    std::os::unix::fs::symlink(&outside, overlays.join("0/upper")).unwrap();
    // Act
    let result = overlay::discard(&overlays).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(std::fs::read(outside.join("data")).unwrap(), b"keep");
}
