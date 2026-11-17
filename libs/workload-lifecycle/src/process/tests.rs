use super::*;
use std::{
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncBufReadExt, BufReader};

const STOP: Duration = Duration::from_secs(10);
const LITERAL: &str = "literal-$UNSET ${HOME} $$ with spaces";

struct Fixture {
    root: PathBuf,
    repository: PathBuf,
    helper: HelperProcess,
}
impl Fixture {
    fn new(systemd: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "holoscan-process-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let repository = root.join("repository");
        let artifacts = root.join("attempt");
        std::fs::create_dir_all(&repository).unwrap();
        std::fs::create_dir(&artifacts).unwrap();
        let repo_hash = host_filesystem::files::digest(repository.as_os_str().as_encoded_bytes());
        let run_hash = host_filesystem::files::digest(artifacts.as_os_str().as_encoded_bytes());
        let parent = if systemd {
            format!(
                "doramcap-compose-{}-{}-system.slice",
                &repo_hash[..12],
                &run_hash[..12]
            )
        } else {
            format!(
                "/doramcap-compose-{}/{}/system",
                &repo_hash[..12],
                &run_hash[..12]
            )
        };
        let placement = CgroupPlacement {
            path: parent_path(&parent).unwrap().to_string_lossy().into_owned(),
            parent,
        };
        let application_parent = if systemd {
            placement
                .parent
                .replace("-system.slice", "-user-build.slice")
        } else {
            placement.parent.replace("/system", "/user/build")
        };
        let helper =
            HelperProcess::prepare_at(placement, &artifacts, &repository, application_parent, STOP)
                .unwrap()
                .with_admission(crate::admission::Admission::acquire(&repository).unwrap())
                .unwrap();
        Self {
            root,
            repository,
            helper,
        }
    }
    async fn finish(self) {
        self.helper.remove().await.unwrap();
        self.helper.remove().await.unwrap();
        assert!(!Path::new(&self.helper.ownership.helper_path).exists());
        std::fs::remove_dir_all(&self.root).unwrap();
        std::fs::remove_dir(&self.helper.directory).unwrap();
    }
}

#[test]
fn ownership_precedes_launch_and_survives_source_removal() {
    // Arrange: systemd preparation records identity without creating units.
    let fixture = Fixture::new(true);
    let record = fixture.helper.directory.join(OWNERSHIP_FILE);
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&record).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!Path::new(&fixture.helper.ownership.helper_path).exists());
    // Act
    std::fs::remove_dir(&fixture.repository).unwrap();
    std::fs::remove_dir(fixture.helper.artifacts()).unwrap();
    let recovered = HelperProcess::recover_repository(&fixture.repository, STOP)
        .unwrap()
        .unwrap();
    // Assert
    assert_eq!(recovered.repository_root(), fixture.repository);
    assert_eq!(recovered.cgroup_parent(), fixture.helper.cgroup_parent());
    assert!(
        recovered
            .command(Path::new("relative-cli"))
            .unwrap_err()
            .contains("admission guard")
    );
    std::fs::remove_file(fixture.helper.directory.join(OWNERSHIP_FILE)).ok();
    std::fs::remove_dir(&fixture.helper.directory).unwrap();
    std::fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn missing_and_malformed_records_are_distinct_and_foreign_placement_is_rejected() {
    // Arrange
    let fixture = Fixture::new(true);
    let record = fixture.helper.directory.join(OWNERSHIP_FILE);
    let mut ownership = fixture.helper.ownership.clone();
    ownership.placement = Some(CgroupPlacement {
        parent: "foreign.slice".into(),
        path: cgroup::slice::path("foreign.slice")
            .to_string_lossy()
            .into_owned(),
    });
    // Act / Assert
    std::fs::write(&record, ownership.encode_to_vec()).unwrap();
    assert!(HelperProcess::recover_repository(&fixture.repository, STOP).is_err());
    std::fs::write(&record, b"malformed record").unwrap();
    assert!(HelperProcess::recover_repository(&fixture.repository, STOP).is_err());
    std::fs::remove_file(&record).unwrap();
    assert!(
        HelperProcess::recover_repository(&fixture.repository, STOP)
            .unwrap()
            .is_none()
    );
    std::fs::remove_file(fixture.helper.directory.join(OWNERSHIP_FILE)).ok();
    std::fs::remove_dir(&fixture.helper.directory).unwrap();
    std::fs::remove_dir_all(fixture.root).unwrap();
}

