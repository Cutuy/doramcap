use super::project::PROJECT_LABEL;
use super::*;
use crate::compose::{
    ComposeProject, ComposeProjectState, ComposeService, Container, Error, ResolveInput,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn input_project(directory: &Path) -> ComposeProject {
    ComposeProject {
        key: "workload".into(),
        directory: directory.to_str().unwrap().into(),
        files: vec!["compose.yaml".into()],
        env_files: vec![],
        environment: BTreeMap::new(),
        profiles: vec![],
        services: vec![],
        completed_services: vec![],
        remove_volumes: false,
        ..Default::default()
    }
}

#[tokio::test]
async fn compose_arguments_pass_explicit_project_identity() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    tokio::fs::write(root.path().join("compose.yaml"), b"services: {}")
        .await
        .unwrap();
    let project = input_project(root.path());
    // Act
    let name = project_name(&project.key, root.path());
    let (_, args, _) = super::configuration::config_arguments(&project, &name)
        .await
        .unwrap();
    // Assert
    let names: Vec<_> = args
        .windows(2)
        .filter(|pair| pair[0] == "--project-name")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(names, [name.as_str()]);
    assert!(project.environment.is_empty());
}

#[tokio::test]
async fn resolution_rejects_invalid_identity_before_io() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("engine.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let adapter = DockerAdapter {
        engine: Engine::new(socket.to_str().unwrap(), 120).unwrap(),
        starting: Default::default(),
    };
    for (run, key) in [("invalid/run", "workload"), ("valid", "invalid/key")] {
        let mut project = input_project(Path::new("/nonexistent-project"));
        project.key = key.into();
        // Act
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
        let error = resolution.unwrap_err();
        assert!(
            matches!(error, Error::Invalid(ref message) if message.starts_with("identity must"))
        );
    }
}

#[tokio::test]
async fn scoped_resolution_rejects_noncanonical_or_nondirectory_roots_before_docker() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    let link = root.path().join("linked-repository");
    std::fs::write(&file, b"not a repository").unwrap();
    std::os::unix::fs::symlink(root.path(), &link).unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    for repository in [file, link, std::path::PathBuf::from(".")] {
        // Act
        let result = adapter
            .configuration()
            .resolve(ResolveInput {
                run: "run".into(),
                artifacts: root.path().to_str().unwrap().into(),
                project: Some(input_project(root.path())),
                repository_root: Some(repository.to_str().unwrap().into()),
            })
            .await;

        // Assert
        assert!(matches!(result, Err(Error::Invalid(message))
            if message.contains("repository root must be an existing canonical absolute directory")));
    }
    server.await.unwrap();
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

