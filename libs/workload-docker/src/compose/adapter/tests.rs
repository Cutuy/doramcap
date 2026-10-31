use super::configuration::config_arguments;
use super::project::PROJECT_LABEL;
use super::*;
use crate::compose::{
    ComposeProject, ComposeProjectState, ComposeService, Container, Error, ResolveInput,
};
use bollard::Docker;
use serde_json::{Value, json};
use std::time::Duration;

fn discovery_project(directory: &Path) -> ComposeProject {
    ComposeProject {
        key: "workload".into(),
        directory: directory.to_string_lossy().into_owned(),
        files: vec!["compose.yaml".into()],
        env_files: vec![],
        environment: BTreeMap::new(),
        profiles: vec![],
        services: vec![],
        completed_services: vec![],
        remove_volumes: false,
    }
}

#[tokio::test]
async fn compose_arguments_pass_explicit_run_and_project_identity() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    tokio::fs::write(root.path().join("compose.yaml"), b"services: {}")
        .await
        .unwrap();
    let project = discovery_project(root.path());
    for run in ["review-a", "review-b"] {
        // Act
        let name = project_name(run, &project.key);
        let (_, args, _) = config_arguments(&project, &name).await.unwrap();
        // Assert
        let names: Vec<_> = args
            .windows(2)
            .filter(|pair| pair[0] == "--project-name")
            .map(|pair| pair[1].as_str())
            .collect();
        assert_eq!(names, [name.as_str()]);
        assert!(project.environment.is_empty());
    }
}

#[tokio::test]
async fn discovery_and_resolution_reject_invalid_identity_before_io() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("engine.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let adapter = DockerAdapter {
        docker: Docker::connect_with_unix(
            socket.to_str().unwrap(),
            120,
            bollard::API_DEFAULT_VERSION,
        )
        .unwrap(),
        socket: socket.to_string_lossy().into_owned(),
        starting: Default::default(),
    };
    for (run, key) in [("invalid/run", "workload"), ("valid", "invalid/key")] {
        let mut project = discovery_project(Path::new("/nonexistent-project"));
        project.key = key.into();
        // Act
        let profiles = adapter.configuration().profiles(run, &project).await;
        let resolution = adapter
            .configuration()
            .resolve(ResolveInput {
                repository_root: None,
                run: run.into(),
                artifacts: "/nonexistent-artifacts".into(),
                project: Some(project),
            })
            .await;
        // Assert
        for error in [profiles.unwrap_err(), resolution.unwrap_err()] {
            assert!(
                matches!(error, Error::Invalid(ref message) if message.starts_with("identity must"))
            );
        }
    }
}

