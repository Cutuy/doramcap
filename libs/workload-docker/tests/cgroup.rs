//! Native Compose cgroup placement. Build unprivileged; elevate only this test executable.
use compose_apps::compose::{ComposeProject, DockerAdapter, RemoveCompose, StartCompose};
use compose_apps::preparation::PrepareRun;
use orchestrator::{Event, State, Step};
use prost::Message;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    rc::Rc,
    time::Duration,
};
use workload_docker::workflows::recorded_compose::{ComposeRun, ComposeRunner};
use workload_lifecycle::cgroup::{
    self, Action, CGROUP_ROOT, IDENTITY_FILE, SystemAppsCgroups, SystemAppsCgroupsInput,
};
use workload_lifecycle::{
    launch::{HostShmAccess, SystemAppCgroup},
    recording::WaitForRecordings,
};

const RUN_LIMIT: Duration = Duration::from_secs(30);
const STARTUP_SECONDS: u64 = 10;
const STOP_SECONDS: u64 = 3;
const RUN_LABEL: &str = "label=io.doramcap.run";
const SERVICE_LABEL: &str = "label=com.docker.compose.service";
const SHM_SERVICE: &str = "doramcap-shm";

struct Fixture {
    directory: tempfile::TempDir,
    socket: String,
    image: String,
    prefix: String,
    parents: RefCell<Vec<SystemAppCgroup>>,
}
impl Fixture {
    fn new() -> Self {
        Self::on_socket("/var/run/docker.sock")
    }
    fn on_socket(socket: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let prefix = format!("cgroup-native-{}", std::process::id());
        let fixture = Self {
            parents: RefCell::new(Vec::new()),
            directory,
            socket: socket.into(),
            image: format!("doramcap-cgroup-test:{}", std::process::id()),
            prefix,
        };
        let stage = fixture.directory.path().join("image");
        let root = stage.join("rootfs");
        fs::create_dir_all(root.join("bin")).unwrap();
        for executable in ["/bin/sh", "/bin/sleep"] {
            fs::copy(executable, root.join(executable.trim_start_matches('/'))).unwrap();
            let linked = Command::new("ldd").arg(executable).output().unwrap();
            assert!(linked.status.success());
            for library in String::from_utf8(linked.stdout)
                .unwrap()
                .split_whitespace()
                .filter(|part| part.starts_with('/'))
            {
                let destination = root.join(library.trim_start_matches('/'));
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::copy(library, destination).unwrap();
            }
        }
        fs::write(
            stage.join("Dockerfile"),
            "FROM scratch\nCOPY rootfs /\nENTRYPOINT [\"/bin/sh\"]\n",
        )
        .unwrap();
        let build = fixture.docker(&[
            "build",
            "--network=none",
            "--tag",
            &fixture.image,
            stage.to_str().unwrap(),
        ]);
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
        let data = fixture.directory.path().join("user-data");
        fs::create_dir(&data).unwrap();
        let background = "trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done";
        for (file, name, command) in [
            ("system", "system", background.to_owned()),
            (
                "user",
                "user",
                format!("printf started > /data/started; {background}"),
            ),
        ] {
            let mut service = serde_json::json!({"image":fixture.image, "command":["-c",command]});
            if name == "user" {
                service["volumes"] =
                    serde_json::json!([{ "type":"bind", "source":data, "target":"/data" }]);
            }
            let mut services = serde_json::json!({name:service});
            if name == "system" {
                services[SHM_SERVICE] = serde_json::json!({"image":fixture.image, "command":["-c",background], "ipc":"shareable"});
            }
            fs::write(
                fixture.directory.path().join(format!("{file}.json")),
                serde_json::to_vec(&serde_json::json!({"services":services})).unwrap(),
            )
            .unwrap();
        }
        fixture
    }
    fn docker(&self, args: &[&str]) -> Output {
        docker(&self.socket, args)
    }
    fn spec(&self, index: usize) -> ComposeRun {
        let app = |key: &str| ComposeProject {
            key: key.into(),
            directory: self.directory.path().to_string_lossy().into(),
            files: vec![format!("{key}.json")],
            environment: BTreeMap::new(),
            ..Default::default()
        };
        ComposeRun {
            id: format!("{}-{index}", self.prefix),
            repository_root: self.directory.path().into(),
            artifacts: self.directory.path().join(format!("run-{index}")),
            projects: vec![app("user")],
            system_apps: vec![app("system")],
            private_records: Vec::new(),
            host_shm_access: HostShmAccess::Passthrough as i32,
            readiness_seconds: STARTUP_SECONDS,
            stop_seconds: STOP_SECONDS,
            filesystem: None,
            dora_apps: vec![],
            dataflow_file: String::new(),
            recordings: vec![],
            mcap_writers: Default::default(),
        }
    }
    fn identity(&self, index: usize) -> SystemAppCgroup {
        let identity = SystemAppCgroup::decode(
            fs::read(self.spec(index).artifacts.join(IDENTITY_FILE))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        self.parents.borrow_mut().push(identity.clone());
        identity
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for index in 0..2 {
            let filter = format!("{RUN_LABEL}={}-{index}", self.prefix);
            for (list, remove) in [
                (&["ps", "--all", "--quiet"][..], &["rm", "--force"][..]),
                (&["network", "ls", "--quiet"][..], &["network", "rm"][..]),
            ] {
                let mut args = list.to_vec();
                args.extend(["--filter", &filter]);
                let found = self.docker(&args);
                for id in String::from_utf8_lossy(&found.stdout).split_whitespace() {
                    let mut args = remove.to_vec();
                    args.push(id);
                    self.docker(&args);
                }
            }
        }
        for identity in self.parents.borrow().iter() {
            if cgroup::verify_identity(identity).is_err()
                || cgroup::verify_empty(Path::new(&identity.path)).is_err()
            {
                continue;
            }
            let path = Path::new(&identity.path);
            let name = path.file_name().unwrap().to_str().unwrap();
            if name.ends_with(".slice") {
                let parent = format!("{}.slice", name.strip_suffix("-system.slice").unwrap());
                let _ = Command::new("systemctl")
                    .args(["stop", name, &parent])
                    .output();
            } else {
                let _ = fs::remove_dir(path);
            }
        }
        self.docker(&["image", "rm", &self.image]);
    }
}
fn docker(socket: &str, args: &[&str]) -> Output {
    Command::new("docker")
        .args(["--host", &format!("unix://{socket}")])
        .args(args)
        .output()
        .unwrap()
}
fn membership(pid: i64) -> PathBuf {
    let text = fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap();
    Path::new(CGROUP_ROOT).join(
        text.lines()
            .find_map(|line| line.strip_prefix("0::/"))
            .unwrap(),
    )
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local Docker with Compose and cgroup v2"]
async fn completed_runs_remove_their_accounting_parents() {
    // Arrange
    let fixture = Fixture::new();
    let adapter = Rc::new(DockerAdapter::connect(&fixture.socket).await.unwrap());
    for index in 0..2 {
        let mut runner = ComposeRunner::new(adapter.clone());
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut run = Box::pin(runner.run_with_ready(
            fixture.spec(index),
            async {
                let _ = stopped.await;
            },
            |_: &Event| {},
            Some(ready),
        ));
        tokio::time::timeout(RUN_LIMIT, async {
            tokio::select! {
                plan = started => { plan.unwrap(); },
                result = &mut run => panic!("startup failed: {result:?}"),
            }
        })
        .await
        .unwrap();
        let identity = fixture.identity(index);

        // Act
        stop.send(()).unwrap();
        let report = tokio::time::timeout(RUN_LIMIT, run).await.unwrap().unwrap();

        // Assert: completed runs cannot accumulate empty per-run cgroups.
        assert!(report.successful(), "{report:#?}");
        let path = Path::new(&identity.path);
        assert!(
            !path.exists(),
            "accounting cgroup remains: {}",
            path.display()
        );
        if path
            .extension()
            .is_some_and(|extension| extension == "slice")
        {
            let parent = path.parent().unwrap();
            assert!(
                !parent.exists(),
                "per-run slice remains: {}",
                parent.display()
            );
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local Docker with Compose and cgroup v2"]
async fn system_app_placement_survives_controller_crash_and_cleans_up() {
    // Arrange
    placement_and_recovery(Fixture::new()).await;
}

async fn placement_and_recovery(fixture: Fixture) {
    let adapter = Rc::new(DockerAdapter::connect(&fixture.socket).await.unwrap());
    let mut previous = None;
    for index in 0..2 {
        let mut runner = ComposeRunner::new(adapter.clone());
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let mut run = Box::pin(runner.run_with_ready(
            ComposeRun {
                private_records: Vec::new(),
                host_shm_access: if index == 0 {
                    HostShmAccess::Passthrough
                } else {
                    HostShmAccess::Blocked
                } as i32,
                id: fixture.spec(0).id,
                ..fixture.spec(index)
            },
            async {
                let _ = stopped.await;
            },
            move |event: &Event| observed.borrow_mut().push(event.clone()),
            Some(ready),
        ));
        // Act
        let plan=tokio::time::timeout(RUN_LIMIT, async { tokio::select! { plan=started=>plan.unwrap(), result=&mut run=>panic!("startup failed: {result:?}") } }).await.unwrap();
        let state = adapter.project(&plan).inspect().await.unwrap();
        let identity = fixture.identity(index);
        // Assert: independent kernel membership agrees with the recorded boundary.
        assert!(state.ready().unwrap());
        let memory: u64 = fs::read_to_string(Path::new(&identity.path).join("memory.current"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let cpu: u64 = fs::read_to_string(Path::new(&identity.path).join("cpu.stat"))
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("usage_usec "))
            .unwrap()
            .parse()
            .unwrap();
        assert!(memory > 0 && cpu > 0);
        assert!(
            fs::read_to_string(Path::new(&identity.path).join("cgroup.procs"))
                .unwrap()
                .trim()
                .is_empty()
        );
        let system = state
            .containers
            .iter()
            .find(|container| container.service == "system")
            .unwrap();
        let user = state
            .containers
            .iter()
            .find(|container| container.service == "user")
            .unwrap();
        assert!(membership(system.pid).starts_with(&identity.path));
        assert!(!membership(user.pid).starts_with(&identity.path));
        // User containers get their own group in the same run group.
        assert!(membership(user.pid).starts_with(Path::new(&identity.path).parent().unwrap()));
        if index == 1 {
            let shm = state
                .containers
                .iter()
                .find(|container| container.service == SHM_SERVICE)
                .unwrap();
            assert!(membership(shm.pid).starts_with(&identity.path));
        }
        {
            let events = events.borrow();
            let prepared = events
                .iter()
                .find(|event| {
                    event.invocation.step
                        == PrepareRun::ID.unwrap_or(std::any::type_name::<PrepareRun>())
                        && event.state == State::Succeeded
                })
                .unwrap();
            let created = events
                .iter()
                .find(|event| {
                    event.invocation.step == SystemAppsCgroups::ID.unwrap()
                        && event.state == State::Started
                        && serde_json::from_value::<SystemAppsCgroupsInput>(
                            event.payload.clone().unwrap(),
                        )
                        .unwrap()
                        .action
                            == i32::from(Action::Create)
                })
                .unwrap();
            assert!(
                created
                    .invocation
                    .dependencies
                    .contains(&prepared.invocation.id)
            );
            let created_index = events
                .iter()
                .position(|event| {
                    event.invocation.id == created.invocation.id && event.state == State::Succeeded
                })
                .unwrap();
            let launched_index = events
                .iter()
                .position(|event| {
                    event.invocation.step
                        == StartCompose::ID.unwrap_or(std::any::type_name::<StartCompose>())
                        && event.state == State::Started
                })
                .unwrap();
            let user_index = events
                .iter()
                .position(|event| {
                    event.invocation.step
                        == StartCompose::ID.unwrap_or(std::any::type_name::<StartCompose>())
                        && event.state == State::Started
                        && event.payload.as_ref().is_some_and(|value| {
                            value["services"].as_array().is_some_and(|services| {
                                services.iter().any(|service| service == "user")
                            })
                        })
                })
                .unwrap();
            assert!(launched_index < created_index && created_index < user_index);
            let output: Option<SystemAppCgroup> =
                serde_json::from_value(events[created_index].payload.clone().unwrap()).unwrap();
            assert_eq!(output, Some(identity.clone()));
        }
        if let Some(previous) = previous {
            let previous: SystemAppCgroup = previous;
            assert_ne!(previous.id, identity.id);
            assert_ne!(previous.path, identity.path);
        }
        previous = Some(identity.clone());
        if index == 0 {
            // Act / Assert: dropping the controller leaves live resources for the next run's reset.
            drop(run);
            assert!(Path::new(&identity.path).exists());
            let restarted = fixture.docker(&["restart", "--time", "1", &system.id]);
            assert!(
                restarted.status.success(),
                "{}",
                String::from_utf8_lossy(&restarted.stderr)
            );
            let inspected = fixture.docker(&["inspect", "--format", "{{.State.Pid}}", &system.id]);
            let pid = String::from_utf8(inspected.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            assert!(membership(pid).starts_with(&identity.path));
        } else {
            // Act
            stop.send(()).unwrap();
            let report = tokio::time::timeout(RUN_LIMIT, run).await.unwrap().unwrap();
            // Assert
            assert!(report.boundary_clean, "{report:#?}");
            assert!(report.execution_error.is_none(), "{report:#?}");
            assert!(
                !Path::new(&identity.path).exists(),
                "per-run cgroup remains"
            );
            assert!(fixture.spec(index).artifacts.join(IDENTITY_FILE).is_file());
            let events = events.borrow();
            let actions: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.invocation.step == SystemAppsCgroups::ID.unwrap()
                        && event.state == State::Started
                })
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
            let removed = events
                .iter()
                .rev()
                .find(|event| {
                    event.invocation.step
                        == RemoveCompose::ID.unwrap_or(std::any::type_name::<RemoveCompose>())
                        && event.state == State::Succeeded
                })
                .unwrap();
            let cgroup_removal = events
                .iter()
                .rev()
                .find(|event| {
                    event.invocation.step == SystemAppsCgroups::ID.unwrap()
                        && event.state == State::Started
                })
                .unwrap();
            assert!(
                cgroup_removal
                    .invocation
                    .dependencies
                    .contains(&removed.invocation.id)
            );
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local Docker with Compose and cgroup v2"]
async fn misplaced_system_app_prevents_user_start() {
    misplaced_on_socket("/var/run/docker.sock").await;
}

async fn misplaced_on_socket(socket: &str) {
    for service in ["system", SHM_SERVICE] {
        // Arrange
        let fixture = Fixture::on_socket(socket);
        let adapter = Rc::new(DockerAdapter::connect(&fixture.socket).await.unwrap());
        let mut runner = ComposeRunner::new(adapter);
        let socket = fixture.socket.clone();
        let filter = format!("{RUN_LABEL}={}-0", fixture.prefix);
        let destination = membership(i64::from(std::process::id())).join("cgroup.procs");
        let moved = Rc::new(Cell::new(false));
        let observed = moved.clone();
        // Act: simulate an external placement change after system readiness and before the user gate.
        let report = tokio::time::timeout(
            RUN_LIMIT,
            runner.run(
                ComposeRun {
                    private_records: Vec::new(),
                    host_shm_access: if service == SHM_SERVICE {
                        HostShmAccess::Blocked
                    } else {
                        HostShmAccess::Passthrough
                    } as i32,
                    ..fixture.spec(0)
                },
                std::future::pending(),
                move |event: &Event| {
                    if event.invocation.step
                        == WaitForRecordings::ID
                            .unwrap_or(std::any::type_name::<WaitForRecordings>())
                        && event.state == State::Succeeded
                        && !observed.get()
                    {
                        let service = format!("{SERVICE_LABEL}={service}");
                        let ids = docker(
                            &socket,
                            &["ps", "--quiet", "--filter", &filter, "--filter", &service],
                        );
                        assert!(
                            ids.status.success(),
                            "{}",
                            String::from_utf8_lossy(&ids.stderr)
                        );
                        let id = String::from_utf8(ids.stdout).unwrap();
                        assert!(!id.trim().is_empty());
                        let inspected = docker(
                            &socket,
                            &["inspect", "--format", "{{.State.Pid}}", id.trim()],
                        );
                        fs::write(
                            &destination,
                            String::from_utf8(inspected.stdout).unwrap().trim(),
                        )
                        .unwrap();
                        observed.set(true);
                    }
                },
            ),
        )
        .await
        .unwrap()
        .unwrap();
        // Assert
        assert!(moved.get());
        assert!(
            report
                .execution_error
                .as_deref()
                .is_some_and(|error| error.contains("expected") || error.contains("cgroup")),
            "{report:#?}"
        );
        assert!(!fixture.directory.path().join("user-data/started").exists());
        assert!(report.boundary_clean, "{report:#?}");
        fixture.identity(0);
    }
}

const DAEMON_START_LIMIT: Duration = Duration::from_secs(60);
const DAEMON_STOP_LIMIT: Duration = Duration::from_secs(10);
const DAEMON_CHECK_INTERVAL: Duration = Duration::from_millis(25);

struct CgroupfsDaemon {
    directory: tempfile::TempDir,
    child: Child,
    socket: String,
}
impl CgroupfsDaemon {
    async fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory
            .path()
            .join("engine.sock")
            .to_string_lossy()
            .into_owned();
        let log = fs::File::create(directory.path().join("daemon.log")).unwrap();
        let child = Command::new("dockerd")
            .process_group(0)
            .args([
                "--host",
                &format!("unix://{socket}"),
                "--data-root",
                directory.path().join("data").to_str().unwrap(),
                "--exec-root",
                directory.path().join("exec").to_str().unwrap(),
                "--pidfile",
                directory.path().join("daemon.pid").to_str().unwrap(),
                "--bridge=none",
                "--iptables=false",
                "--ip6tables=false",
                "--ip-forward=false",
                "--ip-masq=false",
                "--storage-driver=vfs",
                "--exec-opt=native.cgroupdriver=cgroupfs",
            ])
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        let mut daemon = Self {
            directory,
            child,
            socket,
        };
        tokio::time::timeout(DAEMON_START_LIMIT, async {
            loop {
                assert!(
                    daemon.child.try_wait().unwrap().is_none(),
                    "{}",
                    fs::read_to_string(daemon.directory.path().join("daemon.log")).unwrap()
                );
                if Path::new(&daemon.socket).exists() {
                    let info = docker(&daemon.socket, &["info", "--format", "{{.CgroupDriver}}"]);
                    if info.status.success() {
                        assert_eq!(String::from_utf8(info.stdout).unwrap().trim(), "cgroupfs");
                        break;
                    }
                }
                tokio::time::sleep(DAEMON_CHECK_INTERVAL).await;
            }
        })
        .await
        .unwrap();
        daemon
    }
}
impl Drop for CgroupfsDaemon {
    fn drop(&mut self) {
        // This process group contains only the fixture daemon and its managed containerd.
        unsafe {
            libc::killpg(self.child.id() as i32, libc::SIGTERM);
        }
        let deadline = std::time::Instant::now() + DAEMON_STOP_LIMIT;
        while self.child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(DAEMON_CHECK_INTERVAL);
        }
        if self.child.try_wait().unwrap().is_none() {
            unsafe {
                libc::killpg(self.child.id() as i32, libc::SIGKILL);
            }
            self.child.wait().unwrap();
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, dockerd with cgroupfs, Compose and cgroup v2"]
async fn cgroupfs_placement_and_recovery_use_the_shared_step() {
    // Arrange
    let daemon = CgroupfsDaemon::start().await;
    // Act / Assert: execute the same membership, restart, recovery and empty-parent checks.
    placement_and_recovery(Fixture::on_socket(&daemon.socket)).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, dockerd with cgroupfs, Compose and cgroup v2"]
async fn cgroupfs_misplacement_prevents_user_start() {
    // Arrange
    let daemon = CgroupfsDaemon::start().await;
    // Act / Assert
    misplaced_on_socket(&daemon.socket).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires root, local Docker with Compose and cgroup v2"]
async fn shared_system_sequences_preserve_lineage_and_stop_shm_with_system_apps() {
    use compose_apps::{
        compose::{QuiesceCompose, StartInput, StopInput},
        experiment::{RegisterDoraGraph, StartDoraRuntime, StopDoraRuntime},
    };
    // Arrange: real system services run with and without an extra user-service group.
    for index in 0..4 {
        let fixture = Fixture::new();
        let adapter = Rc::new(DockerAdapter::connect(&fixture.socket).await.unwrap());
        let with_user = index % 2 != 0;
        let isolated = index < 2;
        let mut spec = fixture.spec(index % 2);
        spec.host_shm_access = if isolated {
            HostShmAccess::Blocked
        } else {
            HostShmAccess::Passthrough
        } as i32;
        if !with_user {
            spec.projects.clear();
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let mut runner = ComposeRunner::new(adapter.clone());
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let mut run = Box::pin(runner.run_with_ready(
            spec.clone(),
            async {
                let _ = stopped.await;
            },
            move |event: &Event| sink.borrow_mut().push(event.clone()),
            Some(ready),
        ));

        // Act: readiness is independently confirmed before requesting cooperative stop.
        let plan = tokio::time::timeout(RUN_LIMIT, async {
            tokio::select! { plan = started => match plan { Ok(plan) => plan, Err(_) => panic!("startup failed: {:?}", (&mut run).await) }, result = &mut run => panic!("startup failed: {result:?}") }
        }).await.unwrap();
        assert!(
            adapter
                .project(&plan)
                .inspect()
                .await
                .unwrap()
                .ready()
                .unwrap()
        );
        stop.send(()).unwrap();
        let report = tokio::time::timeout(RUN_LIMIT, run).await.unwrap().unwrap();

        // Assert: startup, graph lineage, shutdown and owned-resource cleanup are observable.
        assert!(report.successful(), "{report:?}");
        assert!(
            report.boundary_clean && report.writers_stopped,
            "{report:?}"
        );
        assert!(
            adapter
                .project(&plan)
                .inspect()
                .await
                .unwrap()
                .containers
                .is_empty()
        );
        assert!(
            fixture
                .docker(&[
                    "ps",
                    "-aq",
                    "--filter",
                    &format!("label=io.doramcap.run={}", spec.id)
                ])
                .stdout
                .is_empty()
        );
        let events = events.borrow();
        let event = |id: &str, state| {
            events
                .iter()
                .find(|e| e.invocation.step == id && e.state == state)
                .unwrap()
        };
        let runtime = event(StartDoraRuntime::ID.unwrap(), State::Succeeded);
        let graph = event(RegisterDoraGraph::ID.unwrap(), State::Succeeded);
        assert!(
            graph
                .invocation
                .dependencies
                .contains(&runtime.invocation.id)
        );
        let starts: Vec<_> = events
            .iter()
            .filter(|e| e.invocation.step == StartCompose::ID.unwrap() && e.state == State::Started)
            .collect();
        assert_eq!(
            starts.len(),
            1 + usize::from(isolated) + usize::from(with_user)
        );
        let systems = starts[usize::from(isolated)];
        assert!(
            systems
                .invocation
                .dependencies
                .contains(&graph.invocation.id)
        );
        if isolated {
            let holder: StartInput =
                serde_json::from_value(starts[0].payload.clone().unwrap()).unwrap();
            assert_eq!(holder.services, [SHM_SERVICE]);
            assert!(starts[0].elapsed_ns < runtime.elapsed_ns);
            assert!(
                systems
                    .invocation
                    .dependencies
                    .contains(&starts[0].invocation.id)
            );
        }
        let stops: Vec<_> = events
            .iter()
            .filter(|e| {
                e.invocation.step == QuiesceCompose::ID.unwrap() && e.state == State::Started
            })
            .collect();
        let systems = stops[usize::from(with_user)];
        let input: StopInput = serde_json::from_value(systems.payload.clone().unwrap()).unwrap();
        assert!(input.services.contains(&"system".into()));
        assert_eq!(
            input.services.iter().any(|name| name == SHM_SERVICE),
            isolated
        );
        if with_user {
            let input: StopInput =
                serde_json::from_value(stops[0].payload.clone().unwrap()).unwrap();
            assert_eq!(input.services, ["user"]);
        }
        assert!(
            systems.elapsed_ns < event(StopDoraRuntime::ID.unwrap(), State::Started).elapsed_ns
        );
        assert!(
            event(StopDoraRuntime::ID.unwrap(), State::Succeeded).elapsed_ns
                < event("compose.finalize", State::Started).elapsed_ns
        );
    }
}
