//! Explicitly opt-in real Engine validation. The harness builds unprivileged and
//! elevates only this executable. Every scenario owns distinct named resources.
use orchestrator::Step;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc, time::Duration};
use workload_docker::workflows::recorded_compose::finalization::FinalizeRun;
use workload_docker::{compose::*, workflows::recorded_compose::*};
use workload_lifecycle::recording::Recording;

fn step_id<S: Step>() -> &'static str {
    S::ID.unwrap_or(std::any::type_name::<S>())
}

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
fn monitor(root: &Path, image: &str, artifacts: &Path) -> ComposeProject {
    let file = root.join("monitor.yaml");
    if !file.exists() {
        std::fs::write(
            &file,
            include_bytes!("../../../apps/system-monitor/deploy/recording.yaml"),
        )
        .unwrap();
    }
    ComposeProject {
        key: "monitor".into(),
        directory: root.into(),
        files: vec![file],
        env_files: vec![],
        environment: BTreeMap::from([
            ("DORAMCAP_MONITOR_IMAGE".into(), image.into()),
            (
                "DORAMCAP_ARTIFACTS".into(),
                artifacts.to_string_lossy().into_owned(),
            ),
        ]),
        profiles: vec![],
        services: vec![],
        completed_services: vec![],
        remove_volumes: true,
    }
}
fn recordings(artifacts: &Path) -> Vec<Recording> {
    vec![Recording {
        stream: artifacts.join("cpu.pb"),
        recording: artifacts.join("cpu.mcap"),
    }]
}
fn spec(root: &Path, id: String, image: &str) -> ComposeRun {
    ComposeRun {
        filesystem: Some(Default::default()),
        artifacts: root.join(&id),
        recovery_anchor_dir: None,
        id: id.clone(),
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
        first_party: vec![monitor(root, image, &root.join(&id))],
        dora: Default::default(),
        recordings: recordings(&root.join(&id)),
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
#[ignore = "requires Docker, a native BPF monitor image and Linux capabilities"]
async fn native_recorded_workflow() {
    // Arrange: each test owns its files and Docker run identity.
    let root = std::path::PathBuf::from(std::env::var("DORAMCAP_TEST_OUTPUT").unwrap());
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
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
        first_party: vec![monitor(&root, &image, &artifacts)],
        dora: Default::default(),
        recordings: recordings(&artifacts),
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

    // Assert: the run finalized its native recording and verified cleanup.
    assert!(report.successful(), "{report:?}");
    let resolved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(artifacts.join("experiment.compose.json")).unwrap())
            .unwrap();
    let service = &resolved["services"]["cpu-monitor"];
    assert_eq!(service["network_mode"], "none");
    assert!(
        service["security_opt"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option == "seccomp=unconfined")
    );
    let caps = service["cap_add"].as_array().unwrap();
    assert!(caps.iter().any(|cap| cap == "BPF"));
    assert!(caps.iter().any(|cap| cap == "PERFMON"));
    let mounts = service["volumes"].as_array().unwrap();
    assert!(
        mounts
            .iter()
            .any(|mount| mount["target"] == "/sys/fs/cgroup")
    );
    assert!(
        mounts
            .iter()
            .any(|mount| mount["target"] == "/sys/kernel/btf")
    );
    let bytes = std::fs::read(artifacts.join("cpu.mcap")).unwrap();
    let snapshots: Vec<_> = mcap::MessageStream::new(&bytes)
        .unwrap()
        .map(|message| message.unwrap())
        .filter(|message| message.channel.topic == "doramcap/snapshots")
        .collect();
    assert!(!snapshots.is_empty());
    assert!(
        snapshots
            .iter()
            .all(|message| telemetry_protocol::decode(&message.data).unwrap().source == "bpf")
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
        serde_json::json!({"services":{"second-worker":service(&image)}}),
    );
    // Shared bind identity is observed once with both consumers attached.
    let mut beta_service = service(&image);
    beta_service["volumes"] = serde_json::json!([format!("{}:/shared", host.display())]);
    write(
        &root.join("beta.json"),
        serde_json::json!({"services":{"second-worker":beta_service}}),
    );
    let mut monitor_sessions = Vec::new();
    for index in 0..2 {
        // Act: add a service to existing Compose input; workflow and registrations stay identical.
        if index == 1 {
            write(
                &root.join("beta.json"),
                serde_json::json!({"services":{"second-worker":service(&image),"added":service(&image)}}),
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
                    if event.invocation.step == InspectCompose::ID.unwrap()
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
            let retained = &report.cleanup.retained_volumes;
            assert_eq!(retained.len(), 1);
            assert!(
                tokio::process::Command::new("docker")
                    .args(["volume", "inspect", &retained[0]])
                    .output()
                    .await
                    .unwrap()
                    .status
                    .success()
            );
        }
        let beta: ResolvedComposeProject = serde_json::from_value(
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(request.artifacts.join("experiment.resolution.json")).unwrap(),
            )
            .unwrap()["plan"]
                .clone(),
        )
        .unwrap();
        assert_eq!(beta.services.len(), 3 + index * 2);
        assert!(adapter.inspect(&beta).await.unwrap().containers.is_empty());
        let raw =
            std::fs::read_to_string(request.artifacts.join("experiment.compose.json")).unwrap();
        assert!(raw.contains("value$$not_interpolated"));
        assert!(!raw.contains("missing-image-not-used"));
        assert!(raw.contains("from-caller"));
        let inspected = std::fs::read_dir(&request.artifacts)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("experiment-worker-")
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
            let quiesced = events
                .iter()
                .find(|e| {
                    e.invocation.step == step_id::<QuiesceCompose>()
                        && e.state == orchestrator::State::Succeeded
                })
                .unwrap();
            let finalized = events
                .iter()
                .find(|e| {
                    e.invocation.step == FinalizeRun::ID.unwrap()
                        && e.state == orchestrator::State::Succeeded
                })
                .unwrap();
            let removing = events
                .iter()
                .find(|e| {
                    e.invocation.step == step_id::<RemoveCompose>()
                        && e.state == orchestrator::State::Started
                })
                .unwrap();
            assert_eq!(collected.invocation.parent, Some(finalized.invocation.id));
            assert!(
                finalized
                    .invocation
                    .dependencies
                    .contains(&quiesced.invocation.id)
            );
            assert!(
                removing
                    .invocation
                    .dependencies
                    .contains(&finalized.invocation.id)
            );
            let collected_start = events
                .iter()
                .position(|e| {
                    e.invocation.id == collected.invocation.id
                        && e.state == orchestrator::State::Started
                })
                .unwrap();
            for (index, _) in events.iter().enumerate().filter(|(_, e)| {
                e.invocation.step.ends_with("QuiesceCompose")
                    && e.state == orchestrator::State::Succeeded
            }) {
                assert!(index < collected_start);
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
                allow_volume_deletion: true,
                services: vec![],
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
    assert!(report.cleanup.verified);
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
        serde_json::json!({"services":{"init":service(&image),"dependent-worker":{"image":image,"entrypoint":["/bin/sleep"],"command":["300"],"depends_on":{"init":{"condition":"service_completed_successfully"}}}}}),
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
        serde_json::json!({"services":{"second-worker":service(&image)}}),
    );
    let request = spec(&root, format!("{prefix}-outsider"), &image);
    let resolution = request.artifacts.join("experiment.resolution.json");
    let outsider_name = format!("{prefix}-outsider-container");
    let (injected, notification) = tokio::sync::oneshot::channel();
    let outsider_image = image.clone();
    let inject = async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(resolution).unwrap()).unwrap();
        let project_label = format!(
            "com.docker.compose.project={}",
            saved["plan"]["name"].as_str().unwrap()
        );
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
        let id = String::from_utf8(output.stdout).unwrap().trim().to_owned();
        let _ = injected.send(());
        id
    };
    // Act
    let run = manager.run(
        request,
        async {
            let _ = notification.await;
        },
        |_: &orchestrator::Event| {},
    );
    let (report, id) = tokio::join!(run, inject);
    let report = report.unwrap();
    // Assert: ambiguous ownership preserves stopped resources; the outsider remains and admission blocks.
    assert!(!report.boundary_clean && manager.blocked(), "{report:#?}");
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
                if event.invocation.step == overlay::PrepareOverlays::ID.unwrap()
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
            dora: Default::default(),
            recordings: vec![],
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
            plan = started => vec![plan.unwrap()],
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
                "experiment",
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
                        allow_volume_deletion: false,
                        services: vec![],
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
    for mode in 0..3 {
        let failure = mode == 1;
        let cancel_startup = mode == 2;
        let shared = root.join(format!("sync-{mode}"));
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
            } else if (failure || cancel_startup) && key == "beta" {
                "false"
            } else {
                "test -f /sync/alpha-started && test -f /sync/beta-started"
            };
            write(
                &root.join(format!("{key}.json")),
                serde_json::json!({"services":{key:{
                    "image":image, "entrypoint":["/bin/sh","-c"], "command":[command.replace('$',"$$")],
                    "volumes":[format!("{}:/sync",shared.display())],
                    "healthcheck":{"test":["CMD-SHELL",health],"interval":"100ms","timeout":"1s","retries":20}
                }}}),
            );
        }
        let id = format!("parallel-{}-{mode}", std::process::id());
        let events = Rc::new(RefCell::new(Vec::<orchestrator::Event>::new()));
        let sink = events.clone();
        let spec = ComposeRun {
            id: id.clone(),
            artifacts: root.join(&id),
            filesystem: None,
            recovery_anchor_dir: None,
            dora: Default::default(),
            recordings: vec![],
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
                    if cancel_startup {
                        while !shared.join("alpha-started").is_file()
                            || !shared.join("beta-started").is_file()
                        {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    } else {
                        let _ = started.await;
                    }
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
        // Assert: collection and removal retain their explicit workflow barriers,
        // including when a parallel workload failed during startup.
        let completed = |step: &str| {
            events
                .iter()
                .position(|e| {
                    e.invocation.step.ends_with(step) && e.state == orchestrator::State::Succeeded
                })
                .unwrap()
        };
        let quiesced = completed(step_id::<QuiesceCompose>());
        let finalized = completed(FinalizeRun::ID.unwrap());
        let removing = events
            .iter()
            .position(|e| {
                e.invocation.step == step_id::<RemoveCompose>()
                    && e.state == orchestrator::State::Started
            })
            .unwrap();
        assert!(quiesced < finalized && finalized < removing);
        assert!(
            events[removing]
                .invocation
                .dependencies
                .contains(&events[finalized].invocation.id)
        );
        for event in events
            .iter()
            .filter(|e| e.state == orchestrator::State::Started)
        {
            if event.invocation.step.ends_with("CollectFilesystemChanges") {
                assert_eq!(
                    event.invocation.parent,
                    Some(events[finalized].invocation.id)
                );
                assert!(event.elapsed_ns >= events[quiesced].elapsed_ns);
            }
            if event.invocation.step.ends_with("RemoveCompose") {
                assert_eq!(event.invocation.parent, None);
                assert!(event.elapsed_ns >= events[finalized].elapsed_ns);
            }
        }
        let starts: Vec<_> = events
            .iter()
            .filter(|e| {
                e.invocation.step.ends_with("StartCompose")
                    && e.state == orchestrator::State::Started
            })
            .collect();
        assert_eq!(starts.len(), 2);
        assert_eq!(
            starts[0].payload.as_ref().unwrap()["services"],
            serde_json::json!(["helper"])
        );
        assert_eq!(
            starts[1].payload.as_ref().unwrap()["services"],
            serde_json::json!(["alpha", "beta"])
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine and DORAMCAP_TEST_IMAGE"]
async fn nested_recovery_reuses_evidence_after_partial_removal() {
    use std::{os::unix::fs::DirBuilderExt, process::Command};
    use workload_docker::workflows::recovery::{RecoveryGroup, RecoveryProject, reconcile_group};

    // Arrange: two anchored projects and an unrelated container keeping one network busy.
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let parent = format!("retry-{}", std::process::id());
    let id = format!("{parent}-1");
    let root = std::env::temp_dir().join(&parent);
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let anchors = root.join("anchors");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&anchors)
        .unwrap();
    for key in ["alpha", "beta"] {
        write(
            &root.join(format!("{key}.json")),
            serde_json::json!({"services":{key:service(&image)}}),
        );
    }
    let adapter = Rc::new(DockerAdapter::connect(socket.clone()).await.unwrap());
    let mut runner = ComposeRunner::new(adapter.clone());
    let (ready, started) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(runner.run_with_ready(
        ComposeRun {
            id: id.clone(),
            artifacts: root.join(&id),
            filesystem: None,
            recovery_anchor_dir: Some(anchors.clone()),
            projects: vec![project(&root, "alpha"), project(&root, "beta")],
            first_party: vec![],
            dora: Default::default(),
            recordings: vec![],
            readiness_seconds: 10,
            stop_seconds: 3,
        },
        std::future::pending(),
        |_: &orchestrator::Event| {},
        Some(ready),
    ));
    let plans = tokio::select! {
        ready = started => vec![ready.unwrap()],
        result = &mut run => panic!("ended before readiness: {result:?}"),
    };
    drop(run);
    let policy: Vec<_> = plans
        .iter()
        .rev()
        .map(|p| RecoveryProject {
            key: p.key.clone(),
            remove_volumes: p.remove_volumes,
        })
        .collect();
    let first = plans.last().unwrap();
    let blocker = format!("{parent}-blocker");
    let docker = |args: &[&str]| {
        Command::new("docker")
            .args(["--host", &format!("unix://{socket}")])
            .args(args)
            .output()
            .unwrap()
    };
    let attached = docker(&[
        "run",
        "-d",
        "--name",
        &blocker,
        "--network",
        &format!("{}_default", first.name),
        "--entrypoint",
        "/bin/sleep",
        &image,
        "infinity",
    ]);
    assert!(
        attached.status.success(),
        "{}",
        String::from_utf8_lossy(&attached.stderr)
    );
    let group = |attempt| RecoveryGroup {
        artifacts_root: &root,
        anchors: &anchors,
        parent_run: &parent,
        policy: &policy[0],
        stop_seconds: 3,
        preserve_only: false,
        attempt,
    };

    // Act: collection succeeds, but network removal fails after its container is gone.
    let partial = reconcile_group(adapter.clone(), group(1)).await;
    let remaining = adapter.inspect(first).await.unwrap().containers;
    let detached = docker(&["rm", "-f", &blocker]);
    assert!(
        detached.status.success(),
        "{}",
        String::from_utf8_lossy(&detached.stderr)
    );

    // Assert: complete evidence survives the failed removal.
    assert!(partial.is_err());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].service, "dora-daemon");
    assert_eq!(remaining[0].pid, 0);
    assert!(directory_occupied(&root).await);
    let checkpoint = anchors.join(&id).join("outer-evidence.json");
    let saved = std::fs::read(&checkpoint).unwrap();

    // Act: a corrupt checkpoint must not authorize removal on a retry.
    std::fs::write(&checkpoint, b"invalid").unwrap();
    let corrupt = reconcile_group(adapter.clone(), group(2)).await;
    let retained = !adapter.discover_runs(&id).await.unwrap().is_empty();
    std::fs::write(&checkpoint, saved).unwrap();

    // Act: retry with the intact saved evidence, without recollecting missing containers.
    let retry = reconcile_group(adapter.clone(), group(3)).await;

    // Assert: corrupt evidence fails closed; restored evidence permits verified cleanup.
    assert!(corrupt.is_err());
    assert!(retained);
    assert!(retry.is_ok(), "{retry:?}");
    assert!(adapter.discover_runs(&id).await.unwrap().is_empty());
}

async fn directory_occupied(root: &Path) -> bool {
    use sha2::{Digest, Sha256};
    let name = format!(
        "doramcap-dir-{}",
        hex::encode(Sha256::digest(
            std::fs::canonicalize(root).unwrap().as_os_str().as_bytes()
        ))
    );
    let docker = bollard::Docker::connect_with_unix(
        &std::env::var("DORAMCAP_TEST_SOCKET").unwrap(),
        120,
        bollard::API_DEFAULT_VERSION,
    )
    .unwrap();
    match docker.inspect_container(&name, None).await {
        Ok(_) => true,
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => false,
        Err(error) => panic!("inspect directory owner: {error}"),
    }
}