#[test]
fn digest_preserves_lowercase_sha256_encoding() {
    // Arrange
    let bytes = b"abc";
    // Act
    let actual = digest(bytes);
    // Assert
    assert_eq!(
        actual,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn state() -> ComposeProjectState {
    ComposeProjectState {
        plan: Some(ResolvedComposeProject {
            run: "run".into(),
            key: "project".into(),
            name: "dc-run-project".into(),
            manifest: "/unused".into(),
            sha256: String::new(),
            services: BTreeMap::from([(
                "worker".into(),
                ComposeService {
                    image_id: "image".into(),
                    replicas: 1,
                    completed: false,
                },
            )]),
            diagnostics: vec![],
            remove_volumes: false,
        }),
        containers: vec![Container {
            id: "id".into(),
            service: "worker".into(),
            image_id: "image".into(),
            pid: 1,
            status: "running".into(),
            health: None,
            exit_code: None,
            restart_count: 0,
        }],
    }
}
#[test]
fn health_completion_and_image_identity_are_distinct() {
    // Arrange
    let mut state = state();
    // Act / Assert: running alone is process readiness; health starting is not.
    assert!(state.ready().unwrap());
    state.containers[0].health = Some("starting".into());
    assert!(!state.ready().unwrap());
    state.containers[0].health = Some("unhealthy".into());
    assert!(state.ready().is_err());
    // Act / Assert: successful exit only satisfies a declared completion role.
    state.containers[0].health = None;
    state.containers[0].status = "exited".into();
    state.containers[0].exit_code = Some(0);
    assert!(state.ready().is_err());
    state
        .plan
        .as_mut()
        .unwrap()
        .services
        .get_mut("worker")
        .unwrap()
        .completed = true;
    assert!(state.ready().unwrap());
    // Act / Assert: unexpected image replacement must never count as ready.
    state.containers[0].image_id = "another-image".into();
    assert!(state.ready().is_err());
}
#[test]
fn missing_replicas_never_count_as_ready() {
    // Arrange
    let mut state = state();
    state
        .plan
        .as_mut()
        .unwrap()
        .services
        .get_mut("worker")
        .unwrap()
        .replicas = 2;
    // Act / Assert
    assert!(!state.ready().unwrap());
}

pub(crate) fn test_engine(
    root: &Path,
    responses: Vec<(&'static str, Value)>,
) -> (DockerAdapter, tokio::task::JoinHandle<()>) {
    test_engine_status(
        root,
        responses
            .into_iter()
            .map(|(path, body)| (path, 200, body))
            .collect(),
    )
}

pub(crate) fn test_engine_status(
    root: &Path,
    responses: Vec<(&'static str, u16, Value)>,
) -> (DockerAdapter, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let socket = root.join("engine.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let adapter = DockerAdapter {
        docker: Docker::connect_with_unix(
            socket.to_str().unwrap(),
            1,
            bollard::API_DEFAULT_VERSION,
        )
        .unwrap(),
        socket: socket.to_string_lossy().into_owned(),
        starting: Default::default(),
    };
    let server = tokio::spawn(async move {
        for (path, status, response) in responses {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert!(line.contains(path), "unexpected Engine request: {line}");
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
            }
            let body = if status == 204 {
                String::new()
            } else {
                response.to_string()
            };
            stream.get_mut().write_all(format!(
                "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",body.len()
            ).as_bytes()).await.unwrap();
        }
    });
    (adapter, server)
}

#[tokio::test]
async fn independent_startups_progress_and_duplicate_starts_are_rejected() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    server.await.unwrap();
    let (release, waiting) = tokio::sync::oneshot::channel();
    let plan: ResolvedComposeProject = serde_json::from_value(json!({
        "run":"run","key":"slow","name":"slow","manifest":root.path().join("manifest.json"),
        "sha256":"", "services":{},"diagnostics":[],"remove_volumes":false
    }))
    .unwrap();
    adapter.starting.lock().await.insert(
        plan.name.clone(),
        Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
            waiting.await.unwrap();
            Ok(("slow".into(), String::new()))
        })))),
    );
    adapter.starting.lock().await.insert(
        "fast".into(),
        Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async {
            Ok(("fast".into(), String::new()))
        })))),
    );
    let execution = adapter.project(&plan);
    let mut slow = Box::pin(execution.finish_start());
    assert!(futures_util::poll!(&mut slow).is_pending());

    // Act / Assert: a different project completes while the first remains blocked.
    let fast = tokio::time::timeout(
        Duration::from_secs(1),
        adapter
            .project(&ResolvedComposeProject {
                name: "fast".into(),
                ..Default::default()
            })
            .finish_start(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(fast.unwrap().0, "fast");
    let duplicate = tokio::time::timeout(
        Duration::from_secs(1),
        adapter.project(&plan).command(vec![], 1),
    )
    .await
    .unwrap();
    assert!(
        duplicate
            .unwrap_err()
            .to_string()
            .contains("unfinished startup")
    );

    // Act / Assert: the original startup still completes and releases its entry.
    release.send(()).unwrap();
    assert_eq!(slow.await.unwrap().unwrap().0, "slow");
    assert!(adapter.starting.lock().await.is_empty());
}

#[tokio::test]
async fn cancelled_wait_retains_startup_and_concurrent_waiters_consume_it_once() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    server.await.unwrap();
    let (release, waiting) = tokio::sync::oneshot::channel();
    adapter.starting.lock().await.insert(
        "run".into(),
        Arc::new(tokio::sync::Mutex::new(Some(tokio::spawn(async move {
            waiting.await.unwrap();
            Ok(("finished".into(), String::new()))
        })))),
    );
    let plan = ResolvedComposeProject {
        name: "run".into(),
        ..Default::default()
    };
    let execution = adapter.project(&plan);
    let mut cancelled = Box::pin(execution.finish_start());
    assert!(futures_util::poll!(&mut cancelled).is_pending());

    // Act / Assert: cancellation leaves an unfinished command available for cleanup.
    drop(cancelled);
    let mut first = Box::pin(execution.finish_start());
    let mut second = Box::pin(execution.finish_start());
    assert!(futures_util::poll!(&mut first).is_pending());
    assert!(futures_util::poll!(&mut second).is_pending());
    release.send(()).unwrap();
    let (first, second) = tokio::join!(first, second);
    let outputs: Vec<_> = [first.unwrap(), second.unwrap()]
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(outputs, [("finished".into(), String::new())]);
    assert!(adapter.starting.lock().await.is_empty());
}

#[tokio::test]
async fn exec_rejects_a_replaced_image_before_launching_a_command() {
    // Arrange: Engine reports an owned, running replica with a different image.
    let root = tempfile::tempdir().unwrap();
    let manifest = root.path().join("manifest.json");
    std::fs::write(&manifest, b"{}").unwrap();
    let plan: ResolvedComposeProject = serde_json::from_value(json!({
            "run":"run","key":"app","name":"project","manifest":manifest,
            "sha256":digest(b"{}"),"services":{"worker":{"image_id":"pinned","replicas":1,"completed":false}},
            "diagnostics":[],"remove_volumes":false
        })).unwrap();
    let labels =
        json!({RUN_LABEL:"run",PROJECT_LABEL:plan.name,"com.docker.compose.service":"worker"});
    let (adapter, server) = test_engine(
        root.path(),
        vec![
            (
                "/containers/json",
                json!([{"Id":"container","Labels":labels}]),
            ),
            (
                "/containers/container/json",
                json!({"Id":"container","Image":"replacement",
                "Config":{"Labels":labels},"State":{"Pid":123,"Status":"running","ExitCode":0},"RestartCount":0}),
            ),
        ],
    );

    // Act
    let error = adapter
        .project(&plan)
        .exec("worker", &["should-not-run".into()], Duration::from_secs(1))
        .await
        .unwrap_err();

    // Assert
    assert!(matches!(error, Error::Invalid(message) if message.contains("image changed")));
    server.await.unwrap();
}
