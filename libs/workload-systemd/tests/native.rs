//! Native systemd runs. Build unprivileged; run only this executable as root:
//! `sudo target/release/deps/native-* --ignored --test-threads=1`.
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tokio::sync::oneshot;
use workload_systemd::{Bind, Program, RunPlan, SystemdPlan, SystemdRunReport, SystemdRunner};

/// How long a scenario lets its units run after readiness.
const OBSERVATION: Duration = Duration::from_secs(3);

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("systemd-native-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("repo")).unwrap();
        Self { root }
    }
    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }
    /// The shared run inputs and the systemd plan of run `run`.
    fn plan(&self, run: &str, phases: Vec<SystemdPlan>) -> (RunPlan, SystemdPlan) {
        let mut shared = RunPlan {
            id: run.into(),
            repository_root: self.repo().to_string_lossy().into(),
            artifacts: self.root.join(run).to_string_lossy().into(),
            readiness_seconds: 10,
            stop_seconds: 5,
            ..Default::default()
        };
        let mut plan = SystemdPlan::default();
        let has_system = phases.len() > 1;
        for (index, phase) in phases.into_iter().enumerate() {
            if has_system && index == 0 {
                shared
                    .system_apps
                    .extend(phase.programs.iter().map(|app| app.id.clone()));
                shared
                    .system_apps
                    .extend(phase.unit_files.iter().map(|file| {
                        Path::new(file)
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned()
                    }));
                shared
                    .system_apps
                    .extend(phase.app_units.iter().map(|app| app.id.clone()));
            }
            plan.programs.extend(phase.programs);
            plan.unit_files.extend(phase.unit_files);
            plan.app_units.extend(phase.app_units);
        }
        (shared, plan)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Show the fixture directory at its own path inside a unit's private root.
fn sees(mut program: Program, fixture: &Fixture) -> Program {
    let root = fixture.root.to_string_lossy().into_owned();
    program.binds.push(Bind {
        source: root.clone(),
        target: root,
        read_only: false,
    });
    program
}

fn program(id: &str, script: &str) -> Program {
    Program {
        id: id.into(),
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        ..Default::default()
    }
}

/// Run `plan`, stop it `OBSERVATION` after readiness, and return its report.
async fn run_for_a_while((run, plan): (RunPlan, SystemdPlan)) -> SystemdRunReport {
    let (ready, readiness) = oneshot::channel();
    let stop = async {
        if readiness.await.is_ok() {
            tokio::time::sleep(OBSERVATION).await;
        }
    };
    SystemdRunner::new()
        .run(run, plan, stop, |_: &orchestrator::Event| {}, Some(ready))
        .await
        .unwrap()
}

fn runtime_units() -> Vec<String> {
    std::fs::read_dir("/run/systemd/system")
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("doramcap-"))
                .collect()
        })
        .unwrap_or_default()
}

