//! Native systemd runs. Build unprivileged; run only this executable as root:
//! `sudo target/release/deps/native-* --ignored --test-threads=1`.
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tokio::sync::oneshot;
use workload_systemd::{Program, RunPlan, SystemdPlan, SystemdRunReport, SystemdRunner, UnitGroup};

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
    fn plan(&self, run: &str, groups: Vec<UnitGroup>) -> (RunPlan, SystemdPlan) {
        let shared = RunPlan {
            id: run.into(),
            repository_root: self.repo().to_string_lossy().into(),
            artifacts: self.root.join(run).to_string_lossy().into(),
            readiness_seconds: 10,
            stop_seconds: 5,
            ..Default::default()
        };
        (shared, SystemdPlan { groups })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
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
async fn groups_start_in_order_and_stop_cleanly() {
    // Arrange
    let fixture = Fixture::new("order");
    let adopted = fixture.repo().join("adopted.service");
    std::fs::write(&adopted, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
    let marker = fixture.root.join("first-started");
    let plan = fixture.plan(
        "r1",
        vec![
            UnitGroup {
                programs: vec![program(
                    "first",
                    &format!("touch {}; exec sleep 300", marker.display()),
                )],
                ..Default::default()
            },
            UnitGroup {
                programs: vec![program(
                    "second",
                    &format!("test -e {} && exec sleep 300", marker.display()),
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
        vec![UnitGroup {
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
    limited.cpus = "0".into();
    limited.memlock_bytes = 1024 * 1024;
    let plan = fixture.plan(
        "r1",
        vec![UnitGroup {
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
        vec![UnitGroup {
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
        vec![UnitGroup {
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
async fn same_group_starts_peers_before_waiting_for_readiness() {
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
        vec![UnitGroup {
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
        vec![UnitGroup {
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
        vec![UnitGroup {
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
    // Arrange: a native reference to an existing host target.
    let fixture = Fixture::new("native-references");
    let file = fixture.repo().join("references.service");
    let contents =
        "[Unit]\nRequires=basic.target\nAfter=basic.target\n[Service]\nExecStart=/bin/sleep 300\n";
    std::fs::write(&file, contents).unwrap();
    // Act
    let report = run_for_a_while(fixture.plan(
        "r1",
        vec![UnitGroup {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    ))
    .await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(std::fs::read_to_string(file).unwrap(), contents);
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
        vec![UnitGroup {
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
            vec![UnitGroup {
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
