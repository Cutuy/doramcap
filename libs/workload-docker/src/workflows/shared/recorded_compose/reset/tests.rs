use super::*;
use crate::compose::adapter::tests::test_engine_status;

// Descriptor inheritance is process-wide; unrelated parallel tests may fork.
fn run_in_separate_test_process() -> bool {
    if std::env::args().any(|arg| arg == "--test-threads=1") {
        return false;
    }
    let thread = std::thread::current();
    let case = thread.name().expect("named libtest thread");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", case, "--test-threads=1", "--nocapture"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed;"),
        "isolated test did not run: {stdout}"
    );
    true
}

#[test]
fn controllers_exclude_the_same_canonical_repository_and_allow_other_repositories() {
    if run_in_separate_test_process() {
        return;
    }
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("repo-a");
    let b = root.path().join("repo-b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let first = Controller::acquire(&a).unwrap();
    let directory = first.directory.clone();

    // Act / Assert: aliases share admission; independent repositories have separate workspaces.
    assert!(Controller::acquire(&a.join(".")).is_err());
    let other = Controller::acquire(&b).unwrap();
    let other_directory = other.directory.clone();
    assert_ne!(first.workspace(), other.workspace());

    // Act / Assert: the same repository can reacquire its unchanged identity after exit.
    drop(first);
    let next = Controller::acquire(&a.join(".")).unwrap();
    assert_eq!(next.directory, directory);
    drop(next);
    drop(other);
    for path in [directory, other_directory] {
        std::fs::remove_file(path.join("controller.lock")).unwrap();
        std::fs::remove_dir(path).unwrap();
    }
}

fn empty_scope() -> Vec<(&'static str, u16, serde_json::Value)> {
    use serde_json::json;
    vec![
        ("/containers/dc-", 404, json!({"message":"absent"})),
        ("/containers/json", 200, json!([])),
        ("/containers/json", 200, json!([])),
        ("/containers/json", 200, json!([])),
        ("/containers/json", 200, json!([])),
        ("/networks", 200, json!([])),
        ("/networks", 200, json!([])),
        ("/containers/json", 200, json!([])),
        ("/containers/json", 200, json!([])),
        ("/networks", 200, json!([])),
    ]
}

#[tokio::test]
async fn reset_is_the_same_operation_for_absent_dirty_partial_and_clean_workspaces() {
    for state in ["absent", "dirty", "partial", "clean"] {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let controller = Rc::new(
            Controller::in_directory(root.path().to_owned(), root.path().join("controller"))
                .unwrap(),
        );
        let workspace = controller.workspace();
        let output = root.path().join("recording.mcap");
        std::fs::write(&output, b"retained output").unwrap();
        // Obsolete history files cannot influence reset.
        std::fs::write(controller.directory.join("current.json"), b"malformed").unwrap();
        if state != "absent" {
            std::fs::create_dir(&workspace).unwrap();
            if state != "clean" {
                for name in ["upper", "work", "merged"] {
                    std::fs::create_dir_all(workspace.join("0").join(name)).unwrap();
                }
                std::fs::write(workspace.join("0/upper/data"), b"unfinished").unwrap();
                if state == "partial" {
                    std::fs::remove_dir(workspace.join("0/work")).unwrap();
                }
            }
        }
        let (adapter, server) =
            test_engine_status(root.path(), [empty_scope(), empty_scope()].concat());
        let mut reset = ResetExperiment {
            adapter: Rc::new(adapter),
            controller,
            stop_seconds: 3,
        };
        // Act / Assert: repeat the identical reset with no saved experiment identity.
        for _ in 0..2 {
            reset.run(()).await.unwrap();
            assert!(!workspace.exists());
            assert_eq!(std::fs::read(&output).unwrap(), b"retained output");
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn engine_inspection_failure_preserves_upper_data() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap(),
    );
    let upper = controller.workspace().join("0/upper");
    std::fs::create_dir_all(&upper).unwrap();
    std::fs::write(upper.join("data"), b"unfinished").unwrap();
    let (adapter, server) = test_engine_status(
        root.path(),
        vec![
            (
                "/containers/dc-",
                404,
                serde_json::json!({"message":"absent"}),
            ),
            (
                "/containers/json",
                200,
                serde_json::json!({"invalid":"container list"}),
            ),
        ],
    );
    let mut reset = ResetExperiment {
        adapter: Rc::new(adapter),
        controller,
        stop_seconds: 3,
    };
    // Act
    let result = reset.run(()).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(std::fs::read(upper.join("data")).unwrap(), b"unfinished");
    server.await.unwrap();
}

#[tokio::test]
async fn a_foreign_reservation_for_this_repository_blocks_reset() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap(),
    );
    let workspace = controller.workspace();
    std::fs::create_dir(&workspace).unwrap();
    let (adapter, server) = test_engine_status(
        root.path(),
        vec![(
            "/containers/dc-",
            200,
            serde_json::json!({"Config":{"Labels":{}}}),
        )],
    );
    let mut reset = ResetExperiment {
        adapter: Rc::new(adapter),
        controller,
        stop_seconds: 3,
    };
    // Act
    let result = reset.run(()).await;
    // Assert
    assert!(result.is_err());
    assert!(workspace.exists());
    server.await.unwrap();
}

#[tokio::test]
async fn an_unverified_writer_blocks_workspace_deletion() {
    use crate::compose::adapter::CONTROLLER_LABEL;
    use serde_json::json;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let controller = Rc::new(
        Controller::in_directory(root.path().to_owned(), root.path().join("controller")).unwrap(),
    );
    let upper = controller.workspace().join("0/upper");
    std::fs::create_dir_all(&upper).unwrap();
    let containers = json!([{"Id":"writer","Labels":{CONTROLLER_LABEL:controller.directory}}]);
    let (adapter, server) = test_engine_status(
        root.path(),
        vec![
            ("/containers/dc-", 404, json!({"message":"absent"})),
            ("/containers/json", 200, containers.clone()),
            ("/containers/writer/stop", 204, json!(null)),
            ("/containers/json", 200, containers),
            (
                "/containers/writer/json",
                200,
                json!({"State":{"Pid":123,"Running":true,"Status":"running"}}),
            ),
        ],
    );
    let mut reset = ResetExperiment {
        adapter: Rc::new(adapter),
        controller,
        stop_seconds: 3,
    };
    // Act
    let result = reset.run(()).await;
    // Assert
    assert!(result.is_err());
    assert!(upper.is_dir());
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
    if run_in_separate_test_process() {
        return;
    }
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
