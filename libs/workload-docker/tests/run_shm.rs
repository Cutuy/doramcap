//! Complete recorded-workflow acceptance with real POSIX and SysV payloads.
mod support;
use serde_json::json;
use std::{path::Path, rc::Rc};
use workload_docker::{compose::*, workflows::recorded_compose::*};
use workload_lifecycle::launch::HostShmAccess;

async fn docker(args: &[&str]) -> std::process::Output {
    tokio::process::Command::new("docker")
        .args([
            "--host",
            &format!(
                "unix://{}",
                std::env::var("DORAMCAP_TEST_SOCKET")
                    .unwrap_or_else(|_| "/var/run/docker.sock".into())
            ),
        ])
        .args(args)
        .output()
        .await
        .unwrap()
}

async fn exec(id: &str, args: &[&str]) {
    let mut command = vec!["exec", id];
    command.extend_from_slice(args);
    let result = docker(&command).await;
    assert!(
        result.status.success(),
        "{command:?}: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn project(root: &Path) -> ComposeProject {
    ComposeProject {
        key: "apps".into(),
        directory: root.to_string_lossy().into_owned(),
        files: vec!["compose.json".into()],
        remove_volumes: true,
        ..Default::default()
    }
}

fn system_app(name: &str, manifest: &str) -> ComposeProject {
    ComposeProject {
        key: name.into(),
        directory: format!("{}/deploy/{name}", env!("CARGO_MANIFEST_DIR")),
        files: vec!["compose.yaml".into()],
        manifest: manifest.into(),
        ..Default::default()
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires real Docker, doramcap/dora:local, DORAMCAP_TEST_IMAGE and prebuilt --example shm_fixture; elevate only this executable"]
async fn run_memory_survives_producer_exit_and_preserves_live_host_binds() {
    let fixture = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/shm_fixture");
    assert!(
        fixture.is_file(),
        "build workload-docker --example shm_fixture first"
    );
    let image = std::env::var("DORAMCAP_TEST_IMAGE").unwrap();
    let adapter = Rc::new(
        DockerAdapter::connect(
            std::env::var("DORAMCAP_TEST_SOCKET").unwrap_or_else(|_| "/var/run/docker.sock".into()),
        )
        .await
        .unwrap(),
    );
    for with_dora in [false, true] {
        for mode in [HostShmAccess::Blocked, HostShmAccess::Passthrough] {
            // Arrange: the producer exits before the consumer starts.
            let root = tempfile::tempdir().unwrap();
            let host = tempfile::tempdir_in("/dev/shm").unwrap();
            let source = host.path().join("live");
            std::fs::write(&source, b"initial").unwrap();
            let identity = root
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_start_matches('.');
            let object = format!("doramcap-{identity}");
            let key = format!(
                "{}",
                0x46000000 | (std::process::id() << 4) | ((with_dora as u32) << 2) | mode as u32
            );
            let mut input = project(root.path());
            input.completed_services = vec!["producer".into()];
            let consumer = json!({"image":image,"entrypoint":["/bin/sh","-c"],"command":["trap 'exit 0' TERM INT; sleep infinity & wait $$!"],
            "depends_on":{"producer":{"condition":"service_completed_successfully"}},
            "volumes":[
                {"type":"bind","source":fixture,"target":"/fixture","read_only":true},
                {"type":"bind","source":source,"target":"/input/live","read_only":true},
                {"type":"bind","source":source,"target":"/dev/shm/live","read_only":true},
                {"type":"bind","source":host.path(),"target":"/live"},
                {"type":"volume","source":"selected","target":"/named","read_only":true}
            ]});
            std::fs::write(root.path().join("compose.json"), serde_json::to_vec(&json!({"services":{
                "producer":{"image":image,"entrypoint":["/fixture"],"command":["write",key,object],"volumes":[{"type":"bind","source":fixture,"target":"/fixture","read_only":true}]},
                "consumer":consumer
            },"volumes":{"selected":{"driver":"local","driver_opts":{"type":"none","o":"bind","device":host.path()}}}})).unwrap()).unwrap();
            let spec = ComposeRun {
                host_shm_access: mode as i32,
                id: format!("shm-{}", identity.to_ascii_lowercase()),
                artifacts: root.path().join("run"),
                repository_root: root.path().into(),
                filesystem: None,
                projects: vec![input],
                system_apps: vec![system_app("shm", "shm.pbtxt")],
                dora_apps: if with_dora {
                    vec![
                        system_app("dora-coordinator", "dora-coordinator.pbtxt"),
                        system_app("dora-daemon", "dora-daemon.pbtxt"),
                    ]
                } else {
                    vec![]
                },
                dataflow_file: String::new(),
                recordings: vec![],
                mcap_writers: Default::default(),
                readiness_seconds: 20,
                stop_seconds: 3,
            };
            let mut runner = ComposeRunner::new(adapter.clone());
            let (ready, started) = tokio::sync::oneshot::channel();
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let mut run = Box::pin(runner.run_with_ready(
                spec.clone(),
                async {
                    let _ = stopped.await;
                },
                |_: &orchestrator::Event| {},
                Some(ready),
            ));

            // Act: join and read both kinds of shm after the producer process exits.
            let plan = tokio::select! { ready = started => ready.unwrap(), result = &mut run => panic!("startup failed: {result:?}") };
            let state = adapter.project(&plan).inspect().await.unwrap();
            let producer = state
                .containers
                .iter()
                .find(|c| c.service == "producer")
                .unwrap();
            assert_eq!(producer.status, "exited");
            assert_eq!(producer.exit_code, Some(0));
            let consumer = &state
                .containers
                .iter()
                .find(|c| c.service == "consumer")
                .unwrap()
                .id;
            exec(consumer, &["/fixture", "read", &key, &object]).await;
            std::fs::write(&source, b"updated").unwrap();
            exec(consumer, &["/bin/sh", "-c", "test \"$(cat /input/live)\" = updated && test \"$(cat /dev/shm/live)\" = updated && test \"$(cat /named/live)\" = updated && printf app > /live/written"]).await;
            assert_eq!(std::fs::read(host.path().join("written")).unwrap(), b"app");
            for path in ["/input/live", "/dev/shm/live", "/named/live"] {
                let command = format!("printf denied > {path}");
                assert!(
                    !docker(&["exec", consumer, "/bin/sh", "-c", &command])
                        .await
                        .status
                        .success()
                );
            }
            let host_read = std::process::Command::new(&fixture)
                .args([
                    if mode == HostShmAccess::Blocked {
                        "absent"
                    } else {
                        "read"
                    },
                    &key,
                    &object,
                ])
                .status()
                .unwrap();
            assert!(host_read.success());

            // Assert: another private run cannot see this run's POSIX or SysV objects.
            if mode == HostShmAccess::Blocked {
                let other_root = tempfile::tempdir().unwrap();
                std::fs::write(other_root.path().join("compose.json"), serde_json::to_vec(&json!({"services":{"other":{
                    "image":image,"entrypoint":["/bin/sh","-c"],"command":["trap 'exit 0' TERM INT; sleep infinity & wait $$!"],
                    "volumes":[{"type":"bind","source":fixture,"target":"/fixture","read_only":true}]
                }}})).unwrap()).unwrap();
                let mut other_spec = spec.clone();
                other_spec.id.push_str("-other");
                other_spec.repository_root = other_root.path().into();
                other_spec.artifacts = other_root.path().join("run");
                other_spec.projects = vec![project(other_root.path())];
                other_spec.dora_apps.clear();
                let mut other_runner = ComposeRunner::new(adapter.clone());
                let (other_ready, other_started) = tokio::sync::oneshot::channel();
                let (other_stop, other_stopped) = tokio::sync::oneshot::channel();
                let mut other_run = Box::pin(other_runner.run_with_ready(
                    other_spec.clone(),
                    async {
                        let _ = other_stopped.await;
                    },
                    |_: &orchestrator::Event| {},
                    Some(other_ready),
                ));
                let other_plan = tokio::select! { ready = other_started => ready.unwrap(), result = &mut other_run => panic!("other startup failed: {result:?}") };
                let other_state = adapter.project(&other_plan).inspect().await.unwrap();
                let other = &other_state
                    .containers
                    .iter()
                    .find(|c| c.service == "other")
                    .unwrap()
                    .id;
                exec(other, &["/fixture", "absent", &key, &object]).await;
                other_stop.send(()).unwrap();
                assert!(other_run.await.unwrap().boundary_clean);
                support::assert_no_run_resources(&adapter, &other_spec.id).await;
                exec(consumer, &["/fixture", "read", &key, &object]).await;
            }

            // Act / Assert: cleanup removes owned containers and preserves host data.
            stop.send(()).unwrap();
            let report = run.await.unwrap();
            assert!(report.boundary_clean, "{report:?}");
            assert!(report.execution_error.is_none(), "{report:?}");
            assert_eq!(std::fs::read(&source).unwrap(), b"updated");
            if mode == HostShmAccess::Passthrough {
                assert!(
                    std::process::Command::new(&fixture)
                        .args(["read", &key, &object])
                        .status()
                        .unwrap()
                        .success()
                );
                assert!(
                    std::process::Command::new(&fixture)
                        .args(["remove", &key, &object])
                        .status()
                        .unwrap()
                        .success()
                );
            }
            support::assert_no_run_resources(&adapter, &spec.id).await;
        }
    }
}