fn active(pattern: &str) -> String {
    let output = Command::new("systemctl")
        .args(["list-units", "--all", "--plain", "--no-legend", pattern])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn system_apps_start_before_user_apps_and_stop_cleanly() {
    // Arrange
    let fixture = Fixture::new("order");
    let adopted = fixture.repo().join("adopted.service");
    std::fs::write(&adopted, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
    let marker = fixture.root.join("first-started");
    let support = fixture.repo().join("first.service");
    std::fs::write(
        &support,
        format!(
            "[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=/usr/bin/touch {}\n",
            marker.display()
        ),
    )
    .unwrap();
    let plan = fixture.plan(
        "r1",
        vec![
            SystemdPlan {
                unit_files: vec![support.to_string_lossy().into()],
                ..Default::default()
            },
            SystemdPlan {
                programs: vec![sees(
                    program(
                        "second",
                        &format!("test -e {} && exec sleep 300", marker.display()),
                    ),
                    &fixture,
                )],
                unit_files: vec![adopted.to_string_lossy().into()],
                ..Default::default()
            },
        ],
    );
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(report.units.len(), 3);
    assert!(
        report
            .units
            .iter()
            .all(|unit| unit.result.as_deref() == Some("success")),
        "{report:#?}"
    );
    assert!(
        report
            .units
            .iter()
            .all(|unit| unit.cpu_usage_nsec.is_some()),
        "{report:#?}"
    );
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
    assert!(Path::new(&fixture.root.join("r1/report.json")).is_file());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn a_failing_unit_is_recorded_and_the_run_continues() {
    // Arrange
    let fixture = Fixture::new("failing");
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![
                program("steady", "exec sleep 300"),
                program("fails", "sleep 1; exit 3"),
            ],
            ..Default::default()
        }],
    );
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.execution_error.is_none(), "{report:#?}");
    assert!(
        report.writers_stopped && report.cleanup_verified,
        "{report:#?}"
    );
    let fails = report
        .units
        .iter()
        .find(|unit| unit.unit.ends_with("-fails.service"))
        .unwrap();
    assert_eq!(
        (fails.result.as_deref(), fails.exec_main_status),
        (Some("exit-code"), Some(3))
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn placement_and_memlock_reach_the_process() {
    // Arrange
    let fixture = Fixture::new("limits");
    let facts = fixture.root.join("facts");
    let mut limited = program(
        "limited",
        &format!(
            "grep Cpus_allowed_list /proc/self/status > {0}; ulimit -l >> {0}; exec sleep 300",
            facts.display()
        ),
    );
    limited = sees(limited, &fixture);
    limited.cpus = "0".into();
    limited.memlock_bytes = 1024 * 1024;
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![limited],
            ..Default::default()
        }],
    );
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    let facts = std::fs::read_to_string(facts).unwrap();
    assert_eq!(facts, "Cpus_allowed_list:\t0\n1024\n");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn abandoned_units_block_admission_until_explicit_cleanup() {
    // Arrange: a run dropped after readiness leaves its unit running.
    let fixture = Fixture::new("dropped");
    let (run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![program("left", "exec sleep 300")],
            ..Default::default()
        }],
    );
    let (ready, readiness) = oneshot::channel();
    let mut runner = SystemdRunner::new();
    tokio::select! {
        _ = runner.run(run, plan, std::future::pending(), |_: &orchestrator::Event| {}, Some(ready)) => {
            panic!("run ended without stop")
        }
        _ = readiness => {}
    }
    assert!(active("doramcap-*-left.service").contains("active running"));
    // Act: admission observes prior resources without changing them.
    let (run, plan) = fixture.plan("r2", vec![]);
    let error = runner
        .run(run, plan, async {}, |_: &orchestrator::Event| {}, None)
        .await
        .unwrap_err();
    // Assert
    assert!(error.contains("--cleanup"), "{error}");
    assert!(active("doramcap-*-left.service").contains("active running"));
    assert!(!fixture.root.join("r2").exists());
    // Act
    runner.cleanup(&fixture.repo()).await.unwrap();
    let report = run_for_a_while(fixture.plan("r2", vec![])).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(!active("doramcap-*-left.service").contains("running"));
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn timed_out_start_stops_and_reports_the_attempted_writer() {
    // Arrange
    let fixture = Fixture::new("startup-timeout");
    let file = fixture.repo().join("slow.service");
    let pid = fixture.root.join("writer.pid");
    std::fs::write(
        &file,
        format!(
            "[Service]\nType=oneshot\nExecStart=/bin/sh -c 'echo $$$$ > {}; exec sleep 300'\n",
            pid.display()
        ),
    )
    .unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    );
    run.readiness_seconds = 1;
    // Act
    let report = run_for_a_while((run, plan)).await;
    // Assert
    assert!(
        report
            .execution_error
            .as_deref()
            .unwrap()
            .contains("not active")
    );
    assert_eq!(report.units.len(), 1, "{report:#?}");
    assert!(
        report.writers_stopped && report.recording_finalized && report.cleanup_verified,
        "{report:#?}"
    );
    let pid = std::fs::read_to_string(pid).unwrap();
    assert!(!Path::new(&format!("/proc/{}", pid.trim())).exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn user_apps_start_peers_before_waiting_for_readiness() {
    // Arrange
    let fixture = Fixture::new("group-peers");
    let marker = fixture.root.join("peer-started");
    let first = fixture.repo().join("first.service");
    let second = fixture.repo().join("second.service");
    std::fs::write(&first, format!("[Unit]\nWants=basic.target\n[Service]\nType=oneshot\nExecStart=/bin/sh -c 'while ! test -f {}; do sleep 0.01; done'\n", marker.display())).unwrap();
    std::fs::write(
        &second,
        format!(
            "[Service]\nType=oneshot\nExecStart=/bin/touch {}\n",
            marker.display()
        ),
    )
    .unwrap();
    // Act
    let report = run_for_a_while(fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![first.display().to_string(), second.display().to_string()],
            ..Default::default()
        }],
    ))
    .await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(marker.exists());
    assert_eq!(report.units.len(), 2);
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn adopted_stop_wait_uses_the_service_grace() {
    // Arrange: the declared grace exceeds the run fallback and old reporting margin.
    let fixture = Fixture::new("service-grace");
    let file = fixture.repo().join("slow-stop.service");
    std::fs::write(
        &file,
        "[Service]\nExecStart=/bin/sleep 300\nExecStop=/bin/sleep 7\nTimeoutStopSec=10\n",
    )
    .unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    );
    run.stop_seconds = 1;
    // Act
    let report = run_for_a_while((run, plan)).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn infinite_grace_reports_incomplete_and_preserves_artifacts() {
    // Arrange
    let fixture = Fixture::new("infinite-grace");
    let file = fixture.repo().join("infinite.service");
    std::fs::write(
        &file,
        "[Service]\nExecStart=/bin/sleep 300\nTimeoutStopSec=infinity\n",
    )
    .unwrap();
    // Act
    let report = run_for_a_while(fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    ))
    .await;
    // Assert
    assert!(
        !report.writers_stopped && !report.recording_finalized && !report.cleanup_verified,
        "{report:#?}"
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.contains("infinite stop timeout"))
    );
    assert!(fixture.root.join("r1/report.json").exists());
    // Act: wait for the already requested stop, then clean up explicitly.
    let status = Command::new("systemctl")
        .args(["stop", &report.units[0].unit])
        .status()
        .unwrap();
    assert!(status.success());
    SystemdRunner::new().cleanup(&fixture.repo()).await.unwrap();
    // Assert
    assert!(fixture.root.join("r1/report.json").exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn adopted_native_references_are_accepted_unchanged() {
    // Arrange: systemd owns references, environment files, pre-start commands and permissions.
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new("native-references");
    let file = fixture.repo().join("references.service");
    let environment = fixture.root.join("native.env");
    let marker = fixture.root.join("native-marker");
    std::fs::write(&environment, "NATIVE_VALUE=unit-file-value\n").unwrap();
    let contents = format!(
        "[Unit]\nRequires=basic.target\nAfter=basic.target\n[Service]\nEnvironmentFile={}\nUMask=0077\nExecStartPre=/bin/sh -c 'printf %%s \"$$NATIVE_VALUE\" > {}'\nExecStart=/bin/sleep 300\n",
        environment.display(),
        marker.display(),
    );
    std::fs::write(&file, &contents).unwrap();
    // Act
    let report = run_for_a_while(fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    ))
    .await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(std::fs::read_to_string(file).unwrap(), contents);
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "unit-file-value");
    assert_eq!(
        std::fs::metadata(marker).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn adopted_service_without_grace_uses_the_plan_fallback() {
    // Arrange
    let fixture = Fixture::new("fallback-grace");
    let file = fixture.repo().join("fallback.service");
    std::fs::write(&file, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    );
    run.stop_seconds = 1;
    let (ready, readiness) = oneshot::channel();
    let stop = async {
        readiness.await.unwrap();
        // Act: read the running service's effective timeout from systemd.
        let output = Command::new("systemctl")
            .args([
                "show",
                "--value",
                "--property=TimeoutStopUSec",
                "doramcap-*-fallback.service",
            ])
            .output()
            .unwrap();
        // Assert
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "1s");
    };
    // Act
    let report = SystemdRunner::new()
        .run(run, plan, stop, |_: &orchestrator::Event| {}, Some(ready))
        .await
        .unwrap();
    // Assert
    assert!(report.successful(), "{report:#?}");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn concurrent_repositories_keep_units_and_cleanup_separate() {
    // Arrange: identical service names under different repository roots.
    let first = Fixture::new("isolation-first");
    let second = Fixture::new("isolation-second");
    let make_plan = |fixture: &Fixture, id: &str| {
        let file = fixture.repo().join("isolated.service");
        std::fs::write(&file, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
        fixture.plan(
            id,
            vec![SystemdPlan {
                unit_files: vec![file.display().to_string()],
                ..Default::default()
            }],
        )
    };
    let (first_run, first_plan) = make_plan(&first, "r1");
    let (second_run, second_plan) = make_plan(&second, "r1");
    let (duplicate_run, duplicate_plan) = make_plan(&second, "r2");
    let (first_ready, first_readiness) = oneshot::channel();
    let (second_ready, second_readiness) = oneshot::channel();
    let (first_stop, first_stopping) = oneshot::channel();
    let (second_stop, second_stopping) = oneshot::channel();
    let (first_done, first_completion) = oneshot::channel();
    let mut first_runner = SystemdRunner::new();
    let mut second_runner = SystemdRunner::new();
    // Act
    let first_task = async {
        let report = first_runner
            .run(
                first_run,
                first_plan,
                async {
                    let _ = first_stopping.await;
                },
                |_: &orchestrator::Event| {},
                Some(first_ready),
            )
            .await
            .unwrap();
        assert!(report.successful(), "{report:#?}");
        first_done.send(()).unwrap();
        report
    };
    let second_task = second_runner.run(
        second_run,
        second_plan,
        async {
            let _ = second_stopping.await;
        },
        |_: &orchestrator::Event| {},
        Some(second_ready),
    );
    let control = async {
        first_readiness.await.unwrap();
        second_readiness.await.unwrap();
        // Assert: different repositories run together; the same repository stays locked.
        assert_eq!(active("doramcap-*-isolated.service").lines().count(), 2);
        let duplicate = SystemdRunner::new()
            .run(
                duplicate_run,
                duplicate_plan,
                std::future::pending(),
                |_: &orchestrator::Event| {},
                None,
            )
            .await;
        assert!(
            duplicate
                .unwrap_err()
                .contains("another run owns this repository")
        );
        // Act: stop and explicitly clean only the first repository.
        first_stop.send(()).unwrap();
        first_completion.await.unwrap();
        SystemdRunner::new().cleanup(&first.repo()).await.unwrap();
        // Assert: the other repository's identically named service keeps running.
        let remaining = active("doramcap-*-isolated.service");
        assert_eq!(remaining.lines().count(), 1, "{remaining}");
        assert!(remaining.contains("running"), "{remaining}");
        // Act
        second_stop.send(()).unwrap();
    };
    let (_, second_report, ()) = tokio::join!(first_task, second_task, control);
    // Assert
    let second_report = second_report.unwrap();
    assert!(second_report.successful(), "{second_report:#?}");
    assert!(active("doramcap-*-isolated.service").trim().is_empty());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and DORAMCAP_TEST_DORA"]
async fn dora_runs_as_units_and_registers_the_graph() {
    // Arrange
    let executable =
        std::env::var("DORAMCAP_TEST_DORA").expect("DORAMCAP_TEST_DORA names a dora binary");
    let fixture = Fixture::new("dora");
    let runtime = fixture.root.join("dora");
    std::fs::create_dir_all(&runtime).unwrap();
    let graph = fixture.root.join("graph.yml");
    std::fs::write(
        &graph,
        "nodes:\n  - id: user\n    path: dynamic\n    outputs: [messages]\n",
    )
    .unwrap();
    let mut plan = fixture.plan("r1", vec![]);
    plan.0.dora_apps = vec!["dora-coordinator".into(), "dora-daemon".into()];
    plan.0.dataflow_file = graph.to_string_lossy().into();
    let pool = prost_reflect::DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/systemd_plan.bin")).as_slice(),
    )
    .unwrap();
    for role in ["coordinator", "daemon"] {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../workload-docker/deploy")
            .join(format!("dora-{role}"));
        let text = std::fs::read_to_string(directory.join(format!("dora-{role}.pbtxt"))).unwrap();
        let app: workload_lifecycle::launch::AppManifest =
            prost_reflect::DynamicMessage::parse_text_format(
                pool.get_message_by_name("doramcap.launch.v1.AppManifest")
                    .unwrap(),
                &text,
            )
            .unwrap()
            .transcode_to()
            .unwrap();
        let process = app.process.unwrap();
        process.validate().unwrap();
        let mut argv = process.argv;
        argv[0] = executable.clone();
        let mut binds: Vec<_> = app
            .files
            .iter()
            .map(|file| Bind {
                source: directory.join(&file.source).to_string_lossy().into_owned(),
                target: file.target.clone(),
                read_only: true,
            })
            .collect();
        binds.push(Bind {
            source: runtime.to_string_lossy().into_owned(),
            target: "/runtime".into(),
            read_only: false,
        });
        plan.1.programs.push(Program {
            id: app.id,
            argv,
            working_directory: process.working_directory,
            memlock_bytes: process.memlock_bytes,
            kill_signal: process.stop_signal,
            stop_seconds: process.stop_seconds,
            binds,
            ..Default::default()
        });
    }
    let mut participant = program(
        "user",
        r#"printf '%s\n' "$DORA_NODE_ID" "$DORA_DAEMON_LOCAL_LISTEN_PORT" "$DORA_ZENOH_CONFIG_OVERLAY" > /results/environment; test -s "$DORA_ZENOH_CONFIG_OVERLAY" || exit 42; exec sleep 300"#,
    );
    participant.binds.push(Bind {
        source: runtime.to_string_lossy().into_owned(),
        target: "/results".into(),
        read_only: false,
    });
    plan.1.programs.push(participant);
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    let names: Vec<_> = report.units.iter().map(|unit| unit.unit.as_str()).collect();
    assert!(
        names
            .iter()
            .any(|name| name.ends_with("-dora-coordinator.service")),
        "{names:?}"
    );
    assert!(
        names
            .iter()
            .any(|name| name.ends_with("-dora-daemon.service")),
        "{names:?}"
    );
    let environment = std::fs::read_to_string(runtime.join("environment")).unwrap();
    let values: Vec<_> = environment.lines().collect();
    assert_eq!(values[0], "user");
    assert!(values[1].parse::<u16>().unwrap() > 0);
    assert_eq!(values[2], "/config/zenoh.json5");
    assert!(runtime.join("dataflow.yml").is_file());
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and DORAMCAP_TEST_DORA"]
async fn dora_app_units_resolve_their_native_executable_and_register_the_graph() {
    // Arrange: use external units with a bare executable and private runtime binds.
    let fixture = Fixture::new("dora-apps");
    let binaries = fixture.root.join("bin");
    let runtime = fixture.root.join("runtime");
    std::fs::create_dir(&binaries).unwrap();
    std::fs::create_dir(&runtime).unwrap();
    std::fs::copy(
        std::env::var("DORAMCAP_TEST_DORA").unwrap(),
        binaries.join("dora"),
    )
    .unwrap();
    let graph = fixture.root.join("graph.yml");
    std::fs::write(
        &graph,
        "nodes: [{id: user, path: dynamic, outputs: [messages]}]\n",
    )
    .unwrap();
    let ports: Vec<_> = (0..2)
        .map(|_| std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap())
        .collect();
    let coordinator = ports[0].local_addr().unwrap().port().to_string();
    let daemon = ports[1].local_addr().unwrap().port().to_string();
    let mut plan = fixture.plan("r1", vec![]);
    plan.0.dora_apps = vec!["dora-coordinator".into(), "dora-daemon".into()];
    plan.0.dataflow_file = graph.to_string_lossy().into();
    plan.1.binaries = binaries.to_string_lossy().into();
    for (role, contents) in [
        (
            "coordinator",
            include_str!("fixtures/dora-coordinator.service"),
        ),
        ("daemon", include_str!("fixtures/dora-daemon.service")),
    ] {
        let file = fixture.root.join(format!("dora-{role}.service"));
        std::fs::write(
            &file,
            contents
                .replace("6013", &coordinator)
                .replace("6014", &daemon),
        )
        .unwrap();
        let mut binds = vec![Bind {
            source: runtime.to_string_lossy().into(),
            target: "/runtime".into(),
            read_only: false,
        }];
        if role == "daemon" {
            let overlay = fixture.root.join("zenoh.json5");
            std::fs::write(
                &overlay,
                include_str!("../../../console/release/system-apps/dora-daemon/zenoh.json5"),
            )
            .unwrap();
            binds.push(Bind {
                source: overlay.to_string_lossy().into(),
                target: "/config/zenoh.json5".into(),
                read_only: true,
            });
        }
        plan.1.app_units.push(workload_systemd::AppUnit {
            id: format!("dora-{role}"),
            file: file.to_string_lossy().into(),
            binds,
        });
    }
    drop(ports);
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(runtime.join("dataflow.yml").is_file());
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn bind_mounts_show_host_paths_at_container_targets() {
    // Arrange: the unit reads its config and writes its result at container paths.
    let fixture = Fixture::new("binds");
    let config = fixture.root.join("monitor.pbtxt");
    let results = fixture.root.join("results");
    std::fs::write(&config, "rate: 5\n").unwrap();
    std::fs::create_dir_all(&results).unwrap();
    let mut writer = program(
        "writer",
        "cp /doramcap-native-binds/config/monitor.pbtxt /doramcap-native-binds/results/seen; touch /doramcap-native-binds/config/monitor.pbtxt 2>/dev/null || echo read-only >> /doramcap-native-binds/results/seen; exec sleep 300",
    );
    writer.binds = vec![
        Bind {
            source: config.to_string_lossy().into(),
            target: "/doramcap-native-binds/config/monitor.pbtxt".into(),
            read_only: true,
        },
        Bind {
            source: results.to_string_lossy().into(),
            target: "/doramcap-native-binds/results".into(),
            read_only: false,
        },
    ];
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![writer],
            ..Default::default()
        }],
    );
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(
        std::fs::read_to_string(results.join("seen")).unwrap(),
        "rate: 5\nread-only\n"
    );
    // Bind targets live in the unit's private root, never on the host.
    assert!(!Path::new("/doramcap-native-binds").exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn app_unit_files_find_binaries_and_see_binds_in_a_private_root() {
    // Arrange: an app unit names its executable only; config and results are bound.
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new("app-unit");
    let binaries = fixture.root.join("bin");
    std::fs::create_dir_all(&binaries).unwrap();
    let probe = binaries.join("probe-app");
    std::fs::write(
        &probe,
        "#!/bin/sh\ncp /doramcap-native-app/config /doramcap-native-app/results/seen\nexec sleep 300\n",
    )
    .unwrap();
    std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = fixture.root.join("config.pbtxt");
    let results = fixture.root.join("results");
    std::fs::write(&config, "rate: 5\n").unwrap();
    std::fs::create_dir_all(&results).unwrap();
    let unit = fixture.root.join("probe.service");
    std::fs::write(&unit, "[Service]\nExecStart=probe-app\n").unwrap();
    let mut plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            app_units: vec![workload_systemd::AppUnit {
                id: "capture".into(),
                file: unit.to_string_lossy().into(),
                binds: vec![
                    Bind {
                        source: config.to_string_lossy().into(),
                        target: "/doramcap-native-app/config".into(),
                        read_only: true,
                    },
                    Bind {
                        source: results.to_string_lossy().into(),
                        target: "/doramcap-native-app/results".into(),
                        read_only: false,
                    },
                ],
            }],
            ..Default::default()
        }],
    );
    plan.1.binaries = binaries.to_string_lossy().into();
    plan.0.system_apps = vec!["capture".into()];
    // Act
    let report = run_for_a_while(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(
        report
            .units
            .iter()
            .any(|unit| unit.unit.ends_with("-capture.service"))
    );
    assert_eq!(
        std::fs::read_to_string(results.join("seen")).unwrap(),
        "rate: 5\n"
    );
    assert!(!Path::new("/doramcap-native-app").exists());
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn missing_system_recording_prevents_user_start() {
    // Arrange
    let fixture = Fixture::new("recording-gate");
    let marker = fixture.root.join("user-started");
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![
            SystemdPlan {
                programs: vec![program("writer", "exec sleep 300")],
                ..Default::default()
            },
            SystemdPlan {
                programs: vec![sees(
                    program(
                        "producer",
                        &format!("touch {}; exec sleep 300", marker.display()),
                    ),
                    &fixture,
                )],
                ..Default::default()
            },
        ],
    );
    run.readiness_seconds = 1;
    run.recordings.push(workload_lifecycle::launch::Recording {
        recording: fixture.root.join("missing.mcap").to_string_lossy().into(),
        stream: String::new(),
    });
    // Act
    let report = SystemdRunner::new()
        .run(
            run,
            plan,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
        )
        .await
        .unwrap();
    // Assert
    assert!(report.execution_error.is_some(), "{report:#?}");
    assert!(!marker.exists());
    assert!(
        report
            .units
            .iter()
            .any(|unit| unit.unit.ends_with("-writer.service"))
    );
    assert!(
        !report
            .units
            .iter()
            .any(|unit| unit.unit.ends_with("-producer.service"))
    );
    assert!(report.writers_stopped, "{report:#?}");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn review_preflight_rejects_changes_and_cancellation_before_start_acceptance() {
    // Arrange
    let fixture = Fixture::new("review-reject");
    let marker = fixture.root.join("started");
    let file = fixture.repo().join("worker.service");
    std::fs::write(
        &file,
        format!(
            "[Service]\nExecStart=/bin/sh -c 'touch {}; exec sleep 300'\n",
            marker.display()
        ),
    )
    .unwrap();
    let (run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.to_string_lossy().into()],
            ..Default::default()
        }],
    );
    let expected = workload_systemd::inspect(&run, &plan).unwrap();
    std::fs::write(&file, "[Service]\nExecStart=/bin/false\n").unwrap();
    let accepted = std::cell::Cell::new(false);
    let mut runner = SystemdRunner::new();
    // Act
    let result = runner
        .run_with_preflight(
            run.clone(),
            plan.clone(),
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |actual| {
                if actual != &expected {
                    return Err("ConfigurationChanged".into());
                }
                accepted.set(true);
                Ok(())
            },
        )
        .await;
    // Assert
    assert_eq!(result.unwrap_err(), "ConfigurationChanged");
    assert!(!accepted.get());
    assert!(!runner.blocked());
    assert!(!Path::new(&run.artifacts).exists());
    assert!(!marker.exists());
    assert!(runtime_units().is_empty());
    // Act / Assert: cancellation at the same admission boundary also starts nothing.
    let expected = workload_systemd::inspect(&run, &plan).unwrap();
    let result = runner
        .run_with_preflight(
            run.clone(),
            plan,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |actual| {
                assert_eq!(actual, &expected);
                Err("Stopped".into())
            },
        )
        .await;
    assert_eq!(result.unwrap_err(), "Stopped");
    assert!(!Path::new(&run.artifacts).exists());
    assert!(!marker.exists());
    assert!(runtime_units().is_empty());
}

