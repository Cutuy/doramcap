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
        report.units.iter().all(|unit| unit.result == "success"),
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
        (fails.result.as_str(), fails.exec_main_status),
        ("exit-code", 3)
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
async fn the_next_run_resets_units_left_by_a_dropped_run() {
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
    // Act
    let report = run_for_a_while(fixture.plan("r2", vec![])).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(!active("doramcap-*-left.service").contains("running"));
}
