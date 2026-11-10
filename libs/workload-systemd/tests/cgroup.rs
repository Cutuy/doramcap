//! Native system-app accounting boundary checks. Build unprivileged and elevate this executable.
use orchestrator::{Event, LocalExecutor, State};
use std::{
    cell::RefCell,
    fs,
    io::Write,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    time::Duration,
};
use workload_systemd::cgroup::{
    self, Action, IDENTITY_FILE, SystemAppSlice, SystemAppsCgroups, SystemAppsCgroupsInput,
};

const STARTUP: Duration = Duration::from_secs(10);
const PAYLOAD_BYTES: u64 = 8 * 1024 * 1024;
const CHECK_INTERVAL: Duration = Duration::from_millis(10);

struct Cleanup {
    directory: PathBuf,
    unit: String,
    slice: String,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        let parent = format!(
            "{}.slice",
            self.slice.strip_suffix("-system.slice").unwrap()
        );
        let _ = Command::new("systemctl")
            .args(["stop", &self.unit, &self.slice, &parent])
            .output();
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn cpu(path: &Path) -> u64 {
    fs::read_to_string(path.join("cpu.stat"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("usage_usec "))
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, systemd, cgroup v2 and prebuilt --example cgroup_fixture"]
async fn nested_apps_are_accounted_and_live_removal_is_refused() {
    // Arrange
    let directory = std::env::temp_dir().join(format!("system-app-cgroup-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut slice = SystemAppSlice::connect(&directory).await.unwrap();
    let unit = format!("cgroup-fixture-{}.service", std::process::id());
    let cleanup = Cleanup {
        directory: directory.clone(),
        unit: unit.clone(),
        slice: slice.name(),
    };
    slice.create(&directory, STARTUP).await.unwrap();
    let identity = slice.identity().unwrap();
    let parent = Path::new(&identity.path);
    let before = cpu(parent);
    let marker = directory.join("ready");
    let socket = directory.join("release");
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/cgroup_fixture");
    assert!(worker.is_file(), "build --example cgroup_fixture first");

    // Act
    let started = Command::new("systemd-run")
        .args([
            "--quiet",
            "--collect",
            "--property=Type=exec",
            "--property=Delegate=yes",
            "--unit",
            &unit,
            "--slice",
            &slice.name(),
        ])
        .arg(worker)
        .arg(&marker)
        .arg(&socket)
        .output()
        .unwrap();
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    tokio::time::timeout(STARTUP, async {
        while !marker.exists() {
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    let pid: u32 = fs::read_to_string(&marker).unwrap().parse().unwrap();
    slice.verify_pid(pid).unwrap();
    let memory: u64 = fs::read_to_string(parent.join("memory.current"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    // Assert: the parent's own process list is empty while nested work is included.
    assert!(
        fs::read_to_string(parent.join("cgroup.procs"))
            .unwrap()
            .trim()
            .is_empty()
    );
    assert!(cpu(parent) > before);
    assert!(memory >= PAYLOAD_BYTES, "parent charged {memory} bytes");
    assert!(
        fs::read_to_string(directory.join(IDENTITY_FILE))
            .unwrap()
            .contains(&identity.id.to_string())
    );
    assert!(slice.verify_pid(std::process::id()).is_err());
    assert!(slice.remove().await.unwrap_err().contains("live processes"));
    assert!(Path::new(&format!("/proc/{pid}")).exists());

    // Act
    UnixStream::connect(&socket)
        .unwrap()
        .write_all(&[1])
        .unwrap();
    tokio::time::timeout(STARTUP, async {
        while Path::new(&format!("/proc/{pid}")).exists() {
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    slice.remove().await.unwrap();
    slice.remove().await.unwrap();

    // Assert
    assert!(!parent.exists());
    assert!(directory.join(IDENTITY_FILE).is_file());
    drop(cleanup);
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, systemd and cgroup v2"]
async fn foreign_slice_is_preserved() {
    use futures_util::StreamExt;
    use zbus::{OwnedVariant, Variant};
    use zbus_systemd::systemd1::{ManagerProxy, UnitProxy};
    // Arrange
    let directory =
        std::env::temp_dir().join(format!("system-app-cgroup-recovery-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut slice = SystemAppSlice::connect(&directory).await.unwrap();
    let _cleanup = Cleanup {
        directory: directory.clone(),
        unit: "cgroup-fixture-absent.service".into(),
        slice: slice.name(),
    };
    let connection = zbus::Connection::system().await.unwrap();
    let manager = ManagerProxy::new(&connection).await.unwrap();
    manager.subscribe().await.unwrap();
    let mut events = manager.receive_job_removed().await.unwrap();
    let foreign = "Foreign accounting slice";
    let job = manager
        .start_transient_unit(
            slice.name(),
            "fail".into(),
            vec![(
                "Description".into(),
                OwnedVariant::try_from(Variant::from(foreign.to_owned())).unwrap(),
            )],
            vec![],
        )
        .await
        .unwrap();
    while let Some(signal) = events.next().await {
        if *signal.args().unwrap().job == *job {
            break;
        }
    }
    // Act / Assert
    assert!(slice.create(&directory, STARTUP).await.is_err());
    assert!(slice.remove().await.unwrap_err().contains("not owned"));
    let unit = UnitProxy::builder(&connection)
        .path(manager.get_unit(slice.name()).await.unwrap())
        .build()
        .await
        .unwrap();
    assert_eq!(unit.description().await.unwrap(), foreign);
    assert_eq!(unit.active_state().await.unwrap(), "active");

    // Assert: the fixture guard removes only this test's foreign resources.
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, systemd and cgroup v2"]
async fn empty_owned_slices_are_recoverable_and_names_can_be_reused() {
    // Arrange
    let directory =
        std::env::temp_dir().join(format!("system-app-cgroup-owned-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut slice = SystemAppSlice::connect(&directory).await.unwrap();
    let _cleanup = Cleanup {
        directory: directory.clone(),
        unit: "cgroup-fixture-absent.service".into(),
        slice: slice.name(),
    };
    slice.create(&directory, STARTUP).await.unwrap();
    let identity = slice.identity().unwrap();
    // Act: a new connection recovers the abandoned accounting boundary.
    drop(slice);
    let mut recovered = SystemAppSlice::connect(&directory).await.unwrap();
    recovered.remove().await.unwrap();
    // Assert
    assert!(!Path::new(&identity.path).exists());
    assert!(directory.join(IDENTITY_FILE).exists());
    // Act / Assert: a subsequent run can create the same ownership names immediately.
    recovered.create(&directory, STARTUP).await.unwrap();
    assert_eq!(recovered.identity().unwrap().slice, identity.slice);
    assert_ne!(recovered.identity().unwrap().id, identity.id);
    recovered.remove().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, systemd and cgroup v2"]
async fn cancellation_after_parent_creation_retains_cleanup_ownership() {
    use futures_util::StreamExt;
    use zbus_systemd::systemd1::ManagerProxy;
    // Arrange: subscribe before the first possible slice side effect.
    let directory =
        std::env::temp_dir().join(format!("system-app-cgroup-cancel-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let slice = SystemAppSlice::connect(&directory).await.unwrap();
    let _cleanup = Cleanup {
        directory: directory.clone(),
        unit: "cgroup-fixture-absent.service".into(),
        slice: slice.name(),
    };
    let parent = format!(
        "{}.slice",
        slice.name().strip_suffix("-system.slice").unwrap()
    );
    let connection = zbus::Connection::system().await.unwrap();
    let manager = ManagerProxy::new(&connection).await.unwrap();
    manager.subscribe().await.unwrap();
    let mut created = manager.receive_unit_new().await.unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut executor =
        LocalExecutor::new(move |event: &Event| observed.borrow_mut().push(event.clone()));
    cgroup::register(&mut executor, slice.clone());
    {
        let creation = executor.run::<SystemAppsCgroups>(SystemAppsCgroupsInput {
            action: Action::Create.into(),
            artifacts: directory.to_string_lossy().into_owned(),
            readiness_seconds: STARTUP.as_secs(),
        });
        tokio::pin!(creation);
        // Act: drop the creation future after observing its first owned unit.
        tokio::time::timeout(STARTUP, async {
        tokio::select! {
            biased;
            result = async {
                while let Some(signal) = created.next().await {
                    if signal.args().unwrap().id == parent { return; }
                }
                panic!("unit creation notifications ended");
            } => result,
            result = &mut creation => panic!("creation completed before cancellation: {result:?}"),
        }
    })
    .await
    .unwrap();
    }
    // The original handle retains its connection references after cancellation.
    executor
        .run::<SystemAppsCgroups>(SystemAppsCgroupsInput {
            action: Action::Remove.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    // Assert
    let remaining = manager
        .list_units_by_patterns(vec![], vec![parent, slice.name()])
        .await
        .unwrap();
    assert!(remaining.is_empty(), "{remaining:?}");
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.state == State::Cancelled)
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, systemd and cgroup v2"]
async fn failed_identity_save_is_observed_and_retains_cleanup_ownership() {
    // Arrange
    let directory = std::env::temp_dir().join(format!(
        "system-app-cgroup-save-failure-{}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    fs::create_dir(directory.join(IDENTITY_FILE)).unwrap();
    let slice = SystemAppSlice::connect(&directory).await.unwrap();
    let _cleanup = Cleanup {
        directory: directory.clone(),
        unit: "cgroup-fixture-absent.service".into(),
        slice: slice.name(),
    };
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut executor =
        LocalExecutor::new(move |event: &Event| observed.borrow_mut().push(event.clone()));
    cgroup::register(&mut executor, slice.clone());

    // Act
    let result = executor
        .run::<SystemAppsCgroups>(SystemAppsCgroupsInput {
            action: Action::Create.into(),
            artifacts: directory.to_string_lossy().into_owned(),
            readiness_seconds: STARTUP.as_secs(),
        })
        .await;
    // Assert: a failed artifact write preserves the original launcher's live identity.
    assert!(result.is_err());
    let identity = slice.identity().unwrap();
    assert!(Path::new(&identity.path).exists());
    assert_eq!(events.borrow().last().unwrap().state, State::Failed);

    // Act
    let removed = executor
        .run::<SystemAppsCgroups>(SystemAppsCgroupsInput {
            action: Action::Remove.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    // Assert
    assert!(removed.value().is_none());
    assert!(!Path::new(&identity.path).exists());
    assert_eq!(events.borrow().last().unwrap().state, State::Succeeded);
}
