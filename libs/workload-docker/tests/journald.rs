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

/// Count numbered messages incrementally, retaining identities and detecting duplicates.
fn load_messages(
    container: &str,
    prefix: &str,
    emitted: usize,
    after_exit: Option<usize>,
) -> Value {
    use std::{collections::BTreeSet, io::BufRead, process::Stdio};
    let mut child = std::process::Command::new("journalctl")
        .args([
            "--no-pager",
            "--all",
            "--output=json",
            "--output-fields=MESSAGE,_SYSTEMD_UNIT,_PID",
            &format!("CONTAINER_ID_FULL={container}"),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BTreeSet::new();
    let mut stderr = BTreeSet::new();
    let mut units = BTreeSet::new();
    let mut writers = BTreeSet::new();
    for line in std::io::BufReader::new(child.stdout.take().unwrap()).lines() {
        let entry: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let message = entry["MESSAGE"].as_str().unwrap();
        let rest = message
            .strip_prefix(prefix)
            .expect("unexpected load output");
        let (stream, sequence) = rest.split_once(' ').unwrap();
        let sequence: usize = sequence.parse().unwrap();
        assert!((1..=emitted).contains(&sequence));
        let received = match stream {
            "stdout" => &mut stdout,
            "stderr" => &mut stderr,
            _ => panic!("unexpected stream: {stream}"),
        };
        assert!(received.insert(sequence), "duplicate message: {message}");
        units.insert(
            entry["_SYSTEMD_UNIT"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
        );
        writers.insert(entry["_PID"].as_str().unwrap_or("unknown").to_owned());
    }
    assert!(child.wait().unwrap().success(), "journal traversal failed");
    let mut report = json!({
        "emitted_per_stream": emitted,
        "stdout_received": stdout.len(), "stderr_received": stderr.len(),
        "stdout_missing": emitted - stdout.len(), "stderr_missing": emitted - stderr.len(),
        "journal_units": units, "journal_writers": writers,
    });
    if let Some(sequence) = after_exit {
        report["stdout_after_noisy_exit"] = json!(stdout.range(sequence + 1..).count());
        report["stderr_after_noisy_exit"] = json!(stderr.range(sequence + 1..).count());
    }
    report
}

const QUIET_DURING: usize = 40;
const QUIET_AFTER: usize = 20;
// The qualification host's default interval; pacing keeps the three cases independent.
const LOAD_CASE_INTERVAL: Duration = Duration::from_secs(31);
const LOAD_DEADLINE: Duration = Duration::from_secs(120);

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker, journald, a shell image and journal sync permission"]
async fn compose_journald_qualifies_low_rate_bursts_and_saturation() {
    // Arrange: real Compose resolution, numbered streams and an independent quiet producer.
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket =
        std::env::var("DORAMCAP_TEST_SOCKET").unwrap_or_else(|_| "/var/run/docker.sock".into());
    let adapter = Rc::new(DockerAdapter::connect(&socket).await.unwrap());
    let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)
        .unwrap()
        .negotiate_version()
        .await
        .unwrap();
    let gate_wait = |name: &str| format!("while [ ! -e /gate/{name} ]; do sleep 0.01; done;");
    for (case, count, pause) in [
        ("low-rate", 200, "0.01"),
        ("burst", 4_000, "0"),
        ("saturation", 120_000, "0"),
    ] {
        // Pace test cases across the default journal window without changing host settings.
        tokio::time::sleep(LOAD_CASE_INTERVAL).await;
        let root = tempfile::tempdir().unwrap();
        let gate = root.path().join("gate");
        let artifacts = root.path().join("artifacts");
        std::fs::create_dir(&gate).unwrap();
        std::fs::create_dir(&artifacts).unwrap();
        let noisy = format!(
            "{} i=1; while [ $i -le {count} ]; do printf 'load-{case}-noisy stdout %s\\n' $i; printf 'load-{case}-noisy stderr %s\\n' $i >&2; i=$((i+1)); {} done",
            gate_wait("start"),
            if pause == "0" {
                String::new()
            } else {
                format!("sleep {pause};")
            }
        );
        let quiet = format!(
            "{} i=1; while [ $i -le {QUIET_DURING} ]; do printf 'load-{case}-quiet stdout %s\\n' $i; printf 'load-{case}-quiet stderr %s\\n' $i >&2; i=$((i+1)); sleep 0.05; done; {} while [ $i -le {} ]; do printf 'load-{case}-quiet stdout %s\\n' $i; printf 'load-{case}-quiet stderr %s\\n' $i >&2; i=$((i+1)); done",
            gate_wait("start"),
            gate_wait("after"),
            QUIET_DURING + QUIET_AFTER
        );
        let service = |script: String| {
            json!({"image":image,"network_mode":"none",
            "entrypoint":["/bin/sh","-c"],"command":[script.replace('$', "$$")],
            "volumes":[{"type":"bind","source":gate,"target":"/gate","read_only":true}]})
        };
        std::fs::write(
            root.path().join("compose.json"),
            serde_json::to_vec(&json!({
                "services":{"noisy":service(noisy),"quiet":service(quiet)}
            }))
            .unwrap(),
        )
        .unwrap();
        let diagnostics = Rc::new(RefCell::new(Vec::new()));
        let plan = ResolveCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(ResolveInput {
            run: format!("journal-load-{}-{case}", std::process::id()),
            artifacts: artifacts.to_str().unwrap().into(),
            project: Some(ComposeProject {
                key: "load".into(),
                directory: root.path().to_str().unwrap().into(),
                files: vec!["compose.json".into()],
                ..Default::default()
            }),
            repository_root: None,
        })
        .await
        .unwrap();

        // Act: start both producers together, then emit quiet markers after the noisy exit.
        StartCompose {
            adapter: adapter.clone(),
            diagnostics: diagnostics.clone(),
        }
        .run(StartInput {
            plan: Some(plan.clone()),
            services: vec![],
            readiness_seconds: 10,
        })
        .await
        .unwrap();
        let state = adapter.project(&plan).inspect().await.unwrap();
        assert_eq!(state.containers.len(), 2);
        let noisy = state
            .containers
            .iter()
            .find(|c| c.service == "noisy")
            .unwrap();
        let quiet = state
            .containers
            .iter()
            .find(|c| c.service == "quiet")
            .unwrap();
        for container in &state.containers {
            let info = docker.inspect_container(&container.id, None).await.unwrap();
            assert_eq!(
                info.host_config.unwrap().log_config.unwrap().typ.as_deref(),
                Some("journald")
            );
        }
        let started = std::time::Instant::now();
        std::fs::write(gate.join("start"), b"").unwrap();
        let mut noisy_exit = docker.wait_container(&noisy.id, None::<WaitContainerOptions>);
        let noisy_exit = tokio::time::timeout(LOAD_DEADLINE, noisy_exit.next()).await;
        let elapsed = started.elapsed();
        std::fs::write(gate.join("after"), b"").unwrap();
        let mut quiet_exit = docker.wait_container(&quiet.id, None::<WaitContainerOptions>);
        let quiet_exit = tokio::time::timeout(LOAD_DEADLINE, quiet_exit.next()).await;
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
        assert!(cleanup.verified, "{cleanup:?}");
        assert_eq!(noisy_exit.unwrap().unwrap().unwrap().status_code, 0);
        assert_eq!(quiet_exit.unwrap().unwrap().unwrap().status_code, 0);
        assert!(
            std::process::Command::new("journalctl")
                .arg("--sync")
                .status()
                .unwrap()
                .success()
        );
        let noisy_report = load_messages(&noisy.id, &format!("load-{case}-noisy "), count, None);
        let quiet_report = load_messages(
            &quiet.id,
            &format!("load-{case}-quiet "),
            QUIET_DURING + QUIET_AFTER,
            Some(QUIET_DURING),
        );
        let report = json!({"case":case,"noisy_elapsed_ms":elapsed.as_millis(),
            "noisy":noisy_report,"quiet":quiet_report});
        println!("JOURNAL_LOAD {report}");

        // Assert: normal traffic is complete; saturation reports exact loss, including quiet output.
        for container in &state.containers {
            assert!(docker.inspect_container(&container.id, None).await.is_err());
        }
        if case != "saturation" {
            for producer in ["noisy", "quiet"] {
                assert_eq!(report[producer]["stdout_missing"], 0, "{report}");
                assert_eq!(report[producer]["stderr_missing"], 0, "{report}");
            }
        }
    }
}