#[tokio::test]
async fn absent_protobuf_plans_fail_before_engine_access() {
    use crate::compose::{
        QuiesceCompose, RemoveCompose, StartCompose, StartInput, StopCompose, StopInput,
    };
    use orchestrator::Step;
    use std::{cell::RefCell, rc::Rc};

    // Arrange: no Engine is available, and protobuf requests omit their required plan.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    server.await.unwrap();
    let adapter = Rc::new(adapter);
    let diagnostics = Rc::new(RefCell::new(Vec::new()));

    // Act
    let errors = [
        StartCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(StartInput::default())
        .await
        .err(),
        StopCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(StopInput::default())
        .await
        .err(),
        QuiesceCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(StopInput::default())
        .await
        .err(),
        RemoveCompose {
            adapter,
            diagnostics: diagnostics.clone(),
        }
        .run(StopInput::default())
        .await
        .err(),
    ];

    // Assert: invalid requests retain typed diagnostics without attempting Docker operations.
    assert!(errors.iter().all(Option::is_some));
    let diagnostics = diagnostics.borrow();
    assert_eq!(diagnostics.len(), 4);
    assert!(diagnostics.iter().all(
        |diagnostic| diagnostic.kind == "validation" && diagnostic.message.contains("missing")
    ));
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
            ..Default::default()
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
fn an_unexpected_exit_reports_its_status_and_last_output() {
    // Arrange
    let mut state = state();
    state.containers[0].status = "exited".into();
    state.containers[0].exit_code = Some(1);
    state.containers[0].last_output = "Error: usage: worker --config FILE".into();
    // Act
    let message = state.ready().unwrap_err().to_string();
    // Assert
    assert!(
        message.contains("worker exited unexpectedly with status 1"),
        "{message}"
    );
    assert!(message.contains("usage: worker --config FILE"), "{message}");
}
#[test]
fn appended_output_keeps_the_newest_readable_text() {
    // Arrange: more output than the guard keeps, with the cause in the last chunk.
    use crate::compose::state::{MAX_OUTPUT_CHARS, append_output};
    let mut output = String::new();
    let chunk = "h".repeat(MAX_OUTPUT_CHARS);
    // Act
    append_output(&mut output, chunk.as_bytes());
    append_output(&mut output, b"\x1b[31mfinal error\x1b[0m\n");
    // Assert
    assert_eq!(output.chars().count(), MAX_OUTPUT_CHARS);
    assert!(output.ends_with("final error[0m\n"));
    assert!(!output.contains('\u{1b}'));
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
        engine: Engine::new(socket.to_str().unwrap(), 1).unwrap(),
        starting: Default::default(),
    };
    let server = tokio::spawn(async move {
        let mut responses = responses.into_iter().peekable();
        while let Some((path, status, response)) = responses.peek() {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            let negotiation = line.starts_with("HEAD /_ping ");
            assert!(
                negotiation || line.contains(*path),
                "unexpected Engine request: {line}"
            );
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
            }
            // Compose may negotiate before rendering; this is independent of Engine facts.
            if negotiation {
                stream.get_mut().write_all(format!(
                    "HTTP/1.1 200 OK\r\nAPI-Version: {}\r\nOSType: linux\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    bollard::API_DEFAULT_VERSION
                ).as_bytes()).await.unwrap();
                continue;
            }
            let body = if *status == 204 {
                String::new()
            } else {
                response.to_string()
            };
            stream.get_mut().write_all(format!(
                "HTTP/1.1 {status} Response\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",body.len()
            ).as_bytes()).await.unwrap();
            let _ = responses.next();
        }
    });
    (adapter, server)
}

#[tokio::test]
async fn engine_fixture_negotiation_preserves_expected_requests() {
    use tokio::io::AsyncReadExt;

    // Arrange: the image response must remain available after Compose's API probe.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(
        root.path(),
        vec![("/images/example:local/json", json!({"Id":"sha256:pinned"}))],
    );
    let mut probe = tokio::net::UnixStream::connect(root.path().join("engine.sock"))
        .await
        .unwrap();

    // Act: reproduce the negotiation request made by Compose on hosted runners.
    probe
        .write_all(b"HEAD /_ping HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = String::new();
    probe.read_to_string(&mut response).await.unwrap();

    // Assert: negotiation succeeds without consuming the expected image request.
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains(&format!("API-Version: {}", bollard::API_DEFAULT_VERSION)));
    let image = adapter
        .engine
        .client()
        .inspect_image("example:local")
        .await
        .unwrap();
    assert_eq!(image.id.as_deref(), Some("sha256:pinned"));
    server.await.unwrap();
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
        Arc::new(tokio::sync::Mutex::new(Some(StartupOutcome::Running(
            tokio::spawn(async move {
                waiting.await.unwrap();
                Ok(("slow".into(), String::new()))
            }),
        )))),
    );
    adapter.starting.lock().await.insert(
        "fast".into(),
        Arc::new(tokio::sync::Mutex::new(Some(StartupOutcome::Running(
            tokio::spawn(async { Ok(("fast".into(), String::new())) }),
        )))),
    );
    let slow_project = adapter.project(&plan);
    let mut slow = Box::pin(slow_project.finish_start());
    assert!(futures_util::poll!(&mut slow).is_pending());

    // Act / Assert: a different project completes while the first remains blocked.
    let fast_plan = ResolvedComposeProject {
        name: "fast".into(),
        ..plan.clone()
    };
    let fast_project = adapter.project(&fast_plan);
    let fast = tokio::time::timeout(Duration::from_secs(1), fast_project.finish_start())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fast.unwrap().0, "fast");
    let duplicate = tokio::time::timeout(Duration::from_secs(1), slow_project.command(vec![], 1))
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
        Arc::new(tokio::sync::Mutex::new(Some(StartupOutcome::Running(
            tokio::spawn(async move {
                waiting.await.unwrap();
                Ok(("finished".into(), String::new()))
            }),
        )))),
    );
    let plan = ResolvedComposeProject {
        name: "run".into(),
        ..state().plan.unwrap()
    };
    let project = adapter.project(&plan);
    let mut cancelled = Box::pin(project.finish_start());
    assert!(futures_util::poll!(&mut cancelled).is_pending());

    // Act / Assert: cancellation leaves an unfinished command available for cleanup.
    drop(cancelled);
    let mut first = Box::pin(project.finish_start());
    let mut second = Box::pin(project.finish_start());
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
async fn cancelled_completed_startup_retains_outcome_until_entry_removal() {
    for failed in [false, true] {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let (adapter, server) = test_engine(root.path(), vec![]);
        server.await.unwrap();
        let (release, waiting) = tokio::sync::oneshot::channel();
        let (finished, completion) = tokio::sync::oneshot::channel();
        adapter.starting.lock().await.insert(
            "run".into(),
            Arc::new(tokio::sync::Mutex::new(Some(StartupOutcome::Running(
                tokio::spawn(async move {
                    waiting.await.unwrap();
                    finished.send(()).unwrap();
                    if failed {
                        Err(super::invalid("startup failed"))
                    } else {
                        Ok(("finished".into(), String::new()))
                    }
                }),
            )))),
        );
        let plan = ResolvedComposeProject {
            name: "run".into(),
            ..state().plan.unwrap()
        };
        let project = adapter.project(&plan);
        let mut cancelled = Box::pin(project.finish_start());
        assert!(futures_util::poll!(&mut cancelled).is_pending());
        let entries = adapter.starting.lock().await;

        // Act: complete the task, then cancel its waiter while map removal is blocked.
        release.send(()).unwrap();
        completion.await.unwrap();
        assert!(futures_util::poll!(&mut cancelled).is_pending());
        drop(cancelled);
        drop(entries);
        let result = project.finish_start().await;

        // Assert: cleanup receives the original outcome and removes the startup barrier.
        if failed {
            assert_eq!(result.unwrap_err().to_string(), "startup failed");
        } else {
            assert_eq!(result.unwrap(), Some(("finished".into(), String::new())));
        }
        assert!(adapter.starting.lock().await.is_empty());
        assert!(project.finish_start().await.unwrap().is_none());
    }
}

