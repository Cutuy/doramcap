//! Explicitly opt-in real Engine validation. The harness builds unprivileged and
//! elevates only this executable. Every scenario owns distinct named resources.
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc, time::Duration};
use workload_docker::{compose::*, workflows::recorded_compose::*};

fn project(root: &Path, key: &str) -> ComposeProject {
    ComposeProject {
        key: key.into(),
        directory: root.into(),
        files: vec![format!("{key}.json").into()],
        env_files: vec![],
        environment: BTreeMap::new(),
        profiles: vec![],
        services: vec![],
        completed_services: vec![],
        remove_volumes: true,
    }
}
fn spec(root: &Path, id: String, image: &str) -> ComposeRun {
    ComposeRun {
        artifacts: root.join(&id),
        id,
        projects: vec![
            {
                let mut p = project(root, "alpha");
                p.env_files = vec!["inputs.env".into()];
                p.environment
                    .insert("EXPLICIT_VALUE".into(), "from-caller".into());
                p
            },
            project(root, "beta"),
        ],
        first_party: vec![],
        monitor: CpuMonitor {
            image: image.into(),
            backend: "bpf".into(),
            allow_unconfined_seccomp: true,
        },
        readiness_seconds: 10,
        stop_seconds: 3,
    }
}
fn write(path: &Path, value: serde_json::Value) {
    std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
fn service(image: &str) -> serde_json::Value {
    serde_json::json!({"image":image,"entrypoint":["/bin/sh","-c"],"command":["trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"]})
}
fn audit_recording(root: &Path) -> String {
    let bytes = std::fs::read(root.join("cpu.mcap")).unwrap();
    mcap::read::footer(&bytes).unwrap();
    let mut session = None;
    let mut sequences = Vec::new();
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let message = message.unwrap();
        if message.channel.topic == "doramcap/snapshots" {
            let frame = telemetry_protocol::decode(&message.data).unwrap();
            assert_eq!(frame.source, "bpf");
            assert_eq!(frame.snapshot.dropped_batches, 0);
            sequences.push(frame.snapshot.sequence);
            if let Some(ref previous) = session {
                assert_eq!(&frame.session_id, previous);
            }
            session = Some(frame.session_id);
        }
    }
    assert!(sequences.len() >= 3);
    assert!(sequences.iter().copied().eq(0..sequences.len() as u64));
    session.unwrap()
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker, native monitor image and BPF capabilities; use scripts/compose-e2e.py"]
async fn multi_project_lifecycle() {
    // Arrange: fresh shared source files, prebuilt image and independent project boundaries.
    let root =
        std::path::PathBuf::from(std::env::var("DORAMCAP_TEST_OUTPUT").expect("harness output"));
    let image = std::env::var("DORAMCAP_TEST_IMAGE").expect("harness image");
    let prefix = std::env::var("DORAMCAP_TEST_PREFIX").expect("harness identity");
    let adapter = Rc::new(
        DockerAdapter::connect("/var/run/docker.sock")
            .await
            .unwrap(),
    );
    let mut manager = ComposeRunner::new(adapter.clone());
    let mut reports = Vec::new();
    let host = root.join("host-files");
    let readonly = root.join("read-only-files");
    std::fs::create_dir(&host).unwrap();
    std::fs::create_dir(&readonly).unwrap();
    std::fs::write(host.join("modified"), "original").unwrap();
    std::fs::write(host.join("deleted"), "original").unwrap();
    std::fs::write(readonly.join("input"), "unchanged").unwrap();
    let mut alpha = service(&image);
    alpha["command"] = serde_json::json!([
        "printf layer > /layer-added; printf modified > /seed-mod; /bin/rm -f /seed-delete; printf host > /host/added; printf modified > /host/modified; /bin/rm -f /host/deleted; printf volume > /scratch/volume-added; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    alpha["healthcheck"] = serde_json::json!({"test":["CMD","/bin/true"],"interval":"100ms","timeout":"1s","retries":3});
    alpha["volumes"] = serde_json::json!([
        "scratch:/scratch",
        format!("{}:/host", host.display()),
        format!("{}:/inputs:ro", readonly.display())
    ]);
    alpha["environment"] =
        serde_json::json!({"LITERAL":"value$$not_interpolated", "PREPARED":"${EXPLICIT_VALUE}"});
    std::fs::write(root.join("inputs.env"), "EXPLICIT_VALUE=from-file\n").unwrap();
    // A disabled profile must not cause missing-image failure or launch.
    write(
        &root.join("alpha.json"),
        serde_json::json!({"services":{"worker":alpha,"disabled":{"image":"missing-image-not-used:never","profiles":["disabled"]}},"volumes":{"scratch":{}}}),
    );
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"worker":service(&image)}}),
    );
    // Shared bind identity is observed once with both consumers attached.
    let mut beta_service = service(&image);
    beta_service["volumes"] = serde_json::json!([format!("{}:/shared", host.display())]);
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"worker":beta_service}}),
    );
    let mut monitor_sessions = Vec::new();
    for index in 0..2 {
        // Act: add a service to existing Compose input; workflow and registrations stay identical.
        if index == 1 {
            write(
                &root.join("beta.json"),
                serde_json::json!({"services":{"worker":service(&image),"added":service(&image)}}),
            );
        }
        let mut request = spec(&root, format!("{prefix}-normal{index}"), &image);
        if index == 0 {
            request.projects[0].remove_volumes = false;
        }
        if index == 1 {
            write(
                &root.join("auxiliary.json"),
                serde_json::json!({"services":{"helper":service(&image)}}),
            );
            request.first_party.push(project(&root, "auxiliary"));
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let report = manager
            .run(
                request.clone(),
                tokio::time::sleep(Duration::from_secs(4)),
                move |event: &orchestrator::Event| sink.borrow_mut().push(event.clone()),
            )
            .await
            .unwrap();
        // Assert: each instance and its recording are fresh, all owned objects are gone.
        assert!(report.successful(), "{report:#?}");
        assert!(!manager.blocked());
        let evidence = report
            .filesystem_changes
            .as_ref()
            .expect("filesystem collection");
        let rows = |path: &Path| -> Vec<workload_docker::compose::filesystem::FileChange> {
            std::fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        };
        use workload_docker::compose::filesystem::{ChangeKind, StorageCategory};
        let host_rows = rows(&evidence.host_binds.manifest);
        assert!(
            host_rows
                .iter()
                .all(|r| r.category == StorageCategory::HostBind)
        );
        if index == 0 {
            for (name, kind) in [
                ("added", ChangeKind::Added),
                ("modified", ChangeKind::Modified),
                ("deleted", ChangeKind::Deleted),
            ] {
                let row = host_rows
                    .iter()
                    .find(|r| r.path == Path::new(name))
                    .expect("host transition");
                assert_eq!(row.kind, kind);
                assert_eq!(row.consumers.len(), 2);
            }
        } else {
            // Fresh baseline: prior run's additions/deletions are not replayed.
            assert!(
                !host_rows
                    .iter()
                    .any(|r| r.kind == ChangeKind::Added || r.kind == ChangeKind::Deleted)
            );
        }
        assert_eq!(
            std::fs::read_to_string(readonly.join("input")).unwrap(),
            "unchanged"
        );
        let layer_rows = rows(&evidence.container_layers.manifest);
        assert!(
            layer_rows
                .iter()
                .all(|r| r.category == StorageCategory::ContainerLayer
                    && r.before.is_none()
                    && r.after.is_none())
        );
        for (name, kind) in [
            ("/layer-added", ChangeKind::Added),
            ("/seed-mod", ChangeKind::Modified),
            ("/seed-delete", ChangeKind::Deleted),
        ] {
            assert!(
                layer_rows
                    .iter()
                    .any(|r| r.path == Path::new(name) && r.kind == kind),
                "missing {name}: {layer_rows:?}"
            );
        }
        let volume_rows = rows(&evidence.named_volumes.manifest);
        assert!(
            volume_rows
                .iter()
                .all(|r| r.category == StorageCategory::NamedVolume)
        );
        assert!(
            volume_rows
                .iter()
                .any(|r| r.path == Path::new("volume-added") && r.kind == ChangeKind::Added)
        );
        assert!(!evidence.named_volumes.warnings.is_empty());
        assert!(
            !request.artifacts.join("layer-added").exists(),
            "no content export"
        );
        if index == 0 {
            let retained: Vec<_> = report
                .cleanup
                .iter()
                .flat_map(|c| &c.retained_volumes)
                .collect();
            assert_eq!(retained.len(), 1);
            assert!(
                tokio::process::Command::new("docker")
                    .args(["volume", "inspect", retained[0]])
                    .output()
                    .await
                    .unwrap()
                    .status
                    .success()
            );
        }
        let beta: ResolvedComposeProject = serde_json::from_value(
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(request.artifacts.join("beta.resolution.json")).unwrap(),
            )
            .unwrap()["plan"]
                .clone(),
        )
        .unwrap();
        assert_eq!(beta.services.len(), index + 1);
        assert!(adapter.inspect(&beta).await.unwrap().containers.is_empty());
        let raw = std::fs::read_to_string(request.artifacts.join("alpha.compose.json")).unwrap();
        assert!(raw.contains("value$$not_interpolated"));
        assert!(!raw.contains("missing-image-not-used"));
        assert!(raw.contains("from-caller"));
        let inspected = std::fs::read_dir(&request.artifacts)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| {
                e.file_name().to_string_lossy().starts_with("alpha-")
                    && e.file_name().to_string_lossy().ends_with(".stopped.json")
            })
            .unwrap();
        let info: serde_json::Value =
            serde_json::from_slice(&std::fs::read(inspected.path()).unwrap()).unwrap();
        assert!(
            info["Config"]["Env"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "LITERAL=value$not_interpolated"),
            "{info}"
        );
        assert!(
            events
                .borrow()
                .iter()
                .any(|e| e.invocation.step.ends_with("QuiesceCompose"))
        );
        {
            let events = events.borrow();
            let collected = events
                .iter()
                .find(|e| {
                    e.invocation.step.ends_with("CollectFilesystemChanges")
                        && matches!(e.state, orchestrator::State::Succeeded)
                })
                .unwrap();
            for stopped in events.iter().filter(|e| {
                e.invocation.step.ends_with("QuiesceCompose")
                    && matches!(e.state, orchestrator::State::Succeeded)
            }) {
                assert!(
                    collected
                        .invocation
                        .dependencies
                        .contains(&stopped.invocation.id)
                );
            }
            for removed in events
                .iter()
                .filter(|e| e.invocation.step.ends_with("RemoveCompose"))
            {
                assert!(
                    removed
                        .invocation
                        .dependencies
                        .contains(&collected.invocation.id)
                );
            }
        }
        assert!(
            std::fs::metadata(request.artifacts.join("docker-events.jsonl"))
                .unwrap()
                .len()
                > 0
        );
        monitor_sessions.push(audit_recording(&request.artifacts));
        reports.push(report);
        // Act: teardown a second time.
        let repeated = adapter
            .stop(StopInput {
                plan: beta,
                stop_seconds: 3,
            })
            .await
            .unwrap();
        // Assert
        assert!(
            repeated.verified && repeated.diagnostics.is_empty(),
            "{repeated:#?}"
        );
    }
    assert_ne!(monitor_sessions[0], monitor_sessions[1]);

    // Arrange: a later project fails after earlier foreign and native services start.
    let mut broken = service(&image);
    broken["volumes"] = serde_json::json!([{"type":"bind","source":format!("/does-not-exist-{prefix}"),"target":"/missing","bind":{"create_host_path":false}}]);
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"broken":broken}}),
    );
    let request = spec(&root, format!("{prefix}-partial"), &image);
    // Act
    let report = manager
        .run(
            request.clone(),
            std::future::pending(),
            |_: &orchestrator::Event| {},
        )
        .await
        .unwrap();
    // Assert
    assert!(report.execution_error.is_some(), "{report:#?}");
    assert!(
        report.boundary_clean && report.recording_finalized,
        "{report:#?}"
    );
    assert_eq!(report.cleanup.len(), 3);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.kind == "compose" && d.status.is_some() && d.stderr.is_some())
    );
    assert!(!manager.blocked());
    reports.push(report);

    // Arrange: unhealthy services are errors, even though their processes run.
    let mut unhealthy = service(&image);
    unhealthy["healthcheck"] = serde_json::json!({"test":["CMD","/bin/false"],"interval":"100ms","timeout":"1s","retries":1});
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"unhealthy":unhealthy}}),
    );
    // Act
    let report = manager
        .run(
            spec(&root, format!("{prefix}-unhealthy"), &image),
            std::future::pending(),
            |_: &orchestrator::Event| {},
        )
        .await
        .unwrap();
    // Assert
    assert!(
        report
            .execution_error
            .as_deref()
            .unwrap_or_default()
            .contains("unhealthy"),
        "{report:#?}"
    );
    assert!(
        report.boundary_clean && report.recording_finalized,
        "{report:#?}"
    );
    reports.push(report);

    // Arrange: Compose waits indefinitely on an unfinished dependency. CLI timeout
    // must still terminate the plugin and reconcile partially created containers.
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"init":service(&image),"worker":{"image":image,"entrypoint":["/bin/sleep"],"command":["300"],"depends_on":{"init":{"condition":"service_completed_successfully"}}}}}),
    );
    let mut request = spec(&root, format!("{prefix}-timeout"), &image);
    request.readiness_seconds = 2;
    // Act
    let report = manager
        .run(
            request,
            std::future::pending(),
            |_: &orchestrator::Event| {},
        )
        .await
        .unwrap();
    // Assert
    assert!(
        report
            .execution_error
            .as_deref()
            .unwrap_or_default()
            .contains("deadline"),
        "{report:#?}"
    );
    assert!(
        report.boundary_clean && report.recording_finalized,
        "{report:#?}"
    );
    reports.push(report);
    // Arrange: a separately owned container appears in this project's namespace.
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"worker":service(&image)}}),
    );
    let request = spec(&root, format!("{prefix}-outsider"), &image);
    let project_label = format!("com.docker.compose.project=dc-{}-alpha", request.id);
    let outsider_name = format!("{prefix}-outsider-container");
    let outsider_id = Rc::new(RefCell::new(String::new()));
    let created = outsider_id.clone();
    let outsider_image = image.clone();
    let stop = async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let output = tokio::process::Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &outsider_name,
                "--network",
                "none",
                "--label",
                &project_label,
                "--entrypoint",
                "/bin/sleep",
                &outsider_image,
                "300",
            ])
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        *created.borrow_mut() = String::from_utf8(output.stdout).unwrap().trim().into();
    };
    // Act
    let report = manager
        .run(request, stop, |_: &orchestrator::Event| {})
        .await
        .unwrap();
    // Assert: ambiguous ownership preserves stopped resources; the outsider remains and admission blocks.
    assert!(!report.boundary_clean && manager.blocked(), "{report:#?}");
    let id = outsider_id.borrow().clone();
    let inspection = tokio::process::Command::new("docker")
        .args(["inspect", "--format", "{{.State.Running}}", &id])
        .output()
        .await
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&inspection.stdout).trim(), "true");
    assert!(
        manager
            .run(
                spec(&root, format!("{prefix}-blocked"), &image),
                std::future::pending(),
                |_: &orchestrator::Event| {}
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("blocked")
    );
    // Act: only the test owner removes its outsider.
    assert!(
        tokio::process::Command::new("docker")
            .args(["rm", "-f", &id])
            .status()
            .await
            .unwrap()
            .success()
    );
    reports.push(report);
    // Arrange: make metadata collection fail after successful quiescence.
    let mut fresh_manager = ComposeRunner::new(adapter.clone());
    let request = spec(&root, format!("{prefix}-diff-failure"), &image);
    let collision = request.artifacts.join("filesystem/host-binds.jsonl");
    // Act
    let report = fresh_manager
        .run(
            request,
            tokio::time::sleep(Duration::from_secs(4)),
            move |event: &orchestrator::Event| {
                if event.invocation.step.ends_with("CaptureFilesystemBaseline")
                    && matches!(event.state, orchestrator::State::Succeeded)
                {
                    std::fs::write(&collision, "owned test collision").unwrap();
                }
            },
        )
        .await
        .unwrap();
    // Assert: no removal after missing evidence; writers still stopped.
    assert!(
        fresh_manager.blocked() && !report.boundary_clean && report.filesystem_changes.is_none()
    );
    let ids = tokio::process::Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("label=io.doramcap.run={}", report.id),
        ])
        .output()
        .await
        .unwrap();
    let ids = String::from_utf8(ids.stdout).unwrap();
    assert!(!ids.trim().is_empty());
    for id in ids.split_whitespace() {
        let state = tokio::process::Command::new("docker")
            .args(["inspect", "--format", "{{.State.Running}}", id])
            .output()
            .await
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&state.stdout).trim(), "false");
    }
    reports.push(report);
    for report in &reports {
        audit_recording(&root.join(&report.id));
    }
    write(
        &root.join("summary.json"),
        serde_json::to_value(reports).unwrap(),
    );
}
