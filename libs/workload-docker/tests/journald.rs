//! Opt-in host qualification: build unprivileged, then elevate only this executable.
use bollard::{Docker, query_parameters::WaitContainerOptions};
use futures_util::StreamExt;
use orchestrator::Step;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, time::Duration};
use workload_docker::compose::*;

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker, journald, a shell image and journal sync permission"]
async fn compose_journald_preserves_failure_output_and_logs_after_removal() {
    // Arrange: isolated source and run artifacts; reuse the native Docker harness inputs.
    let root = tempfile::tempdir().unwrap();
    let artifacts = root.path().join("artifacts");
    std::fs::create_dir(&artifacts).unwrap();
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket =
        std::env::var("DORAMCAP_TEST_SOCKET").unwrap_or_else(|_| "/var/run/docker.sock".into());
    let run = format!("journal-{}", std::process::id());
    let mut service = json!({"image":image,"network_mode":"none",
        "entrypoint":["/bin/sh","-c"],
        "command":["echo journal-stdout; echo journal-stderr >&2; exit 7"]});
    let mut services = json!({"journal":service});
    service["logging"] = json!({"driver":"json-file","options":{"max-size":"1m"}});
    services["custom"] = service;
    let source = serde_json::to_vec(&json!({"services":services})).unwrap();
    let path = root.path().join("compose.json");
    std::fs::write(&path, &source).unwrap();
    let adapter = Rc::new(DockerAdapter::connect(&socket).await.unwrap());
    let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
        .unwrap()
        .negotiate_version()
        .await
        .unwrap();
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let plan = ResolveCompose {
        adapter: adapter.clone(),
        diagnostics: diagnostics.clone(),
    }
    .run(ResolveInput {
        run: run.clone(),
        artifacts: artifacts.to_str().unwrap().into(),
        project: Some(ComposeProject {
            key: "journal".into(),
            directory: root.path().to_str().unwrap().into(),
            files: vec!["compose.json".into()],
            ..Default::default()
        }),
        repository_root: None,
    })
    .await
    .unwrap();

    // Act: both containers fail; wait for native exits before inspecting failure diagnostics.
    let _ = StartCompose {
        adapter: adapter.clone(),
        diagnostics: diagnostics.clone(),
    }
    .run(StartInput {
        plan: Some(plan.clone()),
        services: vec![],
        readiness_seconds: 10,
    })
    .await;
    let initial = adapter.project(&plan).inspect().await.unwrap();
    for container in &initial.containers {
        let mut exit = docker.wait_container(&container.id, None::<WaitContainerOptions>);
        // Docker reports a nonzero exit as a stream error; completion still proves it stopped.
        let _ = tokio::time::timeout(Duration::from_secs(10), exit.next())
            .await
            .unwrap()
            .unwrap();
    }
    let state = adapter.project(&plan).inspect().await.unwrap();
    let mut drivers = Vec::new();
    for container in &state.containers {
        let info = docker.inspect_container(&container.id, None).await.unwrap();
        drivers.push((
            container.service.clone(),
            info.host_config.unwrap().log_config.unwrap(),
        ));
    }
    let cleanup = StopCompose {
        adapter: adapter.clone(),
        diagnostics,
    }
    .run(StopInput {
        plan: Some(plan),
        ..Default::default()
    })
    .await
    .unwrap();
    let sync = std::process::Command::new("journalctl")
        .arg("--sync")
        .status()
        .unwrap();
    let journal = state
        .containers
        .iter()
        .find(|c| c.service == "journal")
        .unwrap();
    let output = std::process::Command::new("journalctl")
        .args([
            "--no-pager",
            "--output=cat",
            &format!("CONTAINER_ID_FULL={}", journal.id),
        ])
        .output()
        .unwrap();

    // Assert: the default is effective, native overrides survive, and Docker still returns tails.
    assert_eq!(state.containers.len(), 2);
    for container in &state.containers {
        assert_eq!(container.exit_code, Some(7));
        assert!(
            container.last_output.contains("journal-stdout"),
            "{container:?}"
        );
        assert!(
            container.last_output.contains("journal-stderr"),
            "{container:?}"
        );
        assert!(docker.inspect_container(&container.id, None).await.is_err());
    }
    for (service, config) in drivers {
        assert_eq!(
            config.typ.as_deref(),
            Some(if service == "journal" {
                "journald"
            } else {
                "json-file"
            })
        );
        if service == "custom" {
            assert_eq!(
                config.config.unwrap().get("max-size").map(String::as_str),
                Some("1m")
            );
        }
    }
    assert!(cleanup.verified, "{cleanup:?}");
    assert!(sync.success());
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("journal-stdout"), "{text}");
    assert!(text.contains("journal-stderr"), "{text}");
    assert_eq!(std::fs::read(path).unwrap(), source);
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(artifacts.join("journal.compose.json")).unwrap())
            .unwrap();
    assert_eq!(
        frozen["services"]["journal"]["logging"]["driver"],
        "journald"
    );
}