#[tokio::test]
async fn endpoint_rejects_a_replaced_image_before_routing_control() {
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
        .endpoint("worker", "host", 6013)
        .await
        .unwrap_err();

    // Assert
    assert!(matches!(error, Error::Invalid(message) if message.contains("image changed")));
    server.await.unwrap();
}
#[tokio::test]
async fn whole_project_teardown_rejects_selected_services_before_engine_access() {
    use crate::compose::{RemoveCompose, StopCompose, StopInput};
    use orchestrator::Step;
    use std::{cell::RefCell, rc::Rc};

    // Arrange: the Engine listener is closed, so any accidental access fails.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    server.await.unwrap();
    let adapter = Rc::new(adapter);
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let input = StopInput {
        plan: state().plan,
        services: vec!["worker".into()],
        allow_volume_deletion: true,
    };

    // Act
    let results = [
        StopCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(input.clone())
        .await,
        RemoveCompose {
            adapter,
            diagnostics: diagnostics.clone(),
        }
        .run(input)
        .await,
    ];

    // Assert: both public steps reject selection and preserve validation diagnostics.
    assert!(results.iter().all(|result| result.is_err()));
    let diagnostics = diagnostics.borrow();
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.kind == "validation")
    );
}

#[tokio::test]
async fn dead_container_with_no_process_is_quiescent() {
    // Arrange: Docker can retain a dead container after interrupted teardown.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(
        root.path(),
        vec![
            (
                "/containers/json",
                json!([{"Id":"dead-writer","Labels":{CONTROLLER_LABEL:"scope"}}]),
            ),
            (
                "/containers/dead-writer/json",
                json!({"State":{"Pid":0,"Running":false,"Status":"dead"}}),
            ),
        ],
    );
    // Act
    let result = adapter
        .engine
        .resources("scope", "owner")
        .verify_stopped()
        .await;
    // Assert: process absence allows reset to continue, regardless of the failure status.
    assert!(result.is_ok());
    server.await.unwrap();
}