/// Native Linux POSIX ACL input granting one named user read access.
fn read_acl(uid: u32) -> Vec<u8> {
    // linux/posix_acl{,_xattr}.h defines the little-endian kernel ABI.
    const VERSION: u32 = 2;
    const USER_OBJ: u16 = 0x01;
    const USER: u16 = 0x02;
    const GROUP_OBJ: u16 = 0x04;
    const MASK: u16 = 0x10;
    const OTHER: u16 = 0x20;
    const READ: u16 = 0x04;
    const WRITE: u16 = 0x02;
    let mut bytes = VERSION.to_le_bytes().to_vec();
    for (tag, permissions, id) in [
        (USER_OBJ, READ | WRITE, u32::MAX),
        (USER, READ, uid),
        (GROUP_OBJ, 0, u32::MAX),
        (MASK, READ, u32::MAX),
        (OTHER, 0, u32::MAX),
    ] {
        bytes.extend(tag.to_le_bytes());
        bytes.extend(permissions.to_le_bytes());
        bytes.extend(id.to_le_bytes());
    }
    bytes
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and the nobody user"]
async fn review_launches_frozen_units_and_preserves_nonroot_config_access() {
    // Arrange
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new("review-freeze");
    let uid: u32 = String::from_utf8(
        Command::new("id")
            .args(["-u", "nobody"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .parse()
    .unwrap();
    let gid: u32 = String::from_utf8(
        Command::new("id")
            .args(["-g", "nobody"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .parse()
    .unwrap();
    let owner = fixture.root.join("owner.pbtxt");
    let acl = fixture.root.join("acl.pbtxt");
    std::fs::write(&owner, "owned input\n").unwrap();
    std::fs::write(&acl, "ACL input\n").unwrap();
    std::os::unix::fs::chown(&owner, Some(uid), Some(gid)).unwrap();
    for path in [&owner, &acl] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    rustix::fs::setxattr(
        &acl,
        "system.posix_acl_access",
        &read_acl(uid),
        rustix::fs::XattrFlags::empty(),
    )
    .unwrap();
    let output = fixture.root.join("output");
    std::fs::create_dir(&output).unwrap();
    std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o777)).unwrap();
    let mut reader = program(
        "reader",
        "cat /config/owner.pbtxt /config/acl.pbtxt > /results/seen; exec sleep 300",
    );
    reader.user = "nobody".into();
    reader.binds = vec![
        Bind {
            source: owner.to_string_lossy().into(),
            target: "/config/owner.pbtxt".into(),
            read_only: true,
        },
        Bind {
            source: acl.to_string_lossy().into(),
            target: "/config/acl.pbtxt".into(),
            read_only: true,
        },
        Bind {
            source: output.to_string_lossy().into(),
            target: "/results".into(),
            read_only: false,
        },
    ];
    let file = fixture.repo().join("external.service");
    let native_seen = fixture.root.join("native-seen");
    std::fs::write(
        &file,
        format!(
            "[Service]\nExecStart=/bin/sh -c 'printf original > {}; exec sleep 300'\n",
            native_seen.display()
        ),
    )
    .unwrap();
    let (run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![reader],
            unit_files: vec![file.to_string_lossy().into()],
            ..Default::default()
        }],
    );
    let expected = workload_systemd::inspect(&run, &plan).unwrap();
    let (ready, readiness) = oneshot::channel();
    let stop = async {
        if readiness.await.is_ok() {
            tokio::time::sleep(OBSERVATION).await;
        }
    };
    // Act: mutation after capture must not change the bytes any launched unit sees.
    let report = SystemdRunner::new()
        .run_with_preflight(
            run.clone(),
            plan.clone(),
            stop,
            |_: &orchestrator::Event| {},
            Some(ready),
            |actual| {
                assert_eq!(actual, &expected);
                std::fs::write(&owner, "changed owner input").unwrap();
                std::fs::write(&acl, "changed ACL input").unwrap();
                std::fs::write(&file, "[Service]\nExecStart=/bin/false\n").unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(
        std::fs::read_to_string(output.join("seen")).unwrap(),
        "owned input\nACL input\n"
    );
    assert_eq!(std::fs::read_to_string(native_seen).unwrap(), "original");
    assert_ne!(expected, workload_systemd::inspect(&run, &plan).unwrap());
    assert!(runtime_units().is_empty());
}
