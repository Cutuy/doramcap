//! Explicitly opt-in real Engine validation. The harness builds unprivileged and
//! elevates only this executable. Every scenario owns distinct named resources.
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc, time::Duration};
use workload_docker::workflows::system_apps::{CpuMonitor, SystemAppsConfig};
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
        filesystem: Some(Default::default()),
        artifacts: root.join(&id),
        recovery_anchor_dir: None,
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
        system_apps: SystemAppsConfig {
            cpu_monitor: Some(CpuMonitor {
                image: image.into(),
                backend: "bpf".into(),
                allow_unconfined_seccomp: true,
                seccomp_profile: None,
            }),
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
#[ignore = "requires Docker, a synthetic monitor image and reviewed io_uring seccomp policy"]
async fn synthetic_recorded_with_reviewed_profile() {
    // Arrange: each test owns its files and Docker run identity.
    let root = std::path::PathBuf::from(std::env::var("DORAMCAP_TEST_OUTPUT").unwrap());
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let profile = std::path::PathBuf::from(std::env::var("DORAMCAP_TEST_SECCOMP").unwrap());
    let run = std::env::var("DORAMCAP_TEST_PREFIX").unwrap();
    write(
        &root.join("demo.json"),
        serde_json::json!({"services":{"worker":{
            "image":image,"entrypoint":["/bin/sh","-c"],
            "command":["trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"],
            "healthcheck":{"test":["CMD","/bin/true"],"interval":"100ms","timeout":"1s","retries":3}
        }}}),
    );
    let socket =
        std::env::var("DORAMCAP_TEST_SOCKET").unwrap_or_else(|_| "/var/run/docker.sock".into());
    let adapter = Rc::new(DockerAdapter::connect(socket).await.unwrap());
    let mut runner = ComposeRunner::new(adapter);
    let artifacts = root.join(&run);
    let spec = ComposeRun {
        filesystem: Some(Default::default()),
        id: run,
        artifacts: artifacts.clone(),
        recovery_anchor_dir: None,
        projects: vec![project(&root, "demo")],
        first_party: vec![],
        system_apps: SystemAppsConfig {
            cpu_monitor: Some(CpuMonitor {
                image,
                backend: "synthetic".into(),
                allow_unconfined_seccomp: false,
                seccomp_profile: Some(profile.to_str().unwrap().into()),
            }),
        },
        readiness_seconds: 15,
        stop_seconds: 5,
    };

    // Act: run and cooperatively stop the isolated workload and collector.
    let report = runner
        .run(
            spec,
            tokio::time::sleep(Duration::from_secs(8)),
            |_: &orchestrator::Event| {},
        )
        .await
        .unwrap();

    // Assert: the run finalized and retained its exact reviewed policy and synthetic recording.
    assert!(report.successful(), "{report:?}");
    assert_eq!(
        std::fs::read(artifacts.join("monitor-seccomp.json")).unwrap(),
        std::fs::read(profile).unwrap()
    );
    let resolved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(artifacts.join("system.compose.json")).unwrap())
            .unwrap();
    let service = &resolved["services"]["cpu-monitor"];
    assert_eq!(service["network_mode"], "none");
    assert!(
        service
            .get("cap_add")
            .is_none_or(|caps| caps.as_array().is_some_and(Vec::is_empty))
    );
    let mounts = service["volumes"].as_array().unwrap();
    assert_eq!(mounts.len(), 1);
    assert_eq!(mounts[0]["source"], artifacts.to_str().unwrap());
    let bytes = std::fs::read(artifacts.join("cpu.mcap")).unwrap();
    let snapshots: Vec<_> = mcap::MessageStream::new(&bytes)
        .unwrap()
        .map(|message| message.unwrap())
        .filter(|message| message.channel.topic == "doramcap/snapshots")
        .collect();
    assert!(!snapshots.is_empty());
    assert!(
        snapshots.iter().all(
            |message| telemetry_protocol::decode(&message.data).unwrap().source == "synthetic"
        )
    );
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
    std::fs::create_dir(host.join("child")).unwrap();
    std::fs::create_dir(&readonly).unwrap();
    std::fs::write(host.join("modified"), "original").unwrap();
    std::fs::write(host.join("deleted"), "original").unwrap();
    let byte_name = std::ffi::OsString::from_vec(b"byte-\xff".to_vec());
    std::fs::write(host.join(&byte_name), b"original").unwrap();
    let byte_target = std::ffi::OsString::from_vec(b"target-\xfe".to_vec());
    std::os::unix::fs::symlink(&byte_target, host.join("byte-link")).unwrap();
    std::fs::write(readonly.join("input"), "unchanged").unwrap();
    let mut alpha = service(&image);
    alpha["command"] = serde_json::json!([
        "printf layer > /layer-added; printf modified > /seed-mod; /bin/rm -f /seed-delete; printf host > /host/added; printf modified > /host/modified; /bin/rm -f /host/deleted; printf volume > /scratch/volume-added; printf shared > /host/child/value; read -r word < /child-ro/value; [ \"$$word\" = shared ] || exit 42; if (printf blocked > /child-ro/value) 2>/dev/null; then exit 43; fi; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    alpha["healthcheck"] = serde_json::json!({"test":["CMD","/bin/true"],"interval":"100ms","timeout":"1s","retries":3});
    alpha["volumes"] = serde_json::json!([
        "scratch:/scratch",
        format!("{}:/host", host.display()),
        format!("{}:/inputs:ro", readonly.display()),
        format!("{}:/child-ro:ro", host.join("child").display())
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
        let original_bytes = std::fs::read(host.join("modified")).unwrap();
        let audit_host = host.clone();
        let audit_artifacts = request.artifacts.clone();
        let isolation_observed = Rc::new(std::cell::Cell::new(false));
        let observed = isolation_observed.clone();
        let stop_observed = isolation_observed.clone();
        let report = manager
            .run(
                request.clone(),
                async move {
                    tokio::time::timeout(Duration::from_secs(30), async {
                        while !stop_observed.get() {
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }
                    })
                    .await
                    .ok();
                },
                move |event: &orchestrator::Event| {
                    sink.borrow_mut().push(event.clone());
                    if event.invocation.step.ends_with("InspectCompose")
                        && matches!(event.state, orchestrator::State::Succeeded)
                    {
                        let views: overlay::OverlaySet = serde_json::from_slice(
                            &std::fs::read(audit_artifacts.join("overlays/prepared.json")).unwrap(),
                        )
                        .unwrap();
                        let tree = views.trees.iter().find(|t| t.source == audit_host).unwrap();
                        if tree.merged.join("added").exists() {
                            let name = std::ffi::OsString::from_vec(b"byte-\xff".to_vec());
                            std::fs::write(tree.merged.join(name), b"changed").unwrap();
                            assert_eq!(
                                std::fs::read(audit_host.join("modified")).unwrap(),
                                original_bytes
                            );
                            assert_eq!(
                                std::fs::read(tree.merged.join("modified")).unwrap(),
                                b"modified"
                            );
                            observed.set(true);
                        }
                    }
                },
            )
            .await
            .unwrap();
        // Assert: each instance and its recording are fresh, all owned objects are gone.
        assert!(report.successful(), "{report:#?}");
        assert!(
            isolation_observed.get(),
            "real container write isolation was not observed"
        );
        assert!(report.filesystem_results.as_ref().unwrap().unmounted);
        assert_eq!(std::fs::read(host.join("modified")).unwrap(), b"modified");
        assert!(!manager.blocked());
        let evidence = report
            .filesystem_changes
            .as_ref()
            .expect("filesystem collection");
        async fn rows(path: &Path) -> Vec<workload_docker::compose::filesystem::FileChange> {
            let mut reader = workload_docker::compose::filesystem::FileChanges::open(path)
                .await
                .unwrap();
            let mut records = Vec::new();
            while let Some(record) = reader.next_change().await.unwrap() {
                records.push(record);
            }
            records
        }
        use workload_docker::compose::filesystem::{ChangeKind, StorageCategory};
        let host_rows = rows(&evidence.host_binds.manifest).await;
        assert!(host_rows.iter().any(
            |r| r.path.as_os_str().as_bytes() == b"byte-\xff" && r.kind == ChangeKind::Modified
        ));
        assert_eq!(std::fs::read(host.join(&byte_name)).unwrap(), b"changed");
        assert_eq!(
            std::fs::read_link(host.join("byte-link"))
                .unwrap()
                .into_os_string(),
            byte_target
        );
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
        let layer_rows = rows(&evidence.container_layers.manifest).await;
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
        let volume_rows = rows(&evidence.named_volumes.manifest).await;
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
    broken["entrypoint"] = serde_json::json!(["/missing-test-executable"]);
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
    assert!(!id.is_empty(), "outsider was not created: {report:#?}");
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
    let collision = request.artifacts.join("filesystem/host-binds.pb");
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
    // Arrange: a held directory descriptor makes strict unmount fail after staging.
    let mut busy_manager = ComposeRunner::new(adapter.clone());
    let request = spec(&root, format!("{prefix}-busy-overlay"), &image);
    let owned_artifacts = request.artifacts.clone();
    let held = Rc::new(RefCell::new(None));
    let held_sink = held.clone();
    // Act
    let report = busy_manager
        .run(
            request,
            tokio::time::sleep(Duration::from_secs(5)),
            move |event: &orchestrator::Event| {
                if event.invocation.step.ends_with("PrepareOverlays")
                    && matches!(event.state, orchestrator::State::Succeeded)
                {
                    let set: overlay::OverlaySet = serde_json::from_slice(
                        &std::fs::read(owned_artifacts.join("overlays/prepared.json")).unwrap(),
                    )
                    .unwrap();
                    let view = &set.trees[0].merged;
                    std::fs::write(view.join("busy-result"), b"preserved").unwrap();
                    *held_sink.borrow_mut() = Some(std::fs::File::open(view).unwrap());
                }
            },
        )
        .await
        .unwrap();
    // Assert: stopped resources and upper results survive; the next run is blocked.
    assert!(busy_manager.blocked() && !report.boundary_clean);
    assert!(report.filesystem_changes.is_some() && report.filesystem_results.is_none());
    assert!(!host.join("busy-result").exists());
    assert!(
        root.join(&report.id)
            .join("overlays/0/upper/busy-result")
            .exists()
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| format!("{d:?}").contains("busy"))
    );
    held.borrow_mut().take();
    reports.push(report);
    for report in &reports {
        audit_recording(&root.join(&report.id));
    }
    write(
        &root.join("summary.json"),
        serde_json::to_value(reports).unwrap(),
    );
}

#[tokio::test]
#[ignore = "requires a local Docker Engine and DORAMCAP_TEST_IMAGE"]
async fn direct_session_lifecycle_and_recovery() {
    // Arrange: each transition owns a fresh project and private metadata.
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let root = std::env::temp_dir().join(format!("direct-session-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    write(
        &root.join("session.json"),
        serde_json::json!({"services":{"app":service(&image)}}),
    );
    let adapter = Rc::new(DockerAdapter::connect(socket).await.unwrap());
    let mut runner = ComposeRunner::new(adapter.clone());
    for mode in 0..3 {
        let id = format!("session-{}-{}", std::process::id(), mode);
        let artifacts = root.join(&id);
        let spec = ComposeRun {
            id: id.clone(),
            artifacts: artifacts.clone(),
            filesystem: None,
            system_apps: SystemAppsConfig::default(),
            recovery_anchor_dir: None,
            projects: vec![project(&root, "session")],
            first_party: vec![],
            readiness_seconds: 10,
            stop_seconds: 3,
        };
        if mode == 0 {
            // Act: fail startup after Compose has begun creating resources.
            write(
                &root.join("broken.json"),
                serde_json::json!({"services":{"app":{
                    "image": image, "entrypoint": ["/nonexistent-program"]
                }}}),
            );
            let mut broken = spec.clone();
            broken.id.push_str("-broken");
            broken.artifacts = root.join(&broken.id);
            broken.projects = vec![project(&root, "broken")];
            let failed = tokio::time::timeout(
                Duration::from_secs(20),
                runner.run(broken, std::future::pending(), |_: &orchestrator::Event| {}),
            )
            .await
            .unwrap()
            .unwrap();
            // Assert: partial startup is cleaned and does not prevent a new launch.
            assert!(failed.execution_error.is_some());
            assert!(failed.boundary_clean, "{failed:?}");
            assert!(!runner.blocked());
        }
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        // Act: await readiness before requesting stop or simulating a crashed caller.
        let run = runner.run_with_ready(
            spec,
            async {
                let _ = stopped.await;
            },
            |_: &orchestrator::Event| {},
            Some(ready),
        );
        let mut run = Box::pin(run);
        let plans = tokio::select! {
            plans = started => plans.unwrap(),
            result = &mut run => panic!("session ended before readiness: {result:?}"),
        };
        // Assert: a ready session is actually running and uses existing mounts.
        assert!(DockerAdapter::ready(&adapter.inspect(&plans[0]).await.unwrap()).unwrap());
        assert!(!artifacts.join("overlays").exists());
        assert!(!artifacts.join("system-services.yaml").exists());
        if mode == 2 {
            // Act: abandon the caller, then recover using the durable shared metadata.
            drop(run);
            assert!(runner.blocked());
            let recovered = workload_docker::workflows::recovery::recover_project(
                adapter.clone(),
                &artifacts,
                &id,
                "session",
                true,
                3,
            )
            .await
            .unwrap();
            // Assert: recovery stops writers and removes the project without app-specific logic.
            assert!(
                recovered.quiesced && recovered.cleanup_verified,
                "{:?}",
                recovered.errors
            );
            assert!(recovered.errors.is_empty(), "{:?}", recovered.errors);
        } else {
            // Act: cooperatively stop the ready session.
            if mode == 0 {
                stop.send(()).unwrap();
            } else {
                adapter
                    .quiesce(StopInput {
                        plan: plans[0].clone(),
                        stop_seconds: 3,
                    })
                    .await
                    .unwrap();
            }
            let report = tokio::time::timeout(Duration::from_secs(20), run)
                .await
                .unwrap()
                .unwrap();
            // Assert: evidence precedes verified removal, and another session can start.
            assert_eq!(report.execution_error.is_some(), mode == 1, "{report:?}");
            assert!(report.boundary_clean, "{report:?}");
            assert!(report.writers_stopped);
            assert!(artifacts.join("outer-evidence.json").is_file());
            assert!(!runner.blocked());
        }
        assert!(
            adapter
                .inspect(&plans[0])
                .await
                .unwrap()
                .containers
                .is_empty()
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine and DORAMCAP_TEST_IMAGE"]
async fn parallel_projects_preserve_readiness_and_shutdown_barriers() {
    // Arrange: either workload can become healthy only after both have started.
    // The supporting service must outlive both writers during cleanup.
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let root = std::env::temp_dir().join(format!("parallel-compose-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let adapter = Rc::new(DockerAdapter::connect(socket).await.unwrap());
    let mut runner = ComposeRunner::new(adapter.clone());
    for failure in [false, true] {
        let shared = root.join(format!("sync-{}", u8::from(failure)));
        std::fs::create_dir(&shared).unwrap();
        let helper = "trap 'test -f /sync/alpha-stopped && test -f /sync/beta-stopped || exit 42; exit 0' TERM INT; touch /sync/system-ready; while :; do sleep 1 & wait $!; done";
        for key in ["helper", "alpha", "beta"] {
            let command = if key == "helper" {
                helper.into()
            } else {
                format!(
                    "test -f /sync/system-ready || exit 9; trap 'touch /sync/{key}-stopped; exit 0' TERM INT; touch /sync/{key}-started; while :; do sleep 1 & wait $!; done"
                )
            };
            let health = if key == "helper" {
                "test -f /sync/system-ready"
            } else if failure && key == "beta" {
                "false"
            } else {
                "test -f /sync/alpha-started && test -f /sync/beta-started"
            };
            write(
                &root.join(format!("{key}.json")),
                serde_json::json!({"services":{"app":{
                    "image":image, "entrypoint":["/bin/sh","-c"], "command":[command.replace('$',"$$")],
                    "volumes":[format!("{}:/sync",shared.display())],
                    "healthcheck":{"test":["CMD-SHELL",health],"interval":"100ms","timeout":"1s","retries":20}
                }}}),
            );
        }
        let id = format!("parallel-{}-{}", std::process::id(), u8::from(failure));
        let events = Rc::new(RefCell::new(Vec::<orchestrator::Event>::new()));
        let sink = events.clone();
        let spec = ComposeRun {
            id: id.clone(),
            artifacts: root.join(&id),
            filesystem: None,
            recovery_anchor_dir: None,
            system_apps: Default::default(),
            first_party: vec![project(&root, "helper")],
            projects: vec![project(&root, "alpha"), project(&root, "beta")],
            readiness_seconds: 5,
            stop_seconds: 3,
        };
        // Act: wait for all ready workloads before requesting stop, or let readiness fail.
        let (ready, started) = tokio::sync::oneshot::channel();
        let report = runner
            .run_with_ready(
                spec,
                async {
                    let _ = started.await;
                },
                move |event: &orchestrator::Event| sink.borrow_mut().push(event.clone()),
                Some(ready),
            )
            .await
            .unwrap();
        // Assert: failure also drains every sibling and preserves shutdown ordering.
        assert_eq!(report.execution_error.is_some(), failure, "{report:?}");
        assert!(report.boundary_clean, "{report:?}");
        assert!(!runner.blocked());
        assert!(shared.join("alpha-stopped").is_file() && shared.join("beta-stopped").is_file());
        assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
        let events = events.borrow();
        let positions = |step: &str, state: orchestrator::State, keys: &[&str]| -> Vec<usize> {
            events
                .iter()
                .enumerate()
                .filter(|(_, event)| {
                    event.invocation.step.ends_with(step)
                        && event.state == state
                        && event.payload.as_ref().is_some_and(|p| {
                            keys.contains(&p["plan"]["key"].as_str().unwrap_or(""))
                        })
                })
                .map(|(index, _)| index)
                .collect()
        };
        if !failure {
            let starts = positions(
                "StartCompose",
                orchestrator::State::Started,
                &["alpha", "beta"],
            );
            // Start outputs have another payload shape; identify terminals by invocation ID.
            let ids: Vec<_> = starts.iter().map(|i| events[*i].invocation.id).collect();
            let first_ready = events
                .iter()
                .position(|e| {
                    ids.contains(&e.invocation.id) && e.state == orchestrator::State::Succeeded
                })
                .unwrap();
            assert_eq!(starts.len(), 2);
            assert!(*starts.iter().max().unwrap() < first_ready);
        }
        let helper_stop = positions("QuiesceCompose", orchestrator::State::Started, &["helper"])[0];
        let writer_ids: Vec<_> = positions(
            "QuiesceCompose",
            orchestrator::State::Started,
            &["alpha", "beta"],
        )
        .iter()
        .map(|i| events[*i].invocation.id)
        .collect();
        assert_eq!(writer_ids.len(), 2);
        assert_eq!(
            events[..helper_stop]
                .iter()
                .filter(|e| writer_ids.contains(&e.invocation.id)
                    && e.state == orchestrator::State::Succeeded)
                .count(),
            2
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