#[tokio::test]
async fn common_pbtxt_controls_process_files_and_configuration_identity() {
    // Arrange: real YAML stays native; literal process arguments and files live in pbtxt.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = test_engine(root.path(), vec![]);
    std::fs::write(
        root.path().join("compose.yaml"),
        "services:\n  capture:\n    image: example:local\n    network_mode: host\n",
    )
    .unwrap();
    std::fs::write(root.path().join("settings.pbtxt"), "rate: 5").unwrap();
    let manifest = r#"id: "capture" compose { files: "compose.yaml" service: "capture" }
process { argv: "capture" argv: "$LITERAL" working_directory: "/results" memlock_bytes: 8388608 cpus: "0" stop_signal: 2 stop_seconds: 17 }
files { source: "settings.pbtxt" target: "/config/settings.pbtxt" }"#;
    std::fs::write(root.path().join("capture.pbtxt"), manifest).unwrap();
    let mut input = input_project(root.path());
    input.manifest = "capture.pbtxt".into();
    // Act
    let first = adapter
        .configuration()
        .normalize("run", &input, "experiment")
        .await
        .unwrap();
    // Assert
    let service = &first.model["services"]["capture"];
    assert_eq!(service["entrypoint"], json!(["capture"]));
    // Compose preserves dollar escaping for the frozen file consumed at launch.
    assert_eq!(service["command"], json!(["$$LITERAL"]));
    assert_eq!(service["working_dir"], "/results");
    assert_eq!(service["ulimits"]["memlock"]["hard"], 8388608);
    assert_eq!(service["stop_signal"], "2");
    assert_eq!(service["stop_grace_period"], "17s");
    assert_eq!(service["network_mode"], "host");
    assert_eq!(service["volumes"][0]["read_only"], true);
    // Act: editing common config must change both the launch and its input identity.
    std::fs::write(
        root.path().join("capture.pbtxt"),
        manifest.replace("$LITERAL", "changed"),
    )
    .unwrap();
    let second = adapter
        .configuration()
        .normalize("run", &input, "experiment")
        .await
        .unwrap();
    // Assert
    assert_eq!(
        second.model["services"]["capture"]["command"],
        json!(["changed"])
    );
    assert_ne!(first.input_hashes, second.input_hashes);
    server.await.unwrap();
}

#[tokio::test]
async fn frozen_logging_defaults_to_journald_and_preserves_project_settings() {
    for logging in [
        None,
        Some(json!({"driver":"local", "options":{"max-size":"5m"}})),
        Some(json!({"driver":"none"})),
        Some(json!({"driver":"journald", "options":{"tag":"project-tag"}})),
        Some(json!({"options":{"max-size":"10m"}})),
        Some(json!({})),
    ] {
        // Arrange: Compose renders a real source file; Engine supplies only image/resource facts.
        let root = tempfile::tempdir().unwrap();
        let mut service = json!({"image":"example:local", "network_mode":"none"});
        if let Some(logging) = &logging {
            service["logging"] = logging.clone();
        }
        let source = serde_json::to_vec(&json!({"services":{"worker":service}})).unwrap();
        let path = root.path().join("compose.yaml");
        std::fs::write(&path, &source).unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![
                ("/images/example:local/json", json!({"Id":"sha256:pinned"})),
                ("/containers/json", json!([])),
                ("/networks", json!([])),
                ("/volumes", json!({"Volumes":[]})),
            ],
        );

        // Act: freeze the same manifest used by inspection and launch.
        let plan = adapter
            .configuration()
            .resolve(ResolveInput {
                run: "run".into(),
                artifacts: root.path().to_str().unwrap().into(),
                project: Some(input_project(root.path())),
                repository_root: None,
            })
            .await
            .unwrap();
        let bytes = std::fs::read(&plan.manifest).unwrap();
        let model: Value = serde_json::from_slice(&bytes).unwrap();
        let resolution: Value = serde_json::from_slice(
            &std::fs::read(root.path().join("workload.resolution.json")).unwrap(),
        )
        .unwrap();
        let changes: Vec<_> = resolution["policy_changes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|change| change["field"] == "logging")
            .collect();

        // Assert: native logging survives verbatim and only the absent setting gets a default.
        assert_eq!(
            model["services"]["worker"]["logging"],
            logging
                .clone()
                .unwrap_or_else(|| json!({"driver":"journald"}))
        );
        assert_eq!(std::fs::read(path).unwrap(), source);
        assert_eq!(plan.sha256, digest(&bytes));
        if logging.is_some() {
            assert!(changes.is_empty());
            assert!(plan.diagnostics.iter().any(|message| message
                == "worker: logging configured by project; Console will not collect its logs"));
        } else {
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0]["original"], Value::Null);
            assert_eq!(changes[0]["effective"], json!({"driver":"journald"}));
        }
        server.await.unwrap();
    }
}
