//! Explicitly opt-in real Engine validation. The harness builds unprivileged and
//! elevates only this executable. Every scenario owns distinct named resources.
mod support;
use orchestrator::Step;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc, time::Duration};
use workload_docker::workflows::recorded_compose::finalization::FinalizeRun;
use workload_docker::workflows::{
    dora::DoraRuntime,
    experiment::{RegisterDoraGraph, StartDoraRuntime, StopDoraRuntime},
};
use workload_docker::{compose::*, workflows::recorded_compose::*};
use workload_lifecycle::recording::Recording;

fn step_id<S: Step>() -> &'static str {
    S::ID.unwrap_or(std::any::type_name::<S>())
}

fn project(root: &Path, key: &str) -> ComposeProject {
    ComposeProject {
        key: key.into(),
        directory: root.to_str().unwrap().into(),
        files: vec![format!("{key}.json")],
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
        directory: root.to_str().unwrap().into(),
        files: vec![file.to_str().unwrap().into()],
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
        repository_root: root.to_path_buf(),
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
        system_apps: vec![monitor(root, image, &root.join(&id))],
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
        if message.channel.topic == "doramcap/cpuacct" {
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
        repository_root: root.to_path_buf(),
        projects: vec![project(&root, "demo")],
        system_apps: vec![monitor(&root, &image, &artifacts)],
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
        .filter(|message| message.channel.topic == "doramcap/cpuacct")
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
            request.system_apps.push(project(&root, "auxiliary"));
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
        assert!(
            adapter
                .project(&beta)
                .inspect()
                .await
                .unwrap()
                .containers
                .is_empty()
        );
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
        let repeated = StopCompose {
            adapter: adapter.clone(),
            diagnostics: Default::default(),
        }
        .run(StopInput {
            allow_volume_deletion: true,
            services: vec![],
            plan: Some(beta),
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
        serde_json::from_slice::<overlay::OverlaySet>(
            &std::fs::read(root.join(&report.id).join("overlays/prepared.json")).unwrap()
        )
        .unwrap()
        .trees[0]
            .upper
            .join("busy-result")
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
async fn direct_session_lifecycle_and_prerun_reset() {
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
            repository_root: root.to_path_buf(),
            projects: vec![project(&root, "session")],
            system_apps: vec![],
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
            spec.clone(),
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
        assert!(
            adapter
                .project(&plans[0])
                .inspect()
                .await
                .unwrap()
                .ready()
                .unwrap()
        );
        assert!(!artifacts.join("overlays").exists());
        assert!(!artifacts.join("system-services.yaml").exists());
        if mode == 2 {
            // Act: skip teardown, then let the next workflow reset the old project.
            drop(run);
            assert!(runner.blocked());
            for attempt in 0..2 {
                let mut probe = spec.clone();
                probe.id = format!("reset-{}-{attempt}", std::process::id());
                probe.artifacts = root.join(&probe.id);
                reset_probe(adapter.clone(), probe).await.unwrap();
            }
            // Assert: reset is repeatable and preserves the previous output directory.
            assert!(artifacts.exists());
        } else {
            // Act: cooperatively stop the ready session.
            if mode == 0 {
                stop.send(()).unwrap();
            } else {
                QuiesceCompose {
                    adapter: adapter.clone(),
                    diagnostics: Default::default(),
                }
                .run(StopInput {
                    allow_volume_deletion: false,
                    services: vec![],
                    plan: Some(plans[0].clone()),
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
                .project(&plans[0])
                .inspect()
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
            repository_root: root.to_path_buf(),
            dora: Default::default(),
            recordings: vec![],
            system_apps: vec![project(&root, "helper")],
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
        support::assert_no_run_resources(&adapter, &id).await;
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

async fn reset_probe(
    adapter: Rc<DockerAdapter>,
    input: ComposeRun,
) -> std::result::Result<(), String> {
    let result = ComposeRunner::new(adapter)
        .run_with_preflight(
            input,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |_| {
                Err(workload_docker::compose::Error::Invalid(
                    "reset-probe-complete".into(),
                ))
            },
        )
        .await;
    match result {
        Err(workload_docker::compose::Error::Invalid(message))
            if message == "reset-probe-complete" =>
        {
            Ok(())
        }
        other => Err(format!("{other:?}")),
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine and the prepared Dora image"]
async fn reset_failure_blocks_preparation_and_retries_after_partial_removal() {
    // Arrange: a live experiment and a test container keeping its network busy.
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let root = std::env::temp_dir().join(format!("reset-partial-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    write(
        &root.join("session.json"),
        serde_json::json!({"services":{"app":service(&image)}}),
    );
    let adapter = Rc::new(DockerAdapter::connect(socket.clone()).await.unwrap());
    let original = ComposeRun {
        id: format!("partial-{}", std::process::id()),
        artifacts: root.join("first"),
        repository_root: root.clone(),
        filesystem: None,
        projects: vec![project(&root, "session")],
        system_apps: vec![],
        dora: Default::default(),
        recordings: vec![],
        readiness_seconds: 15,
        stop_seconds: 3,
    };
    let mut runner = ComposeRunner::new(adapter.clone());
    let (ready, started) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(runner.run_with_ready(
        original.clone(),
        std::future::pending(),
        |_: &orchestrator::Event| {},
        Some(ready),
    ));
    let plan = tokio::select! { ready = started => ready.unwrap(), result = &mut run => panic!("startup failed: {result:?}") };
    let blocker = format!("reset-blocker-{}", std::process::id());
    let docker = |args: &[&str]| {
        std::process::Command::new("docker")
            .args(["--host", &format!("unix://{socket}")])
            .args(args)
            .output()
            .unwrap()
    };
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&plan.manifest).unwrap()).unwrap();
    let network = manifest["networks"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["name"]
        .as_str()
        .unwrap();
    let attached = docker(&[
        "run",
        "-d",
        "--name",
        &blocker,
        "--network",
        network,
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
    drop(run);
    let mut probe = original.clone();
    probe.id.push_str("-next");
    probe.artifacts = root.join("next");
    // Act: cleanup cannot remove the network while the unrelated container uses it.
    let failed = reset_probe(adapter.clone(), probe.clone()).await;
    // Assert: reset failure prevents preparation, even though some containers were removed.
    assert!(failed.is_err());
    assert!(!probe.artifacts.exists());
    assert!(original.artifacts.exists());
    let remaining = adapter.project(&plan).inspect().await.unwrap();
    assert!(remaining.containers.iter().all(|c| c.pid == 0));
    assert!(
        remaining
            .containers
            .iter()
            .any(|c| c.service == "dora-daemon")
    );
    // Act: release the test blocker and retry the same next request.
    assert!(docker(&["rm", "-f", &blocker]).status.success());
    reset_probe(adapter.clone(), probe).await.unwrap();
    // Assert: the old project is fully removed; its output directory remains.
    assert!(
        adapter
            .project(&plan)
            .inspect()
            .await
            .unwrap()
            .containers
            .is_empty()
    );
    assert!(original.artifacts.exists());
    // Act / Assert: a clean previous teardown is also harmless.
    let mut clean = original;
    clean.id.push_str("-clean");
    clean.artifacts = root.join("clean");
    reset_probe(adapter, clean).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine and the prepared Dora image"]
async fn stable_project_keeps_retained_volumes_isolated_between_experiments() {
    // Arrange
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let root = std::env::temp_dir().join(format!("stable-volumes-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut app = service(&image);
    app["command"] = serde_json::json!([
        "test ! -e /scratch/marker || exit 44; touch /scratch/marker; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    app["volumes"] = serde_json::json!(["scratch:/scratch"]);
    write(
        &root.join("session.json"),
        serde_json::json!({"services":{"app":app},"volumes":{"scratch":{}}}),
    );
    let adapter = Rc::new(DockerAdapter::connect(socket.clone()).await.unwrap());
    let mut runner = ComposeRunner::new(adapter.clone());
    let mut projects = Vec::new();
    let mut volumes = Vec::new();
    for index in 0..2 {
        let id = format!("retained-{}-{index}", std::process::id());
        let mut project = project(&root, "session");
        project.remove_volumes = false;
        let input = ComposeRun {
            id: id.clone(),
            artifacts: root.join(id),
            repository_root: root.clone(),
            filesystem: None,
            projects: vec![project],
            system_apps: vec![],
            dora: Default::default(),
            recordings: vec![],
            readiness_seconds: 15,
            stop_seconds: 3,
        };
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        // Act
        let mut run = Box::pin(runner.run_with_ready(
            input.clone(),
            async {
                let _ = stopped.await;
            },
            |_: &orchestrator::Event| {},
            Some(ready),
        ));
        let plan = tokio::select! { plan = started => plan.unwrap(), result = &mut run => panic!("startup failed: {result:?}") };
        projects.push(plan.name);
        stop.send(()).unwrap();
        let report = run.await.unwrap();
        // Assert: neither the old marker nor the old volume is reused.
        assert!(report.boundary_clean, "{report:?}");
        assert!(report.execution_error.is_none(), "{report:?}");
        assert_eq!(report.cleanup.retained_volumes.len(), 1);
        volumes.push(report.cleanup.retained_volumes[0].clone());

        // Act: inspect the same run while its retained volume still exists.
        let mut inspection = input;
        inspection.artifacts = root.join(format!("inspection-{index}"));
        let inspected = preparation::resolve(&adapter, inspection).await.unwrap();

        // Assert
        assert_eq!(inspected.plan.name, projects[index]);
        assert_eq!(
            inspected.plan.run,
            format!("retained-{}-{index}", std::process::id())
        );
    }
    assert_eq!(projects[0], projects[1]);
    assert_ne!(volumes[0], volumes[1]);
    for volume in volumes {
        assert!(
            std::process::Command::new("docker")
                .args([
                    "--host",
                    &format!("unix://{socket}"),
                    "volume",
                    "rm",
                    &volume
                ])
                .status()
                .unwrap()
                .success()
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine and the prepared Dora image"]
async fn active_experiments_are_exclusive_per_repository_and_independent_between_repositories() {
    // Arrange
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let root = std::env::temp_dir().join(format!("repository-experiments-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let repositories: Vec<_> = ["first", "second"]
        .iter()
        .map(|name| {
            let path = root.join(name);
            std::fs::create_dir(&path).unwrap();
            write(
                &path.join("session.json"),
                serde_json::json!({"services":{"app":service(&image)}}),
            );
            path
        })
        .collect();
    let input = |repository: &Path, id: &str| ComposeRun {
        id: id.into(),
        artifacts: repository.join(id),
        repository_root: repository.to_owned(),
        filesystem: None,
        projects: vec![project(repository, "session")],
        system_apps: vec![],
        dora: Default::default(),
        recordings: vec![],
        readiness_seconds: 15,
        stop_seconds: 3,
    };
    let adapter = Rc::new(DockerAdapter::connect(socket).await.unwrap());
    let mut runner = ComposeRunner::new(adapter.clone());
    let (ready, started) = tokio::sync::oneshot::channel();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(runner.run_with_ready(
        input(&repositories[0], "active"),
        async {
            let _ = stopped.await;
        },
        |_: &orchestrator::Event| {},
        Some(ready),
    ));
    let plan = tokio::select! { plan = started => plan.unwrap(), result = &mut run => panic!("startup failed: {result:?}") };
    // Act / Assert: a sibling run cannot mutate the active repository.
    let blocked = input(&repositories[0], "blocked");
    assert!(reset_probe(adapter.clone(), blocked.clone()).await.is_err());
    assert!(!blocked.artifacts.exists());

    // Act: run the second repository while the first remains active.
    let mut other_runner = ComposeRunner::new(adapter.clone());
    let (other_ready, other_started) = tokio::sync::oneshot::channel();
    let (other_stop, other_stopped) = tokio::sync::oneshot::channel();
    let mut other_run = Box::pin(other_runner.run_with_ready(
        input(&repositories[1], "independent"),
        async {
            let _ = other_stopped.await;
        },
        |_: &orchestrator::Event| {},
        Some(other_ready),
    ));
    let other_plan = tokio::select! { ready = other_started => ready.unwrap(), result = &mut other_run => panic!("second startup failed: {result:?}") };

    // Assert: both produced projects and their real Docker containers are independent.
    let first_state = adapter.project(&plan).inspect().await.unwrap();
    let other_state = adapter.project(&other_plan).inspect().await.unwrap();
    assert!(first_state.ready().unwrap());
    assert!(other_state.ready().unwrap());
    assert_ne!(plan.name, other_plan.name);
    assert!(first_state.containers.iter().all(|first| {
        other_state
            .containers
            .iter()
            .all(|other| first.id != other.id)
    }));

    // Act: stop and reset the second repository while the first keeps running.
    other_stop.send(()).unwrap();
    assert!(other_run.await.unwrap().boundary_clean);
    reset_probe(
        adapter.clone(),
        input(&repositories[1], "reset-independent"),
    )
    .await
    .unwrap();

    // Assert: cleanup never touches the first repository's containers or lock.
    let after = adapter.project(&plan).inspect().await.unwrap();
    assert!(after.ready().unwrap());
    assert_eq!(
        first_state
            .containers
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>(),
        after.containers.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    assert!(
        reset_probe(adapter.clone(), input(&repositories[0], "still-blocked"))
            .await
            .is_err()
    );

    // Act / Assert: normal teardown releases admission for the same repository.
    stop.send(()).unwrap();
    assert!(run.await.unwrap().boundary_clean);
    reset_probe(adapter.clone(), input(&repositories[0], "accepted"))
        .await
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker, OverlayFS capabilities and the prepared Dora image"]
async fn reset_discards_fixed_workspace_without_launch_history() {
    // Arrange: leave a live experiment and its isolated writes without teardown.
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("original"), b"unchanged").unwrap();
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let mut app = service(&image);
    app["volumes"] = serde_json::json!([{"type":"bind","source":source,"target":"/data"}]);
    app["command"] = serde_json::json!([
        "printf unfinished > /data/result; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    app["healthcheck"] = serde_json::json!({"test":["CMD","test","-s","/data/result"],"interval":"200ms","timeout":"1s","retries":30});
    write(
        &root.path().join("app.json"),
        serde_json::json!({"services":{"app":app}}),
    );
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let mut runner = ComposeRunner::new(adapter.clone());
    let original = ComposeRun {
        id: "abandoned".into(),
        artifacts: root.path().join("first"),
        repository_root: root.path().into(),
        filesystem: Some(Default::default()),
        projects: vec![project(root.path(), "app")],
        system_apps: vec![],
        dora: Default::default(),
        recordings: vec![],
        readiness_seconds: 15,
        stop_seconds: 3,
    };
    let (ready, started) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(runner.run_with_ready(
        original.clone(),
        std::future::pending(),
        |_: &orchestrator::Event| {},
        Some(ready),
    ));
    let plan = tokio::select! { plan=started=>plan.unwrap(), result=&mut run=>panic!("startup failed: {result:?}") };
    let records = original.artifacts.join("overlays");
    let set: overlay::OverlaySet =
        serde_json::from_slice(&std::fs::read(records.join("prepared.json")).unwrap()).unwrap();
    std::fs::write(original.artifacts.join("user-output"), b"retain").unwrap();
    assert!(!source.join("result").exists());
    assert!(set.trees[0].upper.join("result").exists());
    drop(run);
    // Remove old launch metadata; reset must address the ownership scope directly.
    std::fs::remove_file(&plan.manifest).unwrap();
    std::fs::remove_file(original.artifacts.join("experiment.resolution.json")).unwrap();
    let other_repository = root.path().join("other-repository");
    std::fs::create_dir(&other_repository).unwrap();
    // Act / Assert: a different repository's reset leaves the abandoned run untouched.
    reset_probe(
        adapter.clone(),
        ComposeRun {
            id: "other-repository".into(),
            artifacts: root.path().join("other-artifacts"),
            repository_root: other_repository,
            filesystem: None,
            ..original.clone()
        },
    )
    .await
    .unwrap();
    assert!(set.trees[0].upper.join("result").exists());
    assert!(
        adapter
            .project(&plan)
            .inspect()
            .await
            .unwrap()
            .ready()
            .unwrap()
    );
    for attempt in 0..2 {
        let request = ComposeRun {
            id: format!("next-{attempt}"),
            artifacts: root.path().join(format!("next-{attempt}")),
            repository_root: root.path().to_owned(),
            filesystem: None,
            ..original.clone()
        };
        // Act: reset the owning repository, then repeat on its empty workspace.
        reset_probe(adapter.clone(), request).await.unwrap();
        // Assert: only the fixed mutable workspace and owned Docker resources disappear.
        assert!(!set.workspace.exists());
        assert!(
            adapter
                .project(&plan)
                .inspect()
                .await
                .unwrap()
                .containers
                .is_empty()
        );
        assert_eq!(
            std::fs::read(source.join("original")).unwrap(),
            b"unchanged"
        );
        assert!(!source.join("result").exists());
        assert_eq!(
            std::fs::read(original.artifacts.join("user-output")).unwrap(),
            b"retain"
        );
        assert!(records.join("prepared.json").is_file());
        assert!(set.trees[0].baseline.is_file());
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine, Dora CLI and built node binaries"]
async fn public_dora_records_and_stops_system_apps() {
    compose_public_run("complete").await;
    compose_public_run("timer-consumer").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine, Dora CLI and built node binaries"]
async fn public_dora_cancels_partial_startup() {
    compose_public_run("cancel").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine, Dora CLI and built node binaries"]
async fn public_dora_cleans_failed_container_start() {
    compose_public_run("failed").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and Dora images"]
async fn public_dora_cleans_up_after_daemon_death() {
    compose_public_run("daemon-death").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine, Dora and application binaries"]
async fn public_dora_marks_forced_writer_shutdown_incomplete() {
    compose_public_run("stalled").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "internal crash child; invoked by public_dora_resets_after_launcher_crash"]
async fn public_dora_crash_child() {
    if std::env::var_os("DORAMCAP_DORA_CRASH_ROOT").is_some() {
        let stage = std::env::var("DORAMCAP_DORA_CRASH_STAGE").unwrap();
        compose_public_run(&stage).await;
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker Engine, Dora and application binaries"]
async fn public_dora_resets_after_launcher_crash() {
    use std::os::unix::process::ExitStatusExt;
    for stage in ["crash-startup", "crash", "crash-removal"] {
        // Arrange: kill the launcher at each lifecycle boundary without teardown.
        let root =
            std::env::temp_dir().join(format!("compose-dora-{}-{stage}", std::process::id()));
        let launcher = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "public_dora_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("DORAMCAP_DORA_CRASH_ROOT", &root)
            .env("DORAMCAP_DORA_CRASH_STAGE", stage)
            .status()
            .unwrap();
        assert_eq!(launcher.signal(), Some(libc::SIGKILL));
        let run = std::fs::read_to_string(root.join("run-id")).unwrap();
        let artifacts = root.join(&run);
        let plan: ResolvedComposeProject = serde_json::from_value(
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(artifacts.join("experiment.resolution.json")).unwrap(),
            )
            .unwrap()["plan"]
                .clone(),
        )
        .unwrap();
        let adapter = Rc::new(
            DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
                .await
                .unwrap(),
        );
        assert!(
            !adapter
                .project(&plan)
                .inspect()
                .await
                .unwrap()
                .containers
                .is_empty()
        );
        let recording = root.join("results/recording.mcap");
        if stage != "crash-startup" {
            assert!(recording.is_file());
        }
        assert!(!artifacts.join("report.json").exists());
        // Act: the next workflow resets the previous run, including a repeated clean reset.
        for attempt in 0..2 {
            reset_probe(
                adapter.clone(),
                ComposeRun {
                    id: format!("after-crash-{attempt}"),
                    artifacts: root.join(format!("next-{attempt}")),
                    repository_root: root.clone(),
                    filesystem: None,
                    projects: vec![project(&root, "adopted")],
                    system_apps: vec![],
                    dora: Default::default(),
                    recordings: vec![],
                    readiness_seconds: 15,
                    stop_seconds: 3,
                },
            )
            .await
            .unwrap();
            // Assert: old processes are gone and interrupted output remains unpublished.
            assert!(
                adapter
                    .project(&plan)
                    .inspect()
                    .await
                    .unwrap()
                    .containers
                    .is_empty()
            );
            if stage != "crash-startup" {
                assert!(recording.is_file());
            }
            assert!(!artifacts.join("report.json").exists());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

async fn compose_public_run(outcome: &str) {
    use prost::Message;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // Arrange
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let socket = std::env::var("DORAMCAP_TEST_SOCKET").unwrap();
    let fixture = std::env::var("DORAMCAP_TEST_DORA_NODE").unwrap();
    let recorder = std::env::var("DORAMCAP_TEST_MCAP_RECORDER").unwrap();
    let root = std::env::var_os("DORAMCAP_DORA_CRASH_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "mixed-dora-stages-{}-{outcome}",
                std::process::id()
            ))
        });
    std::fs::create_dir(&root).unwrap();
    let runtime = root.join("runtime");
    let results = root.join("results");
    for path in [&runtime, &results] {
        std::fs::create_dir(path).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let owner = std::fs::metadata(&root).unwrap();
    let user = format!("{}:{}", owner.uid(), owner.gid());
    let coordinator = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let daemon = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let coordinator_port = coordinator.local_addr().unwrap().port();
    let daemon_port = daemon.local_addr().unwrap().port();
    drop((coordinator, daemon));
    let graph = root.join("graph.yml");
    let mut flow = "nodes:\n  - id: source\n    path: dynamic\n    outputs: [data]\n  - id: observer\n    path: dynamic\n    inputs:\n      data: source/data\n      tick: dora/timer/millis/100\n  - id: recorder\n    path: dynamic\n    inputs:\n      snapshots: {source: source/data, queue_size: 64, queue_policy: backpressure}\n".to_string();
    if outcome == "complete" {
        flow = flow.replace("      tick: dora/timer/millis/100\n", "");
    }
    if outcome == "timer-consumer" {
        flow.push_str("      tick: dora/timer/millis/100\n");
    }
    std::fs::write(&graph, flow).unwrap();
    let packet = telemetry_protocol::pb::Packet {
        body: Some(telemetry_protocol::pb::packet::Body::Snapshot(
            telemetry_protocol::pb::SnapshotFrame {
                version: telemetry_protocol::VERSION,
                snapshot: Some(telemetry_protocol::pb::Snapshot {
                    schema_version: 1,
                    sequence: 1,
                    capture_unix_ns: 123,
                    ..Default::default()
                }),
                source: "fixture".into(),
                session_id: "mixed-test".into(),
                ..Default::default()
            },
        )),
    };
    let payload = packet.encode_to_vec();
    let payload_file = root.join("payload.pb");
    std::fs::write(&payload_file, &payload).unwrap();
    let recorder_config = root.join("recorder.pbtxt");
    std::fs::write(
        &recorder_config,
        include_str!("../../../apps/mcap-recorder/deploy/reusable/cpu.pbtxt")
            .replace(
                "/results/cpu.mcap",
                results.join("recording.mcap").to_str().unwrap(),
            )
            .replace(
                "/results/cpu.report.pbtxt",
                results.join("recording.report.pbtxt").to_str().unwrap(),
            ),
    )
    .unwrap();
    let run = format!("mixed-{}", std::process::id());
    std::fs::write(root.join("run-id"), &run).unwrap();
    let artifacts = root.join(&run);
    write(
        &root.join("session.json"),
        serde_json::json!({"services":{
            "source": {
                "depends_on": {"recorder": {"condition": "service_healthy"}},
                "image": image, "network_mode":"host", "user":user, "shm_size":"64m",
                "ulimits":{"memlock":{"soft":67108864,"hard":67108864}},
                "entrypoint":["/fixture"],
                "command":["source", daemon_port.to_string(), runtime.join("source.ready").to_str().unwrap(),
                    if outcome == "failed" {"invalid"} else {"producer"}, "/payload.pb", runtime.join("source.done").to_str().unwrap()],
                "volumes":[
                    {"type":"bind", "source":fixture, "target":"/fixture", "read_only":true},
                    {"type":"bind", "source":payload_file, "target":"/payload.pb", "read_only":true},
                    {"type":"bind", "source":runtime, "target":runtime}
                ]
            },
            "observer": {
                "image":image, "network_mode":"host", "user":user,
                "entrypoint":["/fixture"],
                "command":["observer", daemon_port.to_string(), runtime.join("observer.ready").to_str().unwrap(), "idle"],
                "volumes":[
                    {"type":"bind", "source":fixture, "target":"/fixture", "read_only":true},
                    {"type":"bind", "source":runtime, "target":runtime}
                ]
            },
            "recorder": {
                "healthcheck": {"test": ["CMD", "test", "-s", runtime.join("recorder.ready")], "interval": "200ms", "timeout": "1s", "retries": 75},
                "image":image, "network_mode":"host", "user":user, "shm_size":"64m",
                "ulimits":{"memlock":{"soft":67108864,"hard":67108864}},
                "security_opt":["seccomp=unconfined"],
                "entrypoint":["/recorder"],
                "command":["--config", "/config/recorder.pbtxt"],
                "environment":{"DORAMCAP_READY_FILE":runtime.join("recorder.ready")},
                "volumes":[
                    {"type":"bind", "source":recorder, "target":"/recorder", "read_only":true},
                    {"type":"bind", "source":recorder_config, "target":"/config/recorder.pbtxt", "read_only":true},
                    {"type":"bind", "source":runtime, "target":runtime},
                    {"type":"bind", "source":results, "target":results}
                ]
            }
        }}),
    );
    let dora = DoraRuntime {
        coordinator_port: coordinator_port.into(),
        daemon_port: daemon_port.into(),
        graph: graph.to_str().unwrap().into(),
        runtime_directory: runtime.to_str().unwrap().into(),
        participant_uid: Some(owner.uid()),
        participant_gid: Some(owner.gid()),
        ..Default::default()
    };
    let mut adopted = service(&image);
    adopted["command"] = serde_json::json!([
        "test \"$$DORA_NODE_ID\" = app || exit 43; test -s /runtime/source.ready && test -s /runtime/recorder.ready && test -s /runtime/support.ready || exit 42; touch /runtime/adopted.started; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    if outcome == "stalled" {
        adopted["command"][0] = adopted["command"][0]
            .as_str()
            .unwrap()
            .replace("trap 'exit 0' TERM INT", "trap '' TERM INT")
            .into();
    }
    adopted["volumes"] =
        serde_json::json!([{"type":"bind", "source":runtime, "target":"/runtime"}]);
    write(
        &root.join("adopted.json"),
        serde_json::json!({"services":{"app":adopted}}),
    );
    let mut support = service(&image);
    support["command"] = serde_json::json!([
        "sleep 1; printf ready > /runtime/support.ready; trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"
    ]);
    support["volumes"] =
        serde_json::json!([{"type":"bind", "source":runtime, "target":"/runtime"}]);
    support["healthcheck"] = serde_json::json!({"test":["CMD", "test", "-s", "/runtime/support.ready"], "interval":"200ms", "timeout":"1s", "retries":75});
    write(
        &root.join("support.json"),
        serde_json::json!({"services":{"support":support}}),
    );
    let spec = ComposeRun {
        id: run.clone(),
        artifacts: artifacts.clone(),
        filesystem: None,
        repository_root: root.clone(),
        projects: vec![project(&root, "adopted")],
        system_apps: vec![project(&root, "session"), project(&root, "support")],
        dora,
        recordings: vec![],
        readiness_seconds: 15,
        stop_seconds: 5,
    };
    let adapter = Rc::new(DockerAdapter::connect(socket).await.unwrap());
    let events = Rc::new(RefCell::new(Vec::<orchestrator::Event>::new()));
    let sink = events.clone();
    let mut runner = ComposeRunner::new(adapter.clone());
    let done = runtime.join("source.done");
    let (ready, mut ready_rx) = tokio::sync::oneshot::channel::<ResolvedComposeProject>();
    let ready_seen = std::cell::Cell::new(false);

    // Act: the public workflow owns startup, user stop, system stop and cleanup.
    let crash_startup = outcome == "crash-startup";
    let crash_removal = outcome == "crash-removal";
    let report = Box::pin(runner.run_with_ready(
        spec,
        async {
            if outcome == "cancel" {
                tokio::time::sleep(Duration::from_millis(100)).await;
                return;
            }
            let Ok(plan) = (&mut ready_rx).await else {
                return std::future::pending::<()>().await;
            };
            let plans = [plan];
            if outcome == "daemon-death" {
                let state = adapter.project(&plans[0]).inspect().await.unwrap();
                let daemon = state
                    .containers
                    .iter()
                    .find(|c| c.service == "dora-daemon")
                    .unwrap();
                assert!(
                    tokio::process::Command::new("docker")
                        .args([
                            "--host",
                            &format!("unix://{}", adapter.socket_path().display()),
                            "kill",
                            "--signal",
                            "KILL",
                            &daemon.id
                        ])
                        .status()
                        .await
                        .unwrap()
                        .success()
                );
                return std::future::pending::<()>().await;
            }
            if outcome == "crash" {
                // SAFETY: terminate this isolated test child without destructors.
                unsafe {
                    libc::raise(libc::SIGKILL);
                }
                unreachable!();
            }
            ready_seen.set(true);
            let _ = tokio::time::timeout(Duration::from_secs(30), async {
                while !done.is_file() {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await;
        },
        move |event: &orchestrator::Event| {
            if (crash_startup
                && event.invocation.step == step_id::<RegisterDoraGraph>()
                && event.state == orchestrator::State::Succeeded)
                || (crash_removal
                    && event.invocation.step == step_id::<RemoveCompose>()
                    && event.state == orchestrator::State::Started)
            {
                // SAFETY: terminate this isolated test child at the selected boundary.
                unsafe {
                    libc::raise(libc::SIGKILL);
                }
            }
            if event.invocation.step == "compose.prepare"
                && event.state == orchestrator::State::Succeeded
            {
                std::fs::write(&payload_file, b"changed after preparation").unwrap();
            }
            sink.borrow_mut().push(event.clone());
        },
        Some(ready),
    ))
    .await;

    // Assert
    let report = report.expect("public workflow report");
    let complete = matches!(outcome, "complete" | "timer-consumer");
    assert_eq!(report.successful(), complete, "{report:?}");
    assert_eq!(report.recording_finalized, complete, "{report:?}");
    if complete {
        assert!(ready_seen.get());
    }
    assert!(!runner.blocked(), "{report:?}");
    support::assert_no_run_resources(&adapter, &run).await;
    if !complete {
        if outcome == "stalled" {
            assert!(
                report.writers_stopped && report.boundary_clean,
                "{report:?}"
            );
            assert!(
                !report.diagnostics.is_empty(),
                "forced stop must retain diagnostics"
            );
            assert!(
                results.join("recording.mcap").is_file(),
                "recording files must be retained"
            );
        }
        if outcome == "cancel" || outcome == "failed" {
            assert!(!runtime.join("adopted.started").exists());
        }
        std::fs::remove_dir_all(root).unwrap();
        return;
    }
    {
        let events = events.borrow();
        let succeeded = |suffix: &str| {
            events
                .iter()
                .position(|event| {
                    event.invocation.step == suffix && event.state == orchestrator::State::Succeeded
                })
                .expect(suffix)
        };
        assert!(
            succeeded(step_id::<StartDoraRuntime>()) < succeeded(step_id::<RegisterDoraGraph>())
        );
        let starts: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.invocation.step == step_id::<StartCompose>()
                    && e.state == orchestrator::State::Succeeded
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(starts.len(), 2);
        assert!(succeeded(step_id::<StartDoraRuntime>()) < starts[0]);
        assert!(starts[0] < starts[1]);
        assert!(runtime.join("adopted.started").is_file());
        assert!(succeeded(step_id::<StopDoraRuntime>()) < succeeded(step_id::<FinalizeRun>()));
    }
    let recording = std::fs::read(results.join("recording.mcap")).ok();
    // Assert: accepted messages are present in a finalized, readable MCAP.
    let recording = recording.expect("finalized recording");
    mcap::read::footer(&recording).unwrap();
    let messages: Vec<_> = mcap::MessageStream::new(&recording)
        .unwrap()
        .map(|message| message.unwrap())
        .collect();
    assert_eq!(messages.len(), 20);
    assert!(messages.iter().map(|message| message.sequence).eq(0..20));
    let mut expected = telemetry_protocol::pb::Packet::decode(payload.as_slice()).unwrap();
    for (sequence, message) in messages.iter().enumerate() {
        let Some(telemetry_protocol::pb::packet::Body::Snapshot(frame)) = expected.body.as_mut()
        else {
            panic!("expected a snapshot fixture");
        };
        frame.snapshot.as_mut().unwrap().sequence = sequence as u64;
        assert_eq!(message.data.as_ref(), expected.encode_to_vec().as_slice());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and prebuilt images; creates no containers"]
async fn inspection_accepts_occupied_fixed_endpoints_but_launch_preparation_rejects_them() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let listeners = [
        std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap(),
        std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap(),
    ];
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    write(
        &root.path().join("worker.json"),
        serde_json::json!({"services":{"worker":service(&image)}}),
    );
    let spec = ComposeRun {
        id: "occupied-ports".into(),
        artifacts: root.path().join("inspection"),
        repository_root: root.path().into(),
        filesystem: None,
        system_apps: vec![project(root.path(), "worker")],
        projects: vec![],
        dora: DoraRuntime {
            coordinator_port: listeners[0].local_addr().unwrap().port().into(),
            daemon_port: listeners[1].local_addr().unwrap().port().into(),
            ..Default::default()
        },
        recordings: vec![],
        readiness_seconds: 10,
        stop_seconds: 3,
    };
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );

    // Act
    let inspected = preparation::resolve(&adapter, spec.clone()).await.unwrap();

    // Assert
    assert_eq!(inspected.dora.coordinator_port, spec.dora.coordinator_port);
    assert_eq!(inspected.dora.daemon_port, spec.dora.daemon_port);

    // Act
    let mut execution = spec.clone();
    execution.artifacts = root.path().join("execution");
    let mut runner = ComposeRunner::new(adapter.clone());
    let result = runner
        .run_with_preflight(
            execution,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |_| panic!("occupied endpoints must fail before preflight acceptance"),
        )
        .await;

    // Assert
    assert!(result.unwrap_err().to_string().contains("Dora endpoint"));
    assert!(!runner.blocked());
    support::assert_no_run_resources(&adapter, &spec.id).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and prebuilt images; creates no containers"]
async fn independent_inspection_and_preparation_have_equal_configuration_snapshots() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    // Use the shipped app input so runtime wiring evolves with its real consumer.
    std::fs::write(
        root.path().join("app.yaml"),
        include_bytes!("../../../apps/mcap-recorder/deploy/compose.yaml"),
    )
    .unwrap();
    let app = |artifacts: &Path| ComposeProject {
        key: "recorder".into(),
        directory: root.path().to_string_lossy().into_owned(),
        files: vec!["app.yaml".into()],
        environment: BTreeMap::from([
            ("MCAP_RECORDER_UID".into(), "1000".into()),
            ("MCAP_RECORDER_GID".into(), "1000".into()),
            (
                "MCAP_RECORDER_RESULTS_DIR".into(),
                artifacts.to_string_lossy().into_owned(),
            ),
        ]),
        ..Default::default()
    };
    let spec = ComposeRun {
        id: "inspected".into(),
        artifacts: root.path().join("inspection"),
        filesystem: None,
        repository_root: root.path().into(),
        system_apps: vec![app(&root.path().join("inspection"))],
        projects: vec![],
        dora: Default::default(),
        recordings: vec![],
        readiness_seconds: 10,
        stop_seconds: 3,
    };
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let executor = Rc::new_cyclic(|weak| {
        let mut executor = orchestrator::LocalExecutor::default();
        workload_docker::workflows::steps::register(
            &mut executor,
            adapter.clone(),
            Default::default(),
        );
        let backend = adapter.clone();
        let weak = weak.clone();
        executor.register::<preparation::PrepareRun>(move || preparation::PrepareRun {
            adapter: backend.clone(),
            executor: weak.clone(),
        });
        executor
    });

    // Act
    let inspected = preparation::resolve(&adapter, spec.clone()).await.unwrap();
    let mut execution = spec.clone();
    execution.id = "prepared".into();
    execution.artifacts = root.path().join("execution");
    execution.system_apps = vec![app(&execution.artifacts)];
    let prepared = executor
        .run::<preparation::PrepareRun>(execution)
        .await
        .unwrap()
        .into_value();

    // Assert
    assert_ne!(
        inspected.dora.runtime_directory,
        prepared.dora.runtime_directory
    );
    assert_eq!(inspected.snapshot, prepared.snapshot);

    // Act: a changed runtime budget must invalidate the inspected configuration.
    let mut changed = spec.clone();
    changed.artifacts = root.path().join("changed");
    changed.system_apps = vec![app(&changed.artifacts)];
    changed.readiness_seconds += 1;
    let changed = preparation::resolve(&adapter, changed).await.unwrap();

    // Assert
    assert_ne!(inspected.snapshot, changed.snapshot);
    support::assert_no_run_resources(&adapter, &spec.id).await;
    support::assert_no_run_resources(&adapter, "prepared").await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and prebuilt images; creates no containers"]
async fn shared_experiment_preserves_native_inputs_and_rejects_conflicts() {
    // Arrange
    let root = std::env::temp_dir().join(format!("experiment-inputs-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    std::fs::write(root.join("settings.pbtxt"), b"mode: original\n").unwrap();
    std::fs::write(root.join("inputs.env"), b"SETTING=from-file\n").unwrap();
    write(
        &root.join("first.json"),
        serde_json::json!({"services":{"monitor":{
            "image":image,"network_mode":"host","environment":{"SETTING":"${SETTING:?}","LITERAL":"$$literal"},
            "volumes":["./settings.pbtxt:/etc/settings.pbtxt:ro"],
            "healthcheck":{"test":["CMD","test","-s","/etc/settings.pbtxt"],"interval":"100ms"}
        }}}),
    );
    write(
        &root.join("override.json"),
        serde_json::json!({"services":{"monitor":{"command":["kept","$$literal"]}}}),
    );
    write(
        &root.join("user.json"),
        serde_json::json!({"services":{"adopted":{"image":image,"networks":["users"],"volumes":["data:/data"],"environment":{"DORA_NODE_ID":"app-chosen"}}},"networks":{"users":{}},"volumes":{"data":{}}}),
    );
    let mut first = project(&root, "first");
    first.files.push("override.json".into());
    first.env_files.push("inputs.env".into());
    first
        .environment
        .insert("SETTING".into(), "explicit".into());
    let spec = ComposeRun {
        id: format!("inputs-{}", std::process::id()),
        artifacts: root.join("frozen"),
        filesystem: None,
        repository_root: root.clone(),
        system_apps: vec![first],
        projects: vec![project(&root, "user")],
        dora: DoraRuntime::default(),
        recordings: vec![],
        readiness_seconds: 10,
        stop_seconds: 3,
    };
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let executor = Rc::new_cyclic(|weak| {
        let mut executor = orchestrator::LocalExecutor::default();
        workload_docker::workflows::steps::register(
            &mut executor,
            adapter.clone(),
            Default::default(),
        );
        let backend = adapter.clone();
        let weak = weak.clone();
        executor.register::<preparation::PrepareRun>(move || preparation::PrepareRun {
            adapter: backend.clone(),
            executor: weak.clone(),
        });
        executor
    });

    // Act
    let prepared = executor
        .run::<preparation::PrepareRun>(spec.clone())
        .await
        .unwrap()
        .into_value();
    let model: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&prepared.plan.manifest).unwrap()).unwrap();
    std::fs::write(root.join("settings.pbtxt"), b"mode: changed\n").unwrap();

    // Assert: cwd, overrides, explicit environment, health, networks and volumes survived normalization.
    assert_eq!(prepared.plan.key, "experiment");
    assert_eq!(prepared.system_services, ["monitor"]);
    assert_eq!(prepared.user_services, ["adopted"]);
    assert_eq!(
        model["services"]["monitor"]["environment"]["DORA_NODE_ID"],
        "monitor"
    );
    assert_eq!(
        model["services"]["adopted"]["environment"]["DORA_NODE_ID"],
        "app-chosen"
    );
    for name in ["monitor", "adopted"] {
        assert_eq!(
            model["services"][name]["environment"]["DORA_DAEMON_LOCAL_LISTEN_PORT"],
            prepared.dora.daemon_port.to_string()
        );
        assert_eq!(
            model["services"][name]["environment"]["DORA_ZENOH_CONFIG_OVERLAY"],
            prepared.dora.zenoh_overlay
        );
    }
    assert_eq!(
        model["services"]["monitor"]["command"],
        serde_json::json!(["kept", "$$literal"])
    );
    assert_eq!(
        model["services"]["monitor"]["environment"]["SETTING"],
        "explicit"
    );
    assert_eq!(
        model["services"]["monitor"]["environment"]["LITERAL"],
        "$$literal"
    );
    assert_eq!(
        model["services"]["monitor"]["healthcheck"]["test"][0],
        "CMD"
    );
    assert!(
        model["services"]["adopted"]["networks"]
            .get("users")
            .is_some()
    );
    assert!(model["volumes"].get("data").is_some());
    let config = model["services"]["monitor"]["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["target"] == "/etc/settings.pbtxt")
        .unwrap();
    assert_eq!(
        std::fs::read(config["source"].as_str().unwrap()).unwrap(),
        b"mode: original\n"
    );
    for name in ["monitor", "adopted", "dora-coordinator"] {
        assert_eq!(model["services"][name]["ipc"], "service:dora-daemon");
        assert!(prepared.plan.services[name].image_id.starts_with("sha256:"));
    }
    assert_eq!(model["services"]["dora-daemon"]["ipc"], "shareable");
    assert!(
        prepared
            .plan
            .diagnostics
            .iter()
            .any(|d| d.contains("IPC silos"))
    );
    support::assert_no_run_resources(&adapter, &spec.id).await;

    // Act: conflicts must fail before containers or other runtime resources exist.
    let mut conflict = spec.clone();
    conflict.id.push_str("-conflict");
    conflict.artifacts = root.join("conflict");
    conflict.projects = vec![project(&root, "first")];
    conflict.projects[0].key = "other-input".into();
    conflict.projects[0]
        .environment
        .insert("SETTING".into(), "conflict".into());
    let error = executor
        .run::<preparation::PrepareRun>(conflict.clone())
        .await
        .unwrap_err();

    // Assert
    assert!(error.to_string().contains("naming conflict"), "{error}");
    support::assert_no_run_resources(&adapter, &conflict.id).await;

    for kind in ["services", "networks", "volumes"] {
        // Arrange: adopted input cannot supply labels used to identify experiment resources.
        let mut input = serde_json::json!({"services":{"adopted":{"image":image}}});
        input[kind]["adopted"]["labels"] =
            serde_json::json!({"io.doramcap.controller":"foreign-repository"});
        match kind {
            "networks" => input["services"]["adopted"][kind] = serde_json::json!(["adopted"]),
            "volumes" => input["services"]["adopted"][kind] = serde_json::json!(["adopted:/data"]),
            _ => {}
        }
        write(&root.join("reserved.json"), input);
        let mut reserved = spec.clone();
        reserved.id.push_str(&format!("-reserved-{kind}"));
        reserved.artifacts = root.join(format!("reserved-{kind}"));
        reserved.projects = vec![project(&root, "reserved")];

        // Act
        let error = executor
            .run::<preparation::PrepareRun>(reserved.clone())
            .await
            .unwrap_err();

        // Assert: resolver rejection happens before any runtime resources are created.
        assert!(
            error
                .to_string()
                .contains("reserved experiment-controller label"),
            "{kind}: {error}"
        );
        support::assert_no_run_resources(&adapter, &reserved.id).await;
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and prebuilt images"]
async fn mixed_volume_policies_and_evidence_preservation() {
    // Arrange: app inputs retain one named volume and remove the other.
    let root = std::env::temp_dir().join(format!("volume-policy-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let mut projects = Vec::new();
    for name in ["keep", "discard"] {
        let mut app = service(&image);
        app["volumes"] = serde_json::json!([format!("{name}:/data")]);
        write(
            &root.join(format!("{name}.json")),
            serde_json::json!({"services":{name:app},"volumes":{name:{}}}),
        );
        let mut input = project(&root, name);
        input.remove_volumes = name == "discard";
        projects.push(input);
    }
    let run = format!("volumes-{}", std::process::id());
    let artifacts = root.join(&run);
    let mut runner = ComposeRunner::new(adapter.clone());
    let (ready, started) = tokio::sync::oneshot::channel();

    // Act: normal workflow removal applies each input's declared policy.
    let report = runner
        .run_with_ready(
            ComposeRun {
                id: run.clone(),
                artifacts: artifacts.clone(),
                filesystem: None,
                repository_root: root.clone(),
                projects,
                system_apps: vec![],
                dora: Default::default(),
                recordings: vec![],
                readiness_seconds: 10,
                stop_seconds: 3,
            },
            async {
                let _ = started.await;
            },
            |_: &orchestrator::Event| {},
            Some(ready),
        )
        .await
        .unwrap();
    let resources = owned_volumes(&adapter, &run).await;

    // Assert: the retained volume is intentional; other resources are gone.
    assert!(report.successful(), "{report:?}");
    assert_eq!(resources.len(), 1, "{resources:?}");
    assert!(resources[0].ends_with("_keep"));

    // Arrange: retry removal with deletion requested but volume evidence unavailable.
    let mut plan: ResolvedComposeProject = serde_json::from_value(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(artifacts.join("experiment.resolution.json")).unwrap(),
        )
        .unwrap()["plan"]
            .clone(),
    )
    .unwrap();
    plan.remove_volumes = true;

    // Act: the evidence-preservation override must survive both native and fallback removal.
    let preserved = RemoveCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(StopInput {
        plan: Some(plan),
        services: vec![],
        allow_volume_deletion: false,
        stop_seconds: 3,
    })
    .await
    .unwrap();
    let remaining = owned_volumes(&adapter, &run).await;

    // Assert: changing the project policy cannot erase retained volume evidence.
    assert!(preserved.verified, "{preserved:?}");
    assert_eq!(remaining, resources);
    assert_eq!(preserved.retained_volumes, vec![resources[0].clone()]);
    let status = tokio::process::Command::new("docker")
        .args([
            "--host",
            &format!("unix://{}", adapter.socket_path().display()),
            "volume",
            "rm",
            &resources[0],
        ])
        .status()
        .await
        .unwrap();
    assert!(status.success());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker and prebuilt images"]
async fn incomplete_volume_scan_retains_volumes() {
    use prost::Message;
    use workload_docker::filesystem_proto as pb;

    // Arrange: an ordinary run whose policy would otherwise delete its named volume.
    let root = tempfile::tempdir().unwrap();
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let mut app = service(&image);
    app["volumes"] = serde_json::json!(["data:/data"]);
    write(
        &root.path().join("app.json"),
        serde_json::json!({
            "services": {"app": app}, "volumes": {"data": {}}
        }),
    );
    let run = format!("incomplete-volume-{}", std::process::id());
    let artifacts = root.path().join("run");
    let (ready, started) = tokio::sync::oneshot::channel();
    let mut runner = ComposeRunner::new(adapter.clone());

    // Act: make one baseline explicitly incomplete after startup, before final scanning.
    let report = runner
        .run_with_ready(
            ComposeRun {
                id: run.clone(),
                artifacts: artifacts.clone(),
                filesystem: None,
                repository_root: root.path().into(),
                projects: vec![project(root.path(), "app")],
                system_apps: vec![],
                dora: Default::default(),
                recordings: vec![],
                readiness_seconds: 10,
                stop_seconds: 3,
            },
            async {
                started.await.unwrap();
                let baseline: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(artifacts.join("filesystem/baseline.json")).unwrap(),
                )
                .unwrap();
                let source = baseline["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|source| source["category"] == "named_volume")
                    .unwrap();
                let header = pb::SnapshotRecord {
                    body: Some(pb::snapshot_record::Body::Header(pb::SnapshotHeader {
                        complete: false,
                        reason: Some("incomplete scan fixture".into()),
                        entries: 0,
                    })),
                };
                std::fs::write(
                    source["baseline"].as_str().unwrap(),
                    header.encode_length_delimited_to_vec(),
                )
                .unwrap();
            },
            |_: &orchestrator::Event| {},
            Some(ready),
        )
        .await
        .unwrap();

    // Assert: incomplete metadata blocks volume deletion.
    assert!(report.writers_stopped);
    assert!(!report.boundary_clean);
    assert!(
        !report
            .filesystem_changes
            .as_ref()
            .unwrap()
            .named_volumes
            .coverage_complete
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("named-volume evidence incomplete"))
    );
    assert_eq!(owned_volumes(&adapter, &run).await.len(), 1);

    // Act / Assert: explicitly dispose of this test's stopped synthetic resources.
    let plan: ResolvedComposeProject = serde_json::from_value(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(artifacts.join("experiment.resolution.json")).unwrap(),
        )
        .unwrap()["plan"]
            .clone(),
    )
    .unwrap();
    assert!(
        adapter
            .project(&plan)
            .inspect()
            .await
            .unwrap()
            .containers
            .iter()
            .all(|c| c.pid == 0)
    );
    let cleanup = RemoveCompose {
        adapter: adapter.clone(),
        diagnostics: Default::default(),
    }
    .run(StopInput {
        allow_volume_deletion: true,
        services: vec![],
        plan: Some(plan),
        stop_seconds: 3,
    })
    .await
    .unwrap();
    assert!(cleanup.verified, "{cleanup:?}");
    support::assert_no_run_resources(&adapter, &run).await;
}

/// List volumes owned by exactly one synthetic run for retention assertions.
async fn owned_volumes(adapter: &DockerAdapter, run: &str) -> Vec<String> {
    let docker = bollard::Docker::connect_with_unix(
        adapter.socket_path().to_str().unwrap(),
        120,
        bollard::API_DEFAULT_VERSION,
    )
    .unwrap()
    .negotiate_version()
    .await
    .unwrap();
    let mut names: Vec<_> = docker
        .list_volumes(Some(bollard::query_parameters::ListVolumesOptions {
            filters: Some(std::collections::HashMap::from([(
                "label".into(),
                vec![format!("io.doramcap.run={run}")],
            )])),
        }))
        .await
        .unwrap()
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.name)
        .collect();
    names.sort();
    names
}
