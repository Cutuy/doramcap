//! Native Compose qualification before migrating the recorded workflow.
use orchestrator::Step;
use prost::Message;
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::Path, rc::Rc, time::Duration};
use workload_docker::compose::*;

async fn cli(executable: &str, port: u16, args: &[&str]) -> std::result::Result<String, String> {
    let output = tokio::time::timeout(
        Duration::from_secs(20),
        tokio::process::Command::new(executable)
            .args(args)
            .args(["--coordinator-port", &port.to_string()])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn wait_file(path: &Path) -> std::result::Result<(), String> {
    tokio::time::timeout(Duration::from_secs(20), async {
        while !path.is_file() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|e| format!("{}: {e}", path.display()))
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker, Dora 1.1.0 and the attachment fixture; run as root"]
async fn one_project_qualifies_zenoh_shm_and_adopted_ipc() {
    // Arrange
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let executable = std::env::var("DORAMCAP_TEST_DORA_CLI").unwrap();
    let fixture = std::env::var("DORAMCAP_TEST_DORA_NODE").unwrap();
    let root = std::env::temp_dir().join(format!("shared-ipc-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let listeners: Vec<_> = (0..2)
        .map(|_| std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap())
        .collect();
    let coordinator = listeners[0].local_addr().unwrap().port();
    let daemon = listeners[1].local_addr().unwrap().port();
    let overlay = root.join("zenoh.json5");
    std::fs::write(&overlay, "{transport:{shared_memory:{enabled:true}}}\n").unwrap();
    let graph = root.join("graph.yml");
    std::fs::write(&graph, "nodes:\n  - id: source\n    path: dynamic\n    outputs: [data]\n  - id: recorder\n    path: dynamic\n    inputs:\n      data: {source: source/data, queue_size: 64, queue_policy: drop_oldest}\n").unwrap();
    let payload = telemetry_protocol::pb::Packet {
        body: Some(telemetry_protocol::pb::packet::Body::Snapshot(
            telemetry_protocol::pb::SnapshotFrame {
                version: telemetry_protocol::VERSION,
                source: "x".repeat(256 * 1024),
                session_id: "shared-ipc".into(),
                snapshot: Some(telemetry_protocol::pb::Snapshot {
                    schema_version: 1,
                    sequence: 1,
                    capture_unix_ns: 456,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )),
    }
    .encode_to_vec();
    std::fs::write(root.join("payload.pb"), &payload).unwrap();
    let base = serde_json::json!({
        "image": image, "network_mode":"host", "ipc":"service:daemon", "stop_signal":"SIGINT",
        "environment":{"HOME":root, "DORA_ZENOH_CONFIG_OVERLAY":overlay, "DORA_DAEMON_LOCAL_LISTEN_PORT":daemon.to_string(), "DORAMCAP_REQUIRE_SHM":"1"},
        "ulimits":{"memlock":{"soft":67108864,"hard":67108864}},
        "volumes":[
            {"type":"bind","source":root,"target":root},
            {"type":"bind","source":fixture,"target":"/fixture","read_only":true},
            {"type":"bind","source":executable,"target":"/dora","read_only":true}
        ]
    });
    let mut services = serde_json::Map::new();
    for (name, args) in [
        (
            "coordinator",
            vec![
                "coordinator".into(),
                "--interface".into(),
                "127.0.0.1".into(),
                "--port".into(),
                coordinator.to_string(),
                "--store".into(),
                "memory".into(),
            ],
        ),
        (
            "daemon",
            vec![
                "daemon".into(),
                "--coordinator-port".into(),
                coordinator.to_string(),
                "--local-listen-port".into(),
                daemon.to_string(),
                "--worker-threads".into(),
                "2".into(),
                "--zenoh-config-overlay".into(),
                overlay.to_str().unwrap().into(),
            ],
        ),
    ] {
        let mut service = base.clone();
        service["entrypoint"] = serde_json::json!(["/dora"]);
        service["command"] = serde_json::json!(args);
        service["working_dir"] = serde_json::json!(root);
        if name == "daemon" {
            service["ipc"] = "shareable".into();
            service["shm_size"] = (128 * 1024 * 1024).into();
        }
        services.insert(name.into(), service);
    }
    for (name, mode, path, done) in [
        (
            "source",
            "producer",
            root.join("payload.pb"),
            root.join("source.done"),
        ),
        (
            "recorder",
            "recorder",
            root.join("recording.mcap"),
            root.join("shm.proof"),
        ),
    ] {
        let mut service = base.clone();
        service["entrypoint"] = serde_json::json!(["/fixture"]);
        service["command"] = serde_json::json!([
            name,
            &daemon.to_string(),
            root.join(format!("{name}.ready")).to_str().unwrap(),
            mode,
            path.to_str().unwrap(),
            done.to_str().unwrap()
        ]);
        service["healthcheck"] = serde_json::json!({"test":["CMD", "test", "-s", root.join(format!("{name}.ready"))],"interval":"100ms","timeout":"1s","retries":100});
        if name == "source" {
            service["depends_on"] = serde_json::json!({"recorder":{"condition":"service_healthy"}});
        }
        services.insert(name.into(), service);
    }
    let mut adopted = base;
    adopted["entrypoint"] = serde_json::json!(["/bin/sh", "-c"]);
    adopted["command"] = serde_json::json!([format!(
        "test -s {}/source.ready && test -s {}/recorder.ready && test -s /dev/shm/first-party-segment || exit 42; cat /dev/shm/first-party-segment > {}/adopted.proof; trap 'exit 0' TERM INT; while :; do sleep 1 & wait $$!; done",
        root.display(),
        root.display(),
        root.display()
    )]);
    services.insert("adopted".into(), adopted);
    std::fs::write(
        root.join("compose.json"),
        serde_json::to_vec(&serde_json::json!({"services":services})).unwrap(),
    )
    .unwrap();
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let plan = ResolveCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(ResolveInput {
        repository_root: None,
        run: format!("shm-{}", std::process::id()),
        artifacts: root.to_string_lossy().into_owned(),
        project: Some(ComposeProject {
            key: "experiment".into(),
            directory: root.to_string_lossy().into_owned(),
            files: vec!["compose.json".into()],
            env_files: vec![],
            environment: BTreeMap::new(),
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: true,
        }),
    })
    .await
    .unwrap();
    let mut executor = orchestrator::LocalExecutor::default();
    let backend = adapter.clone();
    executor.register::<StartCompose>(move || StartCompose {
        adapter: backend.clone(),
        diagnostics: Default::default(),
    });
    drop(listeners);

    // Act: infrastructure precedes first-party apps, then an adopted plain Compose app.
    let result: std::result::Result<(), String> = async {
        executor
            .run::<StartCompose>(StartInput {
                plan: Some(plan.clone()),
                services: vec!["daemon".into(), "coordinator".into()],
                readiness_seconds: 15,
            })
            .await
            .map_err(|e| e.to_string())?;
        tokio::time::timeout(Duration::from_secs(15), async {
            for (service, port) in [("coordinator", coordinator), ("daemon", daemon)] {
                while adapter
                    .project(&plan)
                    .endpoint(service, "host", port)
                    .await
                    .is_err()
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        })
        .await
        .map_err(|e| e.to_string())?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            match cli(
                &executable,
                coordinator,
                &[
                    "start",
                    graph.to_str().unwrap(),
                    "--name",
                    "qualification",
                    "--detach",
                ],
            )
            .await
            {
                Ok(_) => break,
                Err(error)
                    if error.contains("no unnamed daemon connections")
                        && tokio::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                Err(error) => return Err(error),
            }
        }
        executor
            .run::<StartCompose>(StartInput {
                plan: Some(plan.clone()),
                services: vec!["recorder".into(), "source".into()],
                readiness_seconds: 20,
            })
            .await
            .map_err(|e| e.to_string())?;
        wait_file(&root.join("source.done")).await?;
        let state = adapter
            .project(&plan)
            .inspect()
            .await
            .map_err(|e| e.to_string())?;
        let daemon_pid = state
            .containers
            .iter()
            .find(|c| c.service == "daemon")
            .unwrap()
            .pid;
        // The source writes this segment through its container's shared mount.
        let source_pid = state
            .containers
            .iter()
            .find(|c| c.service == "source")
            .unwrap()
            .pid;
        std::fs::write(
            format!("/proc/{source_pid}/root/dev/shm/first-party-segment"),
            b"shared\n",
        )
        .map_err(|e| e.to_string())?;
        executor
            .run::<StartCompose>(StartInput {
                plan: Some(plan.clone()),
                services: vec!["adopted".into()],
                readiness_seconds: 10,
            })
            .await
            .map_err(|e| e.to_string())?;
        wait_file(&root.join("adopted.proof")).await?;

        // Assert: every container joins the same namespace and actual shm filesystem.
        assert_eq!(
            std::fs::read(root.join("adopted.proof")).unwrap(),
            b"shared\n"
        );
        let state = adapter
            .project(&plan)
            .inspect()
            .await
            .map_err(|e| e.to_string())?;
        let namespace = std::fs::read_link(format!("/proc/{daemon_pid}/ns/ipc")).unwrap();
        let shm = std::fs::metadata(format!("/proc/{daemon_pid}/root/dev/shm")).unwrap();
        for container in &state.containers {
            assert_eq!(
                std::fs::read_link(format!("/proc/{}/ns/ipc", container.pid)).unwrap(),
                namespace
            );
            let other = std::fs::metadata(format!("/proc/{}/root/dev/shm", container.pid)).unwrap();
            assert_eq!((other.dev(), other.ino()), (shm.dev(), shm.ino()));
        }
        adapter
            .project(&plan)
            .endpoint("daemon", "host", daemon)
            .await
            .map_err(|e| e.to_string())?;
        assert!(
            adapter
                .project(&plan)
                .endpoint("adopted", "host", daemon)
                .await
                .is_err()
        );

        // Act: root stop propagates closure; the recorder finishes without interruption.
        let list = cli(
            &executable,
            coordinator,
            &["list", "--format", "json", "--name", "qualification"],
        )
        .await?;
        let entry: serde_json::Value =
            serde_json::from_str(list.lines().last().unwrap()).map_err(|e| e.to_string())?;
        cli(
            &executable,
            coordinator,
            &[
                "node",
                "stop",
                "source",
                "--dataflow",
                entry["uuid"].as_str().unwrap(),
                "--grace",
                "10s",
            ],
        )
        .await?;
        wait_file(&root.join("shm.proof")).await?;

        // Assert: every received payload used shm, and all accepted data reached finalized MCAP.
        let bytes = std::fs::read(root.join("recording.mcap")).unwrap();
        mcap::read::footer(&bytes).unwrap();
        let messages: Vec<_> = mcap::MessageStream::new(&bytes)
            .unwrap()
            .map(|m| m.unwrap())
            .collect();
        assert_eq!(messages.len(), 20);
        assert!(messages.iter().map(|message| message.sequence).eq(0..20));
        assert!(
            messages
                .iter()
                .all(|message| message.log_time == 456 && message.publish_time == 456)
        );
        let mut expected = telemetry_protocol::pb::Packet::decode(payload.as_slice()).unwrap();
        for (sequence, message) in messages.iter().enumerate() {
            let Some(telemetry_protocol::pb::packet::Body::Snapshot(frame)) =
                expected.body.as_mut()
            else {
                panic!("expected a snapshot fixture");
            };
            frame.snapshot.as_mut().unwrap().sequence = sequence as u64;
            assert_eq!(message.data.as_ref(), expected.encode_to_vec().as_slice());
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        for container in adapter.project(&plan).inspect().await.unwrap().containers {
            let output = tokio::process::Command::new("docker")
                .args(["logs", &container.id])
                .output()
                .await
                .unwrap();
            eprintln!(
                "{}: {}\n{}",
                container.service,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    let _ = cli(&executable, coordinator, &["down"]).await;
    let cleanup = StopCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(StopInput {
        plan: Some(plan.clone()),
        services: vec![],
        allow_volume_deletion: true,
        stop_seconds: 5,
    })
    .await
    .unwrap();
    assert!(cleanup.verified, "{cleanup:?}");
    assert!(
        result.is_ok(),
        "{}; artifacts: {}",
        result.unwrap_err(),
        root.display()
    );
    // Arrange: repeat setup, then interrupt app startup before the graph exists.
    for name in ["source.ready", "recorder.ready", "source.done", "shm.proof"] {
        std::fs::remove_file(root.join(name)).unwrap();
    }
    let cancel_dir = root.join("cancel");
    std::fs::create_dir(&cancel_dir).unwrap();
    let plan = ResolveCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(ResolveInput {
        repository_root: None,
        run: plan.run.clone(),
        artifacts: cancel_dir.to_string_lossy().into_owned(),
        project: Some(ComposeProject {
            key: "cancellation".into(),
            directory: root.to_string_lossy().into_owned(),
            files: vec!["compose.json".into()],
            env_files: vec![],
            environment: BTreeMap::new(),
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: true,
        }),
    })
    .await
    .unwrap();
    executor
        .run::<StartCompose>(StartInput {
            plan: Some(plan.clone()),
            services: vec!["daemon".into(), "coordinator".into()],
            readiness_seconds: 15,
        })
        .await
        .unwrap();

    // Act: dropping a pending start still leaves the complete project available for cleanup.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(300),
        executor.run::<StartCompose>(StartInput {
            plan: Some(plan.clone()),
            services: vec!["recorder".into()],
            readiness_seconds: 15,
        }),
    )
    .await;
    let state = adapter.project(&plan).inspect().await.unwrap();
    let cleanup = StopCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(StopInput {
        plan: Some(plan.clone()),
        services: vec![],
        allow_volume_deletion: true,
        stop_seconds: 5,
    })
    .await
    .unwrap();

    // Assert
    assert!(!matches!(cancelled, Ok(Ok(_))));
    assert!(!state.containers.iter().any(|c| c.service == "adopted"));
    assert!(cleanup.verified, "{cleanup:?}");
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "manual cleanup of an interrupted shared-ipc fixture"]
async fn cleanup_interrupted_shared_ipc_test() {
    let Some(path) = std::env::var_os("DORAMCAP_SHM_CLEANUP") else {
        return;
    };
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(Path::new(&path).join("experiment.resolution.json")).unwrap(),
    )
    .unwrap();
    let plan: ResolvedComposeProject = serde_json::from_value(record["plan"].clone()).unwrap();
    assert!(plan.run.starts_with("shm-"));
    assert_eq!(plan.key, "experiment");
    let adapter = DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
        .await
        .unwrap();
    let cleaned = StopCompose {
        adapter: Rc::new(adapter.clone()),
        diagnostics: Default::default(),
    }
    .run(StopInput {
        plan: Some(plan.clone()),
        services: vec![],
        allow_volume_deletion: true,
        stop_seconds: 5,
    })
    .await
    .unwrap();
    assert!(cleaned.verified, "{cleaned:?}");
}

#[tokio::test]
#[ignore = "requires Docker Engine; creates only isolated named volumes"]
async fn volume_deletion_requires_opt_in_and_respects_retention_policy() {
    // Arrange: a disposable volume, an explicitly retained volume and a policy default.
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let adapter = DockerAdapter::connect(&socket).await.unwrap();
    let docker =
        bollard::Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION).unwrap();
    let root = tempfile::tempdir().unwrap();
    let run = format!(
        "volume-permission-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut plan: ResolvedComposeProject = serde_json::from_value(serde_json::json!({
        "run": run, "key": "app", "name": run,
        "manifest": root.path().join("compose.json"), "sha256": "",
        "services": {}, "diagnostics": [], "remove_volumes": false
    }))
    .unwrap();
    let mut names = Vec::new();
    for (suffix, policy) in [
        ("discard", Some("true")),
        ("keep", Some("false")),
        ("default", None),
    ] {
        let name = format!("{run}-{suffix}");
        let mut labels = std::collections::HashMap::from([
            ("io.doramcap.run".into(), run.clone()),
            ("com.docker.compose.project".into(), plan.name.clone()),
        ]);
        if let Some(policy) = policy {
            labels.insert("io.doramcap.remove-volume".into(), policy.into());
        }
        docker
            .create_volume(bollard::models::VolumeCreateRequest {
                name: Some(name.clone()),
                labels: Some(labels),
                ..Default::default()
            })
            .await
            .unwrap();
        names.push(name);
    }
    let input = |plan: &ResolvedComposeProject, permission: Option<bool>| {
        let mut input = serde_json::json!({"plan": plan, "services": [], "stop_seconds": 1});
        if let Some(permission) = permission {
            input["allow_volume_deletion"] = permission.into();
        }
        serde_json::from_value::<StopInput>(input).unwrap()
    };

    // Act / Assert: omission retains even a volume explicitly marked disposable.
    let retained = RemoveCompose {
        adapter: Rc::new(adapter.clone()),
        diagnostics: Default::default(),
    }
    .run(input(&plan, None))
    .await
    .unwrap();
    assert!(retained.verified, "{retained:?}");
    assert_eq!(retained.retained_volumes.len(), 3);
    for name in &names {
        docker.inspect_volume(name).await.unwrap();
    }

    // Act / Assert: permission applies the individual policy and project default.
    let removed = RemoveCompose {
        adapter: Rc::new(adapter.clone()),
        diagnostics: Default::default(),
    }
    .run(input(&plan, Some(true)))
    .await
    .unwrap();
    assert!(removed.verified, "{removed:?}");
    assert_eq!(removed.retained_volumes.len(), 2);
    assert!(matches!(
        docker.inspect_volume(&names[0]).await,
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
        })
    ));
    docker.inspect_volume(&names[1]).await.unwrap();
    docker.inspect_volume(&names[2]).await.unwrap();

    // Act / Assert: an enabled project default still preserves an explicit keep policy.
    plan.remove_volumes = true;
    let removed = RemoveCompose {
        adapter: Rc::new(adapter.clone()),
        diagnostics: Default::default(),
    }
    .run(input(&plan, Some(true)))
    .await
    .unwrap();
    assert!(removed.verified, "{removed:?}");
    assert_eq!(removed.retained_volumes, vec![names[1].clone()]);
    assert!(matches!(
        docker.inspect_volume(&names[2]).await,
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
        })
    ));
    docker.inspect_volume(&names[1]).await.unwrap();
    docker
        .remove_volume(
            &names[1],
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await
        .unwrap();
}