async fn literal_and_descendant_contract(systemd: bool) {
    // Arrange
    let fixture = Fixture::new(systemd);
    let mut command = fixture.helper.command(Path::new("/bin/sh")).unwrap();
    command.args(["-c", "printf '%s\\n' \"$$\" \"$PWD\" \"$HOLOSCAN_PROCESS_TEST_LITERAL\" \"$1\"; cat /proc/self/cgroup; exit 17", "fixture", LITERAL])
        .current_dir(&fixture.repository).env("HOLOSCAN_PROCESS_TEST_LITERAL", LITERAL)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // Act
    let child = command.spawn().unwrap();
    let pid = child.id().unwrap();
    let output = tokio::time::timeout(STOP, child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    // Assert: systemd-run execs the CLI in the actual retained child, without interpolating argv.
    assert_eq!(
        output.status.code(),
        Some(17),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines[0], pid.to_string());
    assert_eq!(lines[1], fixture.repository.to_str().unwrap());
    assert_eq!(lines[2], LITERAL);
    assert_eq!(lines[3], LITERAL);
    assert!(lines[4].contains(&fixture.helper.ownership.helper_name));
    fixture.finish().await;

    // Arrange: a new session survives the direct CLI's exit.
    let fixture = Fixture::new(systemd);
    let mut command = fixture.helper.command(Path::new("/bin/sh")).unwrap();
    command.args(["-c", "setsid /bin/sleep infinity </dev/null >/dev/null 2>&1 & printf '%s\\n' \"$!\"; exit 23"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // Act
    let output = tokio::time::timeout(STOP, command.spawn().unwrap().wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(23),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let descendant: u32 = String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // Assert
    assert!(
        std::fs::read_to_string(format!("/proc/{descendant}/cgroup"))
            .unwrap()
            .contains(&fixture.helper.ownership.helper_name)
    );
    // Act / Assert: only the owned helper is stopped, and repetition accepts absence/emptiness.
    let recovered = HelperProcess::recover_repository(&fixture.repository, STOP)
        .unwrap()
        .unwrap();
    recovered.quiesce().await.unwrap();
    recovered.quiesce().await.unwrap();
    fixture.finish().await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_preserves_invocation_reaps_child_and_recovers_descendants() {
    literal_and_descendant_contract(true).await;
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn real_cgroupfs_preserves_invocation_reaps_child_and_recovers_descendants() {
    literal_and_descendant_contract(false).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_preserves_scope_when_any_ownership_fact_differs() {
    // Arrange
    let fixture = Fixture::new(true);
    let mut command = fixture.helper.command(Path::new("/bin/sh")).unwrap();
    command
        .args(["-c", "printf 'ready\\n'; exec /bin/sleep infinity"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut line = String::new();
    tokio::time::timeout(
        STOP,
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut line),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(line, "ready\n");
    let manager = cgroup::slice::connect().await.unwrap();
    let owner = &fixture.helper.ownership;
    // Act / Assert
    for (slice, path, description) in [
        (
            "foreign.slice",
            Path::new(&owner.helper_path),
            owner.description.as_str(),
        ),
        (
            owner.application_parent.as_str(),
            Path::new("/sys/fs/cgroup/foreign"),
            owner.description.as_str(),
        ),
        (
            owner.application_parent.as_str(),
            Path::new(&owner.helper_path),
            "foreign description",
        ),
    ] {
        assert!(
            cgroup::scope::stop(&manager, &owner.helper_name, slice, path, description)
                .await
                .is_err()
        );
        assert!(child.try_wait().unwrap().is_none());
    }
    // Act / Assert
    fixture.helper.quiesce().await.unwrap();
    assert!(!child.wait().await.unwrap().success());
    fixture.finish().await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn real_cgroupfs_preserves_live_process_when_identity_is_uncertain_or_changed() {
    // Arrange
    let fixture = Fixture::new(false);
    let mut command = fixture.helper.command(Path::new("/bin/sleep")).unwrap();
    let mut child = command.arg("infinity").spawn().unwrap();
    // Act / Assert
    for mismatch in 0..3 {
        let mut recovered = HelperProcess::recover_repository(&fixture.repository, STOP)
            .unwrap()
            .unwrap();
        match mismatch {
            0 => recovered.ownership.cgroup_inode = 0,
            1 => recovered.ownership.cgroup_inode += 1,
            _ => recovered.ownership.boot_id = "different-boot".into(),
        }
        std::fs::write(
            recovered.directory.join(OWNERSHIP_FILE),
            recovered.ownership.encode_to_vec(),
        )
        .unwrap();
        assert!(recovered.quiesce().await.is_err());
        assert!(child.try_wait().unwrap().is_none());
    }
    // Act / Assert
    std::fs::write(
        fixture.helper.directory.join(OWNERSHIP_FILE),
        fixture.helper.ownership.encode_to_vec(),
    )
    .unwrap();
    fixture.helper.quiesce().await.unwrap();
    assert!(!child.wait().await.unwrap().success());
    fixture.finish().await;
}

// The test executable acts as a controller process so the owner can die independently of this test runtime.
#[tokio::test(flavor = "current_thread")]
#[ignore = "launched only by the process parent-loss contract"]
async fn controller_fixture() {
    let artifacts = PathBuf::from(
        std::env::var_os("HOLOSCAN_PROCESS_TEST_ARTIFACTS").expect("fixture artifacts"),
    );
    let helper = HelperProcess::recover_repository(&artifacts, STOP)
        .unwrap()
        .unwrap();
    let admission = crate::admission::Admission::acquire(helper.repository_root()).unwrap();
    let helper = helper.with_admission(admission).unwrap();
    let mut command = helper.command(Path::new("/bin/sh")).unwrap();
    command.args(["-c", "setsid /bin/sleep infinity </dev/null >/dev/null 2>&1 & printf '%s %s\\n' \"$$\" \"$!\" > \"$1\"; exec /bin/sleep infinity", "fixture", helper.artifacts().join("ready").to_str().unwrap()])
        .stdin(Stdio::null()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let mut child = command.spawn().unwrap();
    child.wait().await.unwrap();
}

async fn parent_loss_contract(systemd: bool) {
    // Arrange: only the independently killed controller owns admission during its attempt.
    let mut fixture = Fixture::new(systemd);
    fixture.helper.admission.take();
    let marker = fixture.helper.artifacts().join("ready");
    let (_watcher, mut changes) =
        crate::files::watch_changes([fixture.helper.artifacts()]).unwrap();
    let mut controller = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "process::tests::controller_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("HOLOSCAN_PROCESS_TEST_ARTIFACTS", &fixture.repository)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(STOP, async {
        while !marker.exists() {
            crate::files::wait_for_change(&mut changes).await.unwrap();
        }
    })
    .await
    .unwrap();
    let pids = std::fs::read_to_string(&marker).unwrap();
    let pids: Vec<u32> = pids
        .split_whitespace()
        .map(|pid| pid.parse().unwrap())
        .collect();
    assert_eq!(pids.len(), 2);
    // Act: SIGKILL controller; inherited output EOF also establishes that its guarded CLI closed descriptors.
    controller.start_kill().unwrap();
    let output = tokio::time::timeout(STOP, controller.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(!output.status.success());
    // Assert: the guarded CLI is dead while a descendant still belongs to the recorded helper.
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pids[0])) {
        assert!(matches!(
            stat.rsplit_once(')').unwrap().1.trim_start().as_bytes()[0],
            b'Z' | b'X'
        ));
    }
    assert!(
        std::fs::read_to_string(format!("/proc/{}/cgroup", pids[1]))
            .unwrap()
            .contains(&fixture.helper.ownership.helper_name)
    );
    // Admission must be released while the registered descendant is still alive.
    let _admission = crate::admission::Admission::acquire(&fixture.repository).unwrap();
    // Act / Assert
    HelperProcess::recover_repository(&fixture.repository, STOP)
        .unwrap()
        .unwrap()
        .quiesce()
        .await
        .unwrap();
    fixture.finish().await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_parent_loss_guards_cli_and_recovers_surviving_descendants() {
    parent_loss_contract(true).await;
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn real_cgroupfs_parent_loss_guards_cli_and_recovers_surviving_descendants() {
    parent_loss_contract(false).await;
}

async fn thread_loss_contract(systemd: bool) {
    // Arrange: the process survives while exactly the spawning runtime thread exits.
    let fixture = Fixture::new(systemd);
    let artifacts = fixture.repository.clone();
    let admission = fixture.helper.admission.clone().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let owner = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let helper = HelperProcess::recover_repository(&artifacts, STOP)
                    .unwrap()
                    .unwrap();
                let helper = helper.with_admission(admission).unwrap();
                let mut command = helper.command(Path::new("/bin/sh")).unwrap();
                command
                    .args(["-c", "printf 'ready\\n'; exec /bin/sleep infinity"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                let mut child = command.spawn().unwrap();
                let mut line = String::new();
                BufReader::new(child.stdout.take().unwrap())
                    .read_line(&mut line)
                    .await
                    .unwrap();
                assert_eq!(line, "ready\n");
                sender.send(child).unwrap();
            });
    });
    let mut child = receiver.recv().unwrap();
    // Act
    owner.join().unwrap();
    let status = tokio::time::timeout(STOP, child.wait())
        .await
        .unwrap()
        .unwrap();
    // Assert: no explicit signal from the test owner was needed.
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(9));
    fixture.finish().await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_spawning_thread_loss_kills_the_actual_cli() {
    thread_loss_contract(true).await;
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn real_cgroupfs_spawning_thread_loss_kills_the_actual_cli() {
    thread_loss_contract(false).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_registration_cancellation_has_no_late_scope_in_observation_window() {
    // Arrange: race cancellation across bootstrap and D-Bus registration phases.
    const OBSERVATION_WINDOW: Duration = Duration::from_millis(100);
    for round in 0..4 {
        for microseconds in [0, 100, 500, 1000, 2500, 5000, 10000] {
            let fixture = Fixture::new(true);
            let mut command = fixture.helper.command(Path::new("/bin/sleep")).unwrap();
            command
                .arg("infinity")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let mut child = command.spawn().unwrap();
            // Act: a deliberate race delay exercises timing; production has no polling/waits here.
            if microseconds != 0 {
                tokio::time::sleep(Duration::from_micros(microseconds)).await;
            }
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            fixture
                .helper
                .remove()
                .await
                .unwrap_or_else(|e| panic!("round{round} delay{microseconds}: {e}"));
            tokio::time::sleep(OBSERVATION_WINDOW).await;
            // Assert: this is a bounded qualification window, not exhaustive proof of every pending RPC.
            assert!(
                !Path::new(&fixture.helper.ownership.helper_path).exists(),
                "round{round} delay{microseconds}"
            );
            assert!(
                !parent_path(fixture.helper.cgroup_parent())
                    .unwrap()
                    .exists(),
                "late app parent at round{round} delay{microseconds}"
            );
            fixture.finish().await;
        }
    }
}

#[test]
fn stale_handles_preserve_a_newer_current_record_and_existing_records_block_prepare() {
    // Arrange
    let fixture = Fixture::new(true);
    let original = fixture.helper.ownership.encode_to_vec();
    let mut newer = fixture.helper.ownership.clone();
    newer.artifacts_root.push_str("/new-attempt");
    let record = fixture.helper.directory.join(OWNERSHIP_FILE);
    std::fs::write(&record, newer.encode_to_vec()).unwrap();
    // Act / Assert
    assert!(fixture.helper.clear().is_err());
    assert_eq!(std::fs::read(&record).unwrap(), newer.encode_to_vec());
    assert!(
        HelperProcess::prepare_at(
            fixture.helper.ownership.placement.clone().unwrap(),
            fixture.helper.artifacts(),
            &fixture.repository,
            fixture.helper.cgroup_parent().into(),
            STOP
        )
        .is_err()
    );
    // Act / Assert
    std::fs::write(&record, original).unwrap();
    fixture.helper.clear().unwrap();
    fixture.helper.clear().unwrap();
    assert!(
        HelperProcess::recover_repository(&fixture.repository, STOP)
            .unwrap()
            .is_none()
    );
    std::fs::remove_dir(&fixture.helper.directory).unwrap();
    std::fs::remove_dir_all(fixture.root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "manual recovery of one preserved capability-test attempt"]
async fn preserved_registration_fixture() {
    let artifacts = PathBuf::from(
        std::env::var_os("HOLOSCAN_PROCESS_TEST_OLD_ATTEMPT").expect("preserved fixture artifacts"),
    );
    let mut ownership = HelperOwnership::decode(
        std::fs::read(artifacts.join(OWNERSHIP_FILE))
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    ownership.artifacts_root = artifacts.to_str().unwrap().to_owned();
    let directory = repository_scope(Path::new(&ownership.repository_root));
    host_filesystem::files::ensure_private_directory(&directory).unwrap();
    let helper = HelperProcess {
        ownership,
        directory,
        stop: STOP,
        admission: None,
    };
    helper.validate().unwrap();
    helper.save(true).unwrap();
    helper.remove().await.unwrap();
    helper.remove().await.unwrap();
    std::fs::remove_dir(helper.directory).unwrap();
}

#[test]
fn a_record_clear_retries_directory_sync_after_an_acknowledgement_failure() {
    // Arrange
    let fixture = Fixture::new(true);
    let mut attempts = 0;
    // Act: removal succeeds, but its acknowledgement fails.
    assert!(
        fixture
            .helper
            .clear_with_sync(|_| {
                attempts += 1;
                Err("injected directory sync failure".into())
            })
            .is_err()
    );
    // Assert
    assert!(!fixture.helper.directory.join(OWNERSHIP_FILE).exists());
    // Act: retry observes absence and must still perform the durable acknowledgement.
    fixture
        .helper
        .clear_with_sync(|path| {
            attempts += 1;
            sync_directory(path)
        })
        .unwrap();
    // Assert
    assert_eq!(attempts, 2);
    std::fs::remove_dir(&fixture.helper.directory).unwrap();
    std::fs::remove_dir_all(fixture.root).unwrap();
}

#[tokio::test]
async fn helper_population_transition_is_observed_after_change_notification() {
    // Arrange: subscribe before observing a populated helper.
    let root = std::env::temp_dir().join(format!(
        "holoscan-population-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let events = root.join("cgroup.events");
    std::fs::write(&events, b"populated 1\nfrozen 0\n").unwrap();
    let (_watcher, mut changes) = crate::files::watch_changes([events.as_path()]).unwrap();
    assert_eq!(helper_populated(&root), Ok(true));
    // Act: the empty transition can precede the wait; its notification must remain observable.
    std::fs::write(&events, b"populated 0\nfrozen 0\n").unwrap();
    tokio::time::timeout(STOP, crate::files::wait_for_change(&mut changes))
        .await
        .unwrap()
        .unwrap();
    // Assert: one new observation accepts the empty state without another notification/read.
    assert_eq!(helper_populated(&root), Ok(false));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn helper_population_unknown_and_read_failure_do_not_establish_emptiness() {
    // Arrange
    let root = std::env::temp_dir().join(format!(
        "holoscan-population-errors-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let events = root.join("cgroup.events");
    // Act / Assert
    assert!(helper_populated(&root).is_err());
    std::fs::write(&events, b"frozen 0\n").unwrap();
    assert!(helper_populated(&root).unwrap_err().contains("unknown"));
    std::fs::write(&events, b"populated 1\n").unwrap();
    assert_eq!(helper_populated(&root), Ok(true));
    std::fs::remove_dir_all(root).unwrap();
}

// Test fixtures remove their generated empty parent trees; production backends own this policy.
impl HelperProcess {
    async fn remove(&self) -> Result<()> {
        self.remove_leaf().await?;
        let run = Path::new(&self.placement().path).parent().unwrap();
        let repository = run.parent().unwrap();
        if self.systemd() {
            let manager = cgroup::slice::connect().await?;
            for path in [run, repository] {
                cgroup::slice::stop(&manager, path.file_name().unwrap().to_str().unwrap()).await?;
            }
        } else {
            fn remove(path: &Path) -> Result<()> {
                if !path.exists() {
                    return Ok(());
                }
                cgroup::verify_empty(path)?;
                for entry in std::fs::read_dir(path).map_err(error)? {
                    let entry = entry.map_err(error)?;
                    if entry.file_type().map_err(error)?.is_dir() {
                        remove(&entry.path())?;
                    }
                }
                std::fs::remove_dir(path).map_err(error)
            }
            remove(repository)?;
        }
        self.clear()
    }
}

async fn live_scope_refuses_reuse_until_child_reap_and_quiescence(systemd: bool) {
    // Arrange: a real owned child is independently observed inside the helper.
    let fixture = Fixture::new(systemd);
    let mut command = fixture.helper.command(Path::new("/bin/sh")).unwrap();
    command
        .args(["-c", "printf 'ready\n'; exec /bin/sleep infinity"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut line = String::new();
    tokio::time::timeout(
        STOP,
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut line),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(line, "ready\n");
    let record = std::fs::read(fixture.helper.directory.join(OWNERSHIP_FILE)).unwrap();
    // Act / Assert: a second spawn is refused while the actual first helper still has writers.
    assert!(fixture.helper.command(Path::new("/bin/true")).is_err());
    assert!(child.try_wait().unwrap().is_none());
    assert_eq!(
        std::fs::read(fixture.helper.directory.join(OWNERSHIP_FILE)).unwrap(),
        record
    );
    // Act: reap the bootstrap, then explicitly stop and verify every descendant before reuse.
    child.start_kill().unwrap();
    child.wait().await.unwrap();
    fixture.helper.quiesce().await.unwrap();
    let mut command = fixture.helper.command(Path::new("/bin/true")).unwrap();
    assert!(command.spawn().unwrap().wait().await.unwrap().success());
    fixture.helper.quiesce().await.unwrap();
    // Assert: ordinary idempotent removal clears the one current checkpoint.
    fixture.finish().await;
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local systemd and cgroup v2"]
async fn real_systemd_refuses_live_scope_reuse_then_allows_settled_retry() {
    live_scope_refuses_reuse_until_child_reap_and_quiescence(true).await;
}
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root and writable cgroup v2"]
async fn real_cgroupfs_refuses_live_scope_reuse_then_allows_settled_retry() {
    live_scope_refuses_reuse_until_child_reap_and_quiescence(false).await;
}
