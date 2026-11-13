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
            host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
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

#[tokio::test(flavor = "current_thread")]
async fn compose_style_admission_refuses_run_and_cleanup_before_side_effects() {
    // Arrange: this is the admission and inheritance policy used by Compose.
    let fixture = Fixture::new("shared-admission");
    let admission = workload_lifecycle::admission::Admission::acquire(&fixture.repo()).unwrap();
    admission.keep_across_exec().unwrap();
    let (run, plan) = fixture.plan(
        "busy",
        vec![SystemdPlan {
            programs: vec![program("app", "exec sleep 300")],
            ..Default::default()
        }],
    );
    let artifacts = PathBuf::from(&run.artifacts);
    let mut runner = SystemdRunner::new();

    // Act
    let run_error = runner
        .run(
            run,
            plan,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
        )
        .await
        .unwrap_err();
    let cleanup_error = runner.cleanup(&fixture.repo()).await.unwrap_err();

    // Assert: refusal requires no system-manager connection or root privileges.
    assert_eq!(run_error, "another run owns this repository");
    assert_eq!(cleanup_error, run_error);
    assert!(!artifacts.exists());
    assert!(!runner.blocked());
}

/// Prebuilt Rust worker used by the host and by units with private roots.
fn shm_fixture() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let path = executable
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/systemd_shm_fixture");
    assert!(
        path.is_file(),
        "build workload-systemd --example systemd_shm_fixture first"
    );
    path
}

fn unit_shm_fixture(fixture: &Fixture) -> PathBuf {
    let path = fixture.root.join("shm-fixture");
    if !path.exists() {
        std::fs::copy(shm_fixture(), &path).unwrap();
    }
    path
}

