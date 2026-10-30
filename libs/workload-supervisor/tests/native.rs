//! Run built executables with capabilities; builds remain unprivileged.
use prost::Message;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use workload_supervisor::{MonitorRun, Ownership, SupervisorRunner};

struct Sandbox {
    dir: PathBuf,
    parent: PathBuf,
}
impl Sandbox {
    fn new() -> Self {
        let name = format!(
            "supervisor-tests-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = Path::new("/tmp").join(&name);
        let parent = Path::new("/sys/fs/cgroup").join(&name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::create_dir(&parent).expect("requires delegated cgroup v2 access");
        Self { dir, parent }
    }
    fn spec(&self, id: &str, binary: &str) -> MonitorRun {
        MonitorRun {
            id: id.into(),
            output_directory: self
                .dir
                .join(format!("{id} outputs"))
                .to_str()
                .unwrap()
                .into(),
            supervisord: std::env::var("DORAMCAP_SUPERVISORD")
                .unwrap_or("/usr/bin/supervisord".into()),
            supervisorctl: std::env::var("DORAMCAP_SUPERVISORCTL")
                .unwrap_or("/usr/bin/supervisorctl".into()),
            monitor_binary: binary.into(),
            readiness_seconds: 10,
            stop_seconds: 1,
            cgroup_parent: self.parent.to_str().unwrap().into(),
        }
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        // Preserve all artifacts for diagnosis. Empty parent removal proves no owned cgroup remains.
        std::fs::remove_dir(&self.parent).expect("test leaked a cgroup");
        eprintln!("native artifacts: {}", self.dir.display());
    }
}
fn observer(e: &orchestrator::Event) {
    eprintln!("{} {:?}", e.invocation.step, e.state);
}
fn ledger(spec: &MonitorRun) -> Ownership {
    Ownership::decode(
        std::fs::read(Path::new(&spec.output_directory).join("ownership.pb"))
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}
fn assert_reaped(ownership: &Ownership) {
    assert!(
        !Path::new(&format!("/proc/{}", ownership.daemon_pid)).exists(),
        "daemon not reaped"
    );
    assert!(!Path::new(&ownership.cgroup).exists());
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires native BPF, Supervisor, writable cgroup v2 and DORAMCAP_NATIVE_MONITOR"]
async fn real_recordings_spaces_and_concurrent_runs() {
    // Arrange
    let sandbox = Sandbox::new();
    let binary = std::env::var("DORAMCAP_NATIVE_MONITOR").unwrap();
    let binary_with_space = sandbox.dir.join("monitor with spaces");
    std::os::unix::fs::symlink(binary, &binary_with_space).unwrap();
    let left = sandbox.spec("left", binary_with_space.to_str().unwrap());
    let right = sandbox.spec("right", binary_with_space.to_str().unwrap());
    let execute = |spec: MonitorRun| async {
        let mut manager = SupervisorRunner::new();
        let (ready, notification) = tokio::sync::oneshot::channel();
        let stop = async {
            notification.await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        };
        let report = manager
            .run_with_ready(spec.clone(), stop, observer, Some(ready))
            .await
            .unwrap();
        (spec, report, manager.blocked())
    };
    // Act
    let (left, right) = tokio::join!(execute(left), execute(right));
    // Assert
    for (spec, report, blocked) in [left, right] {
        assert!(report.successful(), "{report:?}");
        assert!(!blocked);
        let ownership = ledger(&spec);
        assert!(ownership.cleanup_verified);
        assert_reaped(&ownership);
        let bytes = std::fs::read(Path::new(&spec.output_directory).join("cpu.mcap")).unwrap();
        let messages = mcap::MessageStream::new(&bytes)
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert!(messages.len() >= 2);
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires capabilities, cgroup v2, Supervisor and DORAMCAP_TEST_FIXTURE"]
async fn failures_cancellation_descendants_and_abandoned_admission() {
    // Arrange
    let sandbox = Sandbox::new();
    let fixture = std::env::var("DORAMCAP_TEST_FIXTURE").unwrap();
    let early = sandbox.dir.join("early-monitor");
    // Copy so current_exe retains the injected scenario name.
    std::fs::copy(&fixture, &early).unwrap();
    let mut early_spec = sandbox.spec("early", early.to_str().unwrap());
    early_spec.readiness_seconds = 1;
    let mut manager = SupervisorRunner::new();
    let (ready, mut notification) = tokio::sync::oneshot::channel();
    // Act: early monitor exit fails readiness while preserving independent cleanup.
    let report = manager
        .run_with_ready(
            early_spec.clone(),
            std::future::pending(),
            observer,
            Some(ready),
        )
        .await
        .unwrap();
    // Assert
    assert!(
        report.execution_error.is_some() && !report.recording_finalized && report.cleanup_verified,
        "{report:?}"
    );
    assert_reaped(&ledger(&early_spec));
    assert!(matches!(
        notification.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Closed)
    ));

    // Arrange: a daemon executable that fails before creating a socket.
    let mut failed = sandbox.spec("startup", &fixture);
    failed.supervisord = "/bin/false".into();
    // Act
    let report = manager
        .run(failed.clone(), std::future::pending(), observer)
        .await
        .unwrap();
    // Assert
    assert!(
        report.execution_error.is_some() && report.cleanup_verified,
        "{report:?}"
    );
    assert_reaped(&ledger(&failed));

    // Arrange: cancellation after launch is attempted must account for partial startup.
    let cancelled = sandbox.spec("launchcancel", &fixture);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let signal = std::cell::RefCell::new(Some(stop));
    let (ready, mut notification) = tokio::sync::oneshot::channel();
    // Act
    let report = manager
        .run_with_ready(
            cancelled.clone(),
            async {
                stopped.await.unwrap();
            },
            move |event: &orchestrator::Event| {
                if event.invocation.step.ends_with("::Launch")
                    && matches!(event.state, orchestrator::State::Started)
                    && let Some(stop) = signal.borrow_mut().take()
                {
                    let _ = stop.send(());
                }
            },
            Some(ready),
        )
        .await
        .unwrap();
    // Assert
    assert!(
        report.cleanup_verified && !report.recording_finalized,
        "{report:?}"
    );
    assert!(matches!(
        notification.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Closed)
    ));
    assert_reaped(&ledger(&cancelled));

    // Arrange: a TERM-resistant monitor and descendant writing a truncated MCAP.
    let timeout = sandbox.spec("timeout", &fixture);
    let (ready, notification) = tokio::sync::oneshot::channel();
    // Act
    let report = manager
        .run_with_ready(
            timeout.clone(),
            async {
                notification.await.unwrap();
            },
            observer,
            Some(ready),
        )
        .await
        .unwrap();
    // Assert
    assert!(
        report.writers_stopped && report.cleanup_verified && !report.recording_finalized,
        "{report:?}"
    );
    assert!(!report.diagnostics.is_empty());
    assert_reaped(&ledger(&timeout));

    // Arrange: drop the manager future after readiness, retaining durable intent.
    let abandoned = sandbox.spec("abandoned", &fixture);
    let (ready, notification) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(manager.run_with_ready(
        abandoned.clone(),
        std::future::pending(),
        observer,
        Some(ready),
    ));
    // Act
    tokio::select! { _ = &mut run => panic!("run ended before cancellation"), result = notification => result.unwrap() }
    drop(run);
    // Assert: cancellation kills the owned subtree; it cannot certify asynchronous cleanup.
    assert!(manager.blocked());
    let evidence = ledger(&abandoned);
    assert!(!evidence.cleanup_verified);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let empty = std::fs::read_to_string(Path::new(&evidence.cgroup).join("cgroup.events"))
            .unwrap()
            .contains("populated 0");
        if empty && !Path::new(&format!("/proc/{}", evidence.daemon_pid)).exists() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "cancelled subtree still alive"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Act / Assert: another manager cannot adopt the incomplete output directory.
    assert!(
        SupervisorRunner::new()
            .run(abandoned, std::future::pending(), observer)
            .await
            .is_err()
    );
    // Test-owned recovery after verifying emptiness; application recovery remains deferred.
    std::fs::remove_dir(&evidence.cgroup).unwrap();
}