fn shm_worker(id: &str, role: &str, key: u32, fixture: &Fixture) -> Program {
    sees(
        Program {
            id: id.into(),
            argv: vec![
                unit_shm_fixture(fixture).to_string_lossy().into(),
                role.into(),
                key.to_string(),
                fixture
                    .root
                    .join("producer-exited")
                    .to_string_lossy()
                    .into(),
                fixture
                    .root
                    .join("shared-data-read")
                    .to_string_lossy()
                    .into(),
            ],
            ..Default::default()
        },
        fixture,
    )
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and prebuilt --example systemd_shm_fixture"]
async fn blocked_shm_survives_producer_exit_and_is_removed_after_the_run() {
    // Arrange
    let fixture = Fixture::new("shared-memory");
    let key = std::process::id();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![
                shm_worker("producer", "producer", key, &fixture),
                shm_worker("consumer", "consumer", key, &fixture),
            ],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;

    // Act
    let (report, events) = observe_run_for_a_while((run, plan)).await;

    // Assert
    assert!(report.successful(), "{report:#?}");
    let prepared = events
        .iter()
        .find(|event| event.invocation.step == "systemd.prepare")
        .unwrap();
    let cgroups = events
        .iter()
        .find(|event| {
            event.invocation.step
                == <workload_lifecycle::cgroup::SystemAppsCgroups as orchestrator::Step>::ID
                    .unwrap()
        })
        .unwrap();
    let setup = events
        .iter()
        .find(|event| {
            event.invocation.step == "systemd.runtime.prepare"
                && event.state == orchestrator::State::Succeeded
        })
        .unwrap();
    let removal = events
        .iter()
        .find(|event| {
            event.invocation.step == "systemd.runtime.remove"
                && event.state == orchestrator::State::Succeeded
        })
        .unwrap();
    let mut dependencies = setup.invocation.dependencies.clone();
    dependencies.sort_unstable();
    let mut expected = [prepared.invocation.id, cgroups.invocation.id];
    expected.sort_unstable();
    assert_eq!(dependencies, expected);
    assert!(
        removal
            .invocation
            .dependencies
            .contains(&setup.invocation.id)
    );
    assert!(setup.elapsed_ns < removal.elapsed_ns);
    let directory = setup.payload.as_ref().unwrap()["directory"]
        .as_str()
        .unwrap();
    assert!(!Path::new(directory).join("shm").exists());
    assert!(
        report
            .units
            .iter()
            .all(|unit| unit.result.as_deref() == Some("success")),
        "{report:#?}"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("shared-data-read")).unwrap(),
        "Shared file read after producer exit"
    );
    // Act: inspect visibility from the host after the run stops.
    let status = Command::new(shm_fixture())
        .args(["absent", &key.to_string(), "unused", "unused"])
        .status()
        .unwrap();
    // Assert
    assert!(status.success(), "run shm became visible on the host");

    // Arrange: a new run checks for objects left by the first run.
    let (mut run, plan) = fixture.plan(
        "r2",
        vec![SystemdPlan {
            programs: vec![shm_worker("new-run", "absent", key, &fixture)],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
    // Act
    let report = run_for_a_while((run, plan)).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(
        report.units[0].result.as_deref(),
        Some("success"),
        "{report:#?}"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and prebuilt --example systemd_shm_fixture"]
async fn passthrough_generated_units_share_host_memory_and_preserve_objects() {
    // Arrange
    let fixture = Fixture::new("host-shared-memory");
    let key = std::process::id();
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![
                shm_worker("producer", "producer", key, &fixture),
                shm_worker("consumer", "consumer", key, &fixture),
            ],
            ..Default::default()
        }],
    );

    // Act: the host reads the shared file after all run units and resources have been removed.
    let report = run_for_a_while(plan).await;
    let host_read = Command::new(shm_fixture())
        .args([
            "consumer",
            &key.to_string(),
            &fixture.root.join("producer-exited").to_string_lossy(),
            &fixture.root.join("host-data-read").to_string_lossy(),
        ])
        .status()
        .unwrap();
    let cleanup = std::fs::remove_file(Path::new("/dev/shm").join(format!("doramcap-test-{key}")));

    // Assert
    cleanup.unwrap();
    assert!(report.successful(), "{report:#?}");
    assert!(
        report
            .units
            .iter()
            .all(|unit| unit.result.as_deref() == Some("success")),
        "{report:#?}"
    );
    assert!(host_read.success());
    assert!(fixture.root.join("shared-data-read").exists());
    assert!(fixture.root.join("host-data-read").exists());
}

async fn wait_for_file(path: &Path) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    true
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, the system service manager and prebuilt --example systemd_shm_fixture"]
async fn selective_host_shm_stays_live_with_native_permissions_in_both_modes() {
    // Arrange
    use workload_lifecycle::launch::HostShmAccess::{Blocked, Passthrough};
    let host = Fixture {
        root: Path::new("/dev/shm").join(format!("systemd-live-shm-{}", std::process::id())),
    };
    std::fs::create_dir(&host.root).unwrap();
    for access in [Blocked, Passthrough] {
        for read_only in [false, true] {
            let fixture = Fixture::new(&format!("live-{access:?}-{read_only}"));
            let source = host.root.join("camera");
            let alias = fixture.root.join("source-link");
            std::fs::write(&source, "initial").unwrap();
            std::os::unix::fs::symlink(&source, &alias).unwrap();
            let observed = fixture.root.join("initial-read");
            let done = fixture.root.join("live-read");
            let mut consumer = sees(
                Program {
                    id: "consumer".into(),
                    argv: vec![
                        unit_shm_fixture(&fixture).to_string_lossy().into(),
                        "live".into(),
                        "/input/camera".into(),
                        observed.to_string_lossy().into(),
                        done.to_string_lossy().into(),
                        read_only.to_string(),
                    ],
                    ..Default::default()
                },
                &fixture,
            );
            consumer.binds.push(Bind {
                source: alias.to_string_lossy().into(),
                target: "/input/camera".into(),
                read_only,
            });
            let (mut run, plan) = fixture.plan(
                "r1",
                vec![SystemdPlan {
                    programs: vec![consumer],
                    ..Default::default()
                }],
            );
            run.host_shm_access = access as i32;
            let (ready, readiness) = oneshot::channel();
            let stop = async {
                if readiness.await.is_ok() && wait_for_file(&observed).await {
                    std::fs::write(&source, "host update").unwrap();
                    wait_for_file(&done).await;
                }
            };

            // Act
            let report = SystemdRunner::new()
                .run(run, plan, stop, |_: &orchestrator::Event| {}, Some(ready))
                .await
                .unwrap();

            // Assert
            assert!(report.successful(), "{report:#?}");
            assert_eq!(
                std::fs::read_to_string(&done).unwrap(),
                "live mount verified"
            );
            assert_eq!(
                std::fs::read_to_string(&source).unwrap(),
                if read_only {
                    "host update"
                } else {
                    "app update"
                }
            );
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn blocked_native_conflicts_fail_before_any_app_starts() {
    // Arrange
    for (index, settings, expected) in [
        (
            0,
            "[Service]\nBindPaths=/dev/shm:/host-shm\n",
            "exposes whole host shm",
        ),
        (
            1,
            "[Service]\nBindReadOnlyPaths=/dev/shm:/host-shm\n",
            "exposes whole host shm",
        ),
    ] {
        let fixture = Fixture::new(&format!("native-shm-conflict-{index}"));
        let unit = fixture.repo().join("native.service");
        std::fs::write(&unit, format!("{settings}ExecStart=/bin/true\n")).unwrap();
        let marker = fixture.root.join("must-not-start");
        let writer = sees(
            program(
                "writer",
                &format!("touch {}; exec sleep 300", marker.display()),
            ),
            &fixture,
        );
        let (mut run, plan) = fixture.plan(
            "r1",
            vec![
                SystemdPlan {
                    programs: vec![writer],
                    ..Default::default()
                },
                SystemdPlan {
                    unit_files: vec![unit.to_string_lossy().into()],
                    ..Default::default()
                },
            ],
        );
        run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;

        // Act
        let (report, events) = observe_run_for_a_while((run, plan)).await;

        // Assert
        assert!(
            report
                .execution_error
                .as_deref()
                .is_some_and(|error| error.contains(expected)),
            "{report:#?}"
        );
        assert!(
            !marker.exists(),
            "an earlier phase started before native validation"
        );
        assert!(report.cleanup_verified, "{report:#?}");
        assert!(events.iter().any(|event| {
            event.invocation.step == "systemd.runtime.prepare"
                && event.state == orchestrator::State::Failed
                && event
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains(expected))
        }));
        assert!(
            events
                .iter()
                .any(|event| event.invocation.step == "systemd.runtime.remove"
                    && event.state == orchestrator::State::Succeeded)
        );
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn blocked_native_whole_host_mount_alias_fails_before_start() {
    // Arrange: an inode alias is enough; keep host submounts outside the test mount.
    use std::os::unix::fs::MetadataExt;
    let original_mounts: Vec<_> = ["/dev/shm", "/dev/pts", "/dev/mqueue", "/dev/hugepages"]
        .into_iter()
        .filter_map(|path| {
            std::fs::metadata(path)
                .ok()
                .map(|metadata| (path, metadata.dev(), metadata.ino()))
        })
        .collect();
    struct Alias(PathBuf);
    impl Drop for Alias {
        fn drop(&mut self) {
            if rustix::mount::unmount(&self.0, rustix::mount::UnmountFlags::empty()).is_ok() {
                let _ = std::fs::remove_dir(&self.0);
            }
        }
    }
    let alias = std::env::temp_dir().join(format!("systemd-host-dev-alias-{}", std::process::id()));
    std::fs::create_dir(&alias).unwrap();
    rustix::mount::mount_bind("/dev", &alias).unwrap();
    // Do not install an unmount guard until propagation is private.
    rustix::mount::mount_change(&alias, rustix::mount::MountPropagationFlags::PRIVATE).unwrap();
    let alias = Alias(alias);
    let fixture = Fixture::new("native-host-alias");
    let file = fixture.repo().join("native.service");
    let marker = fixture.root.join("must-not-start");
    std::fs::write(
        &file,
        format!(
            "[Service]\nBindPaths={}:/host-dev\nExecStart=/bin/touch {}\n",
            alias.0.display(),
            marker.display()
        ),
    )
    .unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.to_string_lossy().into()],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;

    // Act
    let report = run_for_a_while((run, plan)).await;
    let alias_path = alias.0.clone();
    drop(alias);

    // Assert
    assert!(!alias_path.exists(), "test alias cleanup failed");
    for (path, device, inode) in original_mounts {
        let metadata = std::fs::metadata(path).unwrap();
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            (device, inode),
            "host mount changed: {path}"
        );
    }
    assert!(
        report
            .execution_error
            .as_deref()
            .is_some_and(|error| error.contains("exposes whole host shm")),
        "{report:#?}"
    );
    assert!(!marker.exists());
    assert!(report.cleanup_verified, "{report:#?}");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn native_default_and_private_ipc_units_share_run_filesystem() {
    // Arrange
    let fixture = Fixture::new("native-shm-private");
    let producer = fixture.repo().join("producer.service");
    let consumer = fixture.repo().join("consumer.service");
    let output = fixture.root.join("observed");
    let exchange = format!("/dev/shm/doramcap-native-exchange-{}", std::process::id());
    std::fs::write(
        &producer,
        format!("[Service]\nType=oneshot\nExecStart=/bin/sh -c 'echo native-data > {exchange}'\n"),
    )
    .unwrap();
    std::fs::write(
        &consumer,
        format!(
            "[Service]\nType=oneshot\nExecStart=/bin/sh -c 'cat {exchange} > {}'\n",
            output.display()
        ),
    )
    .unwrap();
    let drop_ins = PathBuf::from(format!("{}.d", consumer.display()));
    std::fs::create_dir(&drop_ins).unwrap();
    std::fs::write(
        drop_ins.join("10-private.conf"),
        "[Service]\nPrivateIPC=yes\n",
    )
    .unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![
            SystemdPlan {
                unit_files: vec![producer.to_string_lossy().into()],
                ..Default::default()
            },
            SystemdPlan {
                unit_files: vec![consumer.to_string_lossy().into()],
                ..Default::default()
            },
        ],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;

    // Act
    let report = run_for_a_while((run, plan)).await;

    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(
        report
            .units
            .iter()
            .all(|unit| unit.result.as_deref() == Some("success")),
        "{report:#?}"
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), "native-data\n");
    assert!(!Path::new(&exchange).exists());
    assert!(
        std::fs::read_to_string(&consumer)
            .unwrap()
            .starts_with("[Service]\nType=oneshot\n")
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn native_explicit_private_shm_mount_is_preserved() {
    // Arrange
    let fixture = Fixture::new("native-private-shm");
    let file = fixture.repo().join("private.service");
    let name = format!("/dev/shm/doramcap-native-private-{}", std::process::id());
    let private_done = fixture.root.join("private-done");
    std::fs::write(&file, format!("[Service]\nTemporaryFileSystem=/dev/shm\nType=oneshot\nExecStart=/bin/sh -c 'echo private > {name}; echo complete > {}'\n", private_done.display())).unwrap();
    let observed = fixture.root.join("observed");
    let consumer = sees(
        program(
            "consumer",
            &format!("test ! -e {name} && echo isolated > {}", observed.display()),
        ),
        &fixture,
    );
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![
            SystemdPlan {
                unit_files: vec![file.to_string_lossy().into()],
                ..Default::default()
            },
            SystemdPlan {
                programs: vec![consumer],
                ..Default::default()
            },
        ],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;

    // Act
    let report = run_for_a_while((run, plan)).await;

    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(std::fs::read_to_string(private_done).unwrap(), "complete\n");
    assert_eq!(std::fs::read_to_string(observed).unwrap(), "isolated\n");
    assert!(!Path::new(&name).exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn passthrough_preserves_native_whole_shm_bind() {
    // Arrange
    let fixture = Fixture::new("native-passthrough");
    let file = fixture.repo().join("host.service");
    let name = format!("doramcap-native-host-{}", std::process::id());
    let target = fixture.root.join("host-shm");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(&file, format!("[Service]\nBindPaths=/dev/shm:{}\nType=oneshot\nExecStart=/bin/sh -c 'echo host-data > {}/{name}'\n", target.display(), target.display())).unwrap();
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.to_string_lossy().into()],
            ..Default::default()
        }],
    );

    // Act
    let report = run_for_a_while(plan).await;
    let host_file = Path::new("/dev/shm").join(name);
    let payload = std::fs::read_to_string(&host_file);
    let _ = std::fs::remove_file(host_file);

    // Assert
    assert!(report.successful(), "{report:#?}");
    assert_eq!(payload.unwrap(), "host-data\n");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn stop_at_runtime_preparation_keeps_cleanup_observable() {
    use std::{cell::RefCell, rc::Rc};
    // Arrange: stop as runtime preparation begins, before any app is launched.
    let fixture = Fixture::new("runtime-cancel");
    let unit = fixture.repo().join("native.service");
    let marker = fixture.root.join("must-not-start");
    std::fs::write(
        &unit,
        format!("[Service]\nExecStart=/usr/bin/touch {}\n", marker.display()),
    )
    .unwrap();
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![unit.to_string_lossy().into()],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
    let (stop, stopped) = oneshot::channel();
    let stop = RefCell::new(Some(stop));
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();

    // Act
    let report = SystemdRunner::new()
        .run(
            run,
            plan,
            async {
                let _ = stopped.await;
            },
            move |event: &orchestrator::Event| {
                observed.borrow_mut().push(event.clone());
                if event.invocation.step == "systemd.runtime.prepare"
                    && event.state == orchestrator::State::Started
                {
                    let _ = stop.borrow_mut().take().unwrap().send(());
                }
            },
            None,
        )
        .await
        .unwrap();

    // Assert: cancellation keeps the pre-owned scope available to the cleanup step.
    assert!(report.cleanup_verified, "{report:#?}");
    assert!(!marker.exists());
    assert!(report.units.is_empty());
    let events = events.borrow();
    assert!(
        events
            .iter()
            .any(|event| event.invocation.step == "systemd.runtime.prepare"
                && matches!(
                    event.state,
                    orchestrator::State::Cancelled | orchestrator::State::Succeeded
                ))
    );
    let removal = events
        .iter()
        .find(|event| {
            event.invocation.step == "systemd.runtime.remove"
                && event.state == orchestrator::State::Started
        })
        .unwrap();
    let directory = removal.payload.as_ref().unwrap()["directory"]
        .as_str()
        .unwrap();
    assert!(!Path::new(directory).join("shm").exists());
    assert!(
        events
            .iter()
            .any(|event| event.invocation.step == "systemd.runtime.remove"
                && event.state == orchestrator::State::Succeeded)
    );
}

/// Run `plan`, stop it `OBSERVATION` after readiness, and return its report.
/// Observe transient unit creation throughout Dora execution and teardown.
async fn run_without_control_units(plan: (RunPlan, SystemdPlan)) -> SystemdRunReport {
    use futures_util::{FutureExt, StreamExt};
    let connection = zbus::Connection::system().await.unwrap();
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&connection)
        .await
        .unwrap();
    manager.subscribe().await.unwrap();
    let mut created = manager.receive_unit_new().await.unwrap();
    let mut observed = Vec::new();
    let run = run_for_a_while(plan);
    tokio::pin!(run);
    loop {
        tokio::select! {
            report = &mut run => {
                // A manager reply orders already-emitted unit signals before the final check.
                manager.list_units().await.unwrap();
                while let Some(Some(event)) = created.next().now_or_never() { observed.push(event.args().unwrap().id.to_owned()); }
                let prefix = report.units.iter().find_map(|unit| unit.unit.strip_suffix("-dora-coordinator.service")).expect("coordinator exit evidence");
                assert!(!observed.iter().any(|name| name.starts_with(&format!("{prefix}-dora-cli-"))), "transient control units: {observed:?}");
                return report;
            },
            event = created.next() => {
                let event = event.expect("systemd unit event stream ended");
                let arguments = event.args().unwrap();
                observed.push(arguments.id.to_owned());
            }
        }
    }
}

async fn run_for_a_while((run, plan): (RunPlan, SystemdPlan)) -> SystemdRunReport {
    observe_run_for_a_while((run, plan)).await.0
}

async fn observe_run_for_a_while(
    (run, plan): (RunPlan, SystemdPlan),
) -> (SystemdRunReport, Vec<orchestrator::Event>) {
    use std::{cell::RefCell, rc::Rc};
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let (ready, readiness) = oneshot::channel();
    let stop = async {
        if readiness.await.is_ok() {
            tokio::time::sleep(OBSERVATION).await;
        }
    };
    let report = SystemdRunner::new()
        .run(
            run,
            plan,
            stop,
            move |event: &orchestrator::Event| observed.borrow_mut().push(event.clone()),
            Some(ready),
        )
        .await
        .unwrap();
    let captured = events.borrow().clone();
    (report, captured)
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
async fn user_apps_run_in_the_user_slice_unless_their_unit_sets_its_own() {
    // Arrange
    let fixture = Fixture::new("user-slice");
    let own = format!("own{}.slice", std::process::id());
    let generated = fixture.root.join("generated-cgroup");
    let adopted_cgroup = fixture.root.join("adopted-cgroup");
    let adopted = fixture.repo().join("adopted.service");
    std::fs::write(
        &adopted,
        format!(
            "[Service]\nSlice={own}\nExecStart=/bin/sh -c 'cat /proc/self/cgroup > {}; exec sleep 300'\n",
            adopted_cgroup.display()
        ),
    )
    .unwrap();
    let plan = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![sees(
                program(
                    "reporter",
                    &format!(
                        "cat /proc/self/cgroup > {}; exec sleep 300",
                        generated.display()
                    ),
                ),
                &fixture,
            )],
            unit_files: vec![adopted.to_string_lossy().into()],
            ..Default::default()
        }],
    );
    // Act
    let report = run_for_a_while(plan).await;
    let _ = Command::new("systemctl").args(["stop", &own]).output();
    // Assert
    assert!(report.successful(), "{report:#?}");
    let generated = std::fs::read_to_string(generated).unwrap();
    assert!(
        generated.contains("-user.slice/doramcap-")
            && generated.trim_end().ends_with("-reporter.service"),
        "{generated}"
    );
    let adopted = std::fs::read_to_string(adopted_cgroup).unwrap();
    assert!(adopted.starts_with(&format!("0::/{own}/")), "{adopted}");
    let prefix = report
        .units
        .iter()
        .find_map(|unit| unit.unit.strip_suffix("-reporter.service"))
        .unwrap();
    let remaining = active(&format!("{prefix}*"));
    assert!(remaining.trim().is_empty(), "{remaining}");
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
    // Assert: the run slice and its system-app slice leave no cgroups behind.
    let identity = std::fs::read(
        fixture
            .root
            .join("r1")
            .join(workload_lifecycle::cgroup::IDENTITY_FILE),
    )
    .unwrap();
    let identity = <workload_lifecycle::cgroup::SystemAppCgroup as prost::Message>::decode(
        identity.as_slice(),
    )
    .unwrap();
    let system = Path::new(&identity.path);
    for path in [system, system.parent().unwrap()] {
        assert!(
            !path.exists(),
            "accounting cgroup remains: {}",
            path.display()
        );
    }
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
async fn next_admission_cleans_abandoned_units() {
    // Arrange: a run dropped after readiness leaves its unit running.
    let fixture = Fixture::new("dropped");
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            programs: vec![program("left", "exec sleep 300")],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
    let (ready, readiness) = oneshot::channel();
    let mut runner = SystemdRunner::new();
    tokio::select! {
        _ = runner.run(run, plan, std::future::pending(), |_: &orchestrator::Event| {}, Some(ready)) => {
            panic!("run ended without stop")
        }
        _ = readiness => {}
    }
    assert!(active("doramcap-*-left.service").contains("active running"));
    // Act: the next admission stops the abandoned run and exports its saved identities.
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
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
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
    let (mut run, plan) = fixture.plan(
        "r1",
        vec![SystemdPlan {
            unit_files: vec![file.display().to_string()],
            ..Default::default()
        }],
    );
    run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
    // Act
    let report = run_for_a_while((run, plan)).await;
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
    let prefix = report.units[0].unit.strip_prefix("doramcap-").unwrap();
    let scope = Path::new("/run/doramcap").join(prefix.strip_suffix("-infinite.service").unwrap());
    assert!(scope.join("shm").is_dir());
    // Act: wait for the already requested stop, then clean up explicitly.
    let status = Command::new("systemctl")
        .args(["stop", &report.units[0].unit])
        .status()
        .unwrap();
    assert!(status.success());
    SystemdRunner::new().cleanup(&fixture.repo()).await.unwrap();
    // Assert
    assert!(fixture.root.join("r1/report.json").exists());
    assert!(!scope.join("shm").exists());
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
    let shared_name = format!("/dev/shm/doramcap-isolation-{}", std::process::id());
    let make_plan = |fixture: &Fixture, id: &str| {
        let file = fixture.repo().join("isolated.service");
        std::fs::write(&file, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
        let (mut run, plan) = fixture.plan(
            id,
            vec![SystemdPlan {
                unit_files: vec![file.display().to_string()],
                programs: vec![sees(
                    program(
                        "shm-isolation",
                        &format!(
                            "printf '%s' '{}' > {}; touch {}; while [ ! -e {} ]; do sleep 0.01; done; cat {} > {}; exec sleep 300",
                            fixture.root.display(),
                            shared_name,
                            fixture.root.join("memory-ready").display(),
                            fixture.root.join("verify-memory").display(),
                            shared_name,
                            fixture.root.join("memory-observed").display(),
                        ),
                    ),
                    fixture,
                ), shm_worker("producer", "producer", std::process::id(), fixture), shm_worker("consumer", "consumer", std::process::id(), fixture)],
                ..Default::default()
            }],
        );
        run.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
        (run, plan)
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
        // Both active runs can use the same shared-file name without sharing its contents.
        for fixture in [&first, &second] {
            assert!(wait_for_file(&fixture.root.join("memory-ready")).await);
        }
        for fixture in [&first, &second] {
            std::fs::write(fixture.root.join("verify-memory"), "").unwrap();
        }
        for fixture in [&first, &second] {
            let observed = fixture.root.join("memory-observed");
            assert!(wait_for_file(&observed).await);
            assert_eq!(
                std::fs::read_to_string(observed).unwrap(),
                fixture.root.to_string_lossy()
            );
        }
        assert!(!Path::new(&shared_name).exists());
        for fixture in [&first, &second] {
            assert!(wait_for_file(&fixture.root.join("shared-data-read")).await);
        }
        assert!(
            Command::new(shm_fixture())
                .args([
                    "absent",
                    &std::process::id().to_string(),
                    "unused",
                    "unused"
                ])
                .status()
                .unwrap()
                .success()
        );
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
    plan.0.host_shm_access = workload_lifecycle::launch::HostShmAccess::Blocked as i32;
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
    // Native project units preserve their filesystem; app units receive the shared config bind.
    let native = fixture.root.join("native.service");
    std::fs::write(&native, format!(r#"[Service]
Environment=DORA_NODE_ID=native-chosen
ExecStart=/bin/sh -c 'printf "%%s\\n" "$$DORA_NODE_ID" "$$DORA_DAEMON_LOCAL_LISTEN_PORT" "$$DORA_ZENOH_CONFIG_OVERLAY" > {}; test -s "$$DORA_ZENOH_CONFIG_OVERLAY" || exit 42; exec sleep 300'
"#, runtime.join("native-environment").display())).unwrap();
    plan.1
        .unit_files
        .push(native.to_string_lossy().into_owned());
    let adopted = fixture.root.join("participant.service");
    std::fs::write(&adopted, r#"[Service]
Environment=DORA_NODE_ID=app-chosen
ExecStart=/bin/sh -c 'printf "%%s\\n" "$$DORA_NODE_ID" "$$DORA_DAEMON_LOCAL_LISTEN_PORT" "$$DORA_ZENOH_CONFIG_OVERLAY" > /results/app-environment; test -s "$$DORA_ZENOH_CONFIG_OVERLAY" || exit 42; exec sleep 300'
"#).unwrap();
    plan.1.binaries = "/bin".into();
    plan.1.app_units.push(workload_systemd::AppUnit {
        id: "adopted".into(),
        file: adopted.to_string_lossy().into_owned(),
        binds: vec![Bind {
            source: runtime.to_string_lossy().into_owned(),
            target: "/results".into(),
            read_only: false,
        }],
    });
    plan.0.artifacts = fixture
        .root
        .join("artifacts space%quote\"slash\\")
        .to_string_lossy()
        .into_owned();
    let mut changed_plan = plan.clone();
    // Act
    let report = run_without_control_units(plan).await;
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
    for (name, node) in [
        ("native-environment", "native-chosen"),
        ("app-environment", "app-chosen"),
    ] {
        let text = std::fs::read_to_string(runtime.join(name)).unwrap();
        let participant: Vec<_> = text.lines().collect();
        assert_eq!(participant[0], node);
        assert_eq!(participant[1], values[1]);
        if name == "app-environment" {
            assert_eq!(participant[2], values[2]);
        } else {
            assert!(Path::new(participant[2]).is_absolute());
        }
    }
    assert!(runtime.join("dataflow.yml").is_file());
    assert!(
        runtime
            .join(workload_lifecycle::dora::SESSION_FILE)
            .is_file()
    );
    assert!(std::fs::read_dir(&runtime).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("cli-")
    }));
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
    // Arrange: edit the captured graph after preparation has reported success.
    let session_before =
        std::fs::read(runtime.join(workload_lifecycle::dora::SESSION_FILE)).unwrap();
    changed_plan.0.id = "changed".into();
    changed_plan.0.artifacts = fixture
        .root
        .join("changed-artifacts")
        .to_string_lossy()
        .into_owned();
    let artifacts = PathBuf::from(&changed_plan.0.artifacts);
    let (ready, readiness) = oneshot::channel();
    let stop = async {
        if readiness.await.is_ok() {
            tokio::time::sleep(OBSERVATION).await;
        }
    };
    // Act
    let rejected = SystemdRunner::new()
        .run(
            changed_plan.0,
            changed_plan.1,
            stop,
            move |event: &orchestrator::Event| {
                if event.invocation.step == "systemd.prepare"
                    && event.state == orchestrator::State::Succeeded
                {
                    let captured: Vec<_> = std::fs::read_dir(artifacts.join("inputs"))
                        .unwrap()
                        .map(|entry| entry.unwrap().path().join("dataflow"))
                        .filter(|path| path.is_file())
                        .collect();
                    assert_eq!(captured.len(), 1);
                    std::fs::write(
                        &captured[0],
                        r#"{"nodes":[{"id":"user","path":"dynamic","outputs":["changed"]}]}"#,
                    )
                    .unwrap();
                }
            },
            Some(ready),
        )
        .await
        .unwrap();
    // Assert: no graph/session mutation precedes rejection; native resources still exit.
    assert!(
        rejected
            .execution_error
            .as_deref()
            .is_some_and(|message| message.contains("frozen Dora graph changed")),
        "{rejected:#?}"
    );
    assert!(
        rejected.writers_stopped && rejected.cleanup_verified,
        "{rejected:#?}"
    );
    assert_eq!(
        std::fs::read(runtime.join(workload_lifecycle::dora::SESSION_FILE)).unwrap(),
        session_before
    );
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
                .replace("6014", &daemon)
                + "\nNetworkNamespacePath=/proc/1/ns/net\n",
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
    let mut participant = program(
        "user",
        r#"printf '%s\n' "$DORA_DAEMON_LOCAL_LISTEN_PORT" "$DORA_ZENOH_CONFIG_OVERLAY" > /results/environment; test -s "$DORA_ZENOH_CONFIG_OVERLAY" || exit 42; exec sleep 300"#,
    );
    participant.binds.push(Bind {
        source: runtime.to_string_lossy().into_owned(),
        target: "/results".into(),
        read_only: false,
    });
    plan.1.programs.push(participant);
    drop(ports);
    // Act
    let mut isolated_plan = plan.clone();
    let report = run_without_control_units(plan).await;
    // Assert
    assert!(report.successful(), "{report:#?}");
    let environment = std::fs::read_to_string(runtime.join("environment")).unwrap();
    assert_eq!(
        environment.lines().collect::<Vec<_>>(),
        [daemon.as_str(), "/config/zenoh.json5"]
    );
    assert!(runtime.join("dataflow.yml").is_file());
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
    // Arrange: the same native unit explicitly requests an isolated network.
    let session_before =
        std::fs::read(runtime.join(workload_lifecycle::dora::SESSION_FILE)).unwrap();
    let coordinator_file = fixture.root.join("dora-coordinator.service");
    let isolated = std::fs::read_to_string(&coordinator_file)
        .unwrap()
        .replace("NetworkNamespacePath=/proc/1/ns/net", "PrivateNetwork=yes");
    std::fs::write(coordinator_file, isolated).unwrap();
    isolated_plan.0.id = "isolated".into();
    isolated_plan.0.artifacts = fixture
        .root
        .join("isolated-artifacts")
        .to_string_lossy()
        .into_owned();
    // Act
    let rejected = run_for_a_while(isolated_plan).await;
    // Assert: rejection precedes Start/session mutation, and every owned unit is stopped.
    assert!(
        rejected
            .execution_error
            .as_deref()
            .is_some_and(|message| message.contains("host network")),
        "{rejected:#?}"
    );
    assert!(
        rejected.writers_stopped && rejected.cleanup_verified,
        "{rejected:#?}"
    );
    assert_eq!(
        std::fs::read(runtime.join(workload_lifecycle::dora::SESSION_FILE)).unwrap(),
        session_before
    );
    assert!(runtime_units().is_empty(), "{:?}", runtime_units());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn bind_mounts_show_host_paths_at_container_targets() {
    // Arrange: the unit reads its config and writes its result at container paths.
    let fixture = Fixture::new("binds");
    let config = fixture.root.join("monitor.pbtxt");
    let results = fixture.root.join("results space:quote\"slash\\%n\n\t");
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
    // Runtime capture preserves artifact roots requiring native unit-file escaping.
    plan.0.artifacts = fixture
        .root
        .join(concat!(
            r#"artifact space:slash\quote"percent%n"#,
            "\n\tend"
        ))
        .to_string_lossy()
        .into();
    let artifacts = PathBuf::from(&plan.0.artifacts);
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
    // Cleanup unlinks runtime aliases and keeps their captured artifact targets.
    fn contains_config(directory: &Path) -> bool {
        std::fs::read_dir(directory).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                contains_config(&path)
            } else {
                std::fs::read(path).unwrap() == b"rate: 5\n"
            }
        })
    }
    assert!(contains_config(&artifacts));
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
    use std::{cell::RefCell, rc::Rc};
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
    let expected = workload_systemd::inspect(&run, &plan).await.unwrap();
    std::fs::write(&file, "[Service]\nExecStart=/bin/false\n").unwrap();
    let accepted = std::cell::Cell::new(false);
    let mut runner = SystemdRunner::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    // Act
    let result = runner
        .run_with_preflight(
            run.clone(),
            plan.clone(),
            std::future::pending(),
            move |event: &orchestrator::Event| observed.borrow_mut().push(event.clone()),
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
    {
        let events = events.borrow();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].invocation.step, "systemd.prepare");
        assert_eq!(events[1].state, orchestrator::State::Succeeded);
        assert_eq!(events[2].invocation.step, "systemd.discard");
        assert_eq!(events[3].state, orchestrator::State::Succeeded);
        assert_eq!(events[2].invocation.dependencies, [events[0].invocation.id]);
    }
    // Act / Assert: cancellation at the same admission boundary also starts nothing.
    let expected = workload_systemd::inspect(&run, &plan).await.unwrap();
    let result = runner
        .run_with_preflight(
            run.clone(),
            plan.clone(),
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
    // Act: a replacement at the owned artifact path prevents cleanup after rejection.
    let result = runner
        .run_with_preflight(
            run.clone(),
            plan,
            std::future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |_| {
                std::fs::remove_dir_all(&run.artifacts).unwrap();
                std::fs::write(&run.artifacts, "replacement").unwrap();
                Err("Rejected review".into())
            },
        )
        .await;
    // Assert: both causes survive, and the replacement is not removed.
    let error = result.unwrap_err();
    assert!(error.starts_with("Rejected review; could not remove rejected preparation"));
    assert_eq!(
        std::fs::read_to_string(&run.artifacts).unwrap(),
        "replacement"
    );
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
    let expected = workload_systemd::inspect(&run, &plan).await.unwrap();
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
    assert_ne!(
        expected,
        workload_systemd::inspect(&run, &plan).await.unwrap()
    );
    assert!(runtime_units().is_empty());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and the system service manager"]
async fn system_app_restart_stays_beneath_the_run_accounting_boundary() {
    use orchestrator::{Event, State, Step};
    use prost::Message;
    use std::os::unix::fs::MetadataExt;
    use std::{cell::RefCell, rc::Rc};
    use workload_lifecycle::cgroup::{Action, SystemAppsCgroups, SystemAppsCgroupsInput};
    use workload_lifecycle::launch::SystemAppCgroup;
    // Arrange
    let fixture = Fixture::new("cgroup-restart");
    let unit = fixture.repo().join("restart.service");
    std::fs::write(&unit, "[Service]\nExecStart=/bin/sleep 300\n").unwrap();
    let (run, plan) = fixture.plan(
        "r1",
        vec![
            SystemdPlan {
                unit_files: vec![unit.to_string_lossy().into()],
                ..Default::default()
            },
            SystemdPlan::default(),
        ],
    );
    let artifacts = run.artifacts.clone();
    let mut runner = SystemdRunner::new();
    let (ready, started) = oneshot::channel();
    let (stop, stopped) = oneshot::channel();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut running = Box::pin(runner.run(
        run,
        plan,
        async {
            let _ = stopped.await;
        },
        move |event: &Event| observed.borrow_mut().push(event.clone()),
        Some(ready),
    ));
    tokio::select! { ready = started => ready.unwrap(), report = &mut running => panic!("startup failed: {report:?}") };
    let identity = SystemAppCgroup::decode(
        std::fs::read(Path::new(&artifacts).join(workload_lifecycle::cgroup::IDENTITY_FILE))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let name = format!(
        "{}-restart.service",
        Path::new(&identity.path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix("-system.slice")
            .unwrap()
    );
    // Act
    assert!(
        Command::new("systemctl")
            .args(["restart", &name])
            .status()
            .unwrap()
            .success()
    );
    let observed = Command::new("systemctl")
        .args(["show", "-p", "MainPID", "--value", &name])
        .output()
        .unwrap();
    let pid = String::from_utf8(observed.stdout).unwrap();
    let membership = std::fs::read_to_string(format!("/proc/{}/cgroup", pid.trim())).unwrap();
    let group = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::/"))
        .unwrap();
    // Assert
    assert!(
        Path::new(workload_lifecycle::cgroup::CGROUP_ROOT)
            .join(group)
            .starts_with(&identity.path)
    );
    assert_eq!(
        std::fs::metadata(&identity.path).unwrap().ino(),
        identity.id
    );
    // Act
    stop.send(()).unwrap();
    let report = running.await.unwrap();
    // Assert
    assert!(report.successful(), "{report:#?}");
    assert!(!Path::new(&identity.path).exists());
    let events = events.borrow();
    let prepared = events
        .iter()
        .find(|event| {
            event.invocation.step
                == workload_systemd::PrepareRun::ID
                    .unwrap_or(std::any::type_name::<workload_systemd::PrepareRun>())
                && event.state == State::Succeeded
        })
        .unwrap();
    let cgroups: Vec<_> = events
        .iter()
        .filter(|event| {
            event.invocation.step == SystemAppsCgroups::ID.unwrap() && event.state == State::Started
        })
        .collect();
    assert_eq!(cgroups.len(), 2);
    assert!(
        cgroups[0]
            .invocation
            .dependencies
            .contains(&prepared.invocation.id)
    );
    let actions: Vec<_> = cgroups
        .iter()
        .map(|event| {
            serde_json::from_value::<SystemAppsCgroupsInput>(event.payload.clone().unwrap())
                .unwrap()
                .action
        })
        .collect();
    assert_eq!(
        actions,
        [i32::from(Action::Create), i32::from(Action::Remove)]
    );
    let removal = events
        .iter()
        .find(|event| {
            event.invocation.step == "systemd.runtime.remove" && event.state == State::Started
        })
        .unwrap();
    assert_eq!(cgroups[1].invocation.parent, Some(removal.invocation.id));
}
