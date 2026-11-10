//! Native Compose cgroup placement. Build unprivileged; elevate only this test executable.
use orchestrator::{Event, State, Step};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    rc::Rc,
    time::Duration,
};
use workload_docker::{
    compose::{ComposeProject, DockerAdapter, RemoveCompose, StartCompose},
    workflows::recorded_compose::{ComposeRun, ComposeRunner, preparation::PrepareRun},
};
use workload_lifecycle::{
    launch::{HostShmAccess, SystemAppCgroup},
    recording::WaitForRecordings,
};
use workload_systemd::cgroup::{
    Action, CGROUP_ROOT, IDENTITY_FILE, SystemAppSlice, SystemAppsCgroups, SystemAppsCgroupsInput,
};

const RUN_LIMIT: Duration = Duration::from_secs(30);
const STARTUP_SECONDS: u64 = 10;
const STOP_SECONDS: u64 = 3;
const RUN_LABEL: &str = "label=io.doramcap.run";
const SERVICE_LABEL: &str = "label=com.docker.compose.service";

struct Fixture {
    directory: tempfile::TempDir,
    socket: String,
    image: String,
    prefix: String,
    slice: String,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let prefix = format!("cgroup-native-{}", std::process::id());
        let fixture = Self {
            slice: SystemAppSlice::compose_name(directory.path()).unwrap(),
            directory,
            socket: "/var/run/docker.sock".into(),
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
            fs::write(
                fixture.directory.path().join(format!("{file}.json")),
                serde_json::to_vec(&serde_json::json!({"services":{name:service}})).unwrap(),
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
        let pool = prost_reflect::DescriptorPool::decode(
            include_bytes!(concat!(env!("OUT_DIR"), "/workflow_config.bin")).as_slice(),
        )
        .unwrap();
        prost_reflect::DynamicMessage::parse_text_format(
            pool.get_message_by_name("doramcap.launch.v1.SystemAppCgroup")
                .unwrap(),
            &fs::read_to_string(self.spec(index).artifacts.join(IDENTITY_FILE)).unwrap(),
        )
        .unwrap()
        .transcode_to()
        .unwrap()
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
        let parent = format!(
            "{}.slice",
            self.slice.strip_suffix("-system.slice").unwrap()
        );
        let _ = Command::new("systemctl")
            .args(["stop", &self.slice, &parent])
            .output();
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
#[ignore = "requires root, local Docker with Compose, systemd and cgroup v2"]
async fn system_app_placement_survives_controller_crash_and_cleans_up() {
    // Arrange
    let fixture = Fixture::new();
    let adapter = Rc::new(DockerAdapter::connect(&fixture.socket).await.unwrap());
    let mut previous = None;
    for index in 0..2 {
        let mut runner = ComposeRunner::new(adapter.clone());
        let (ready, started) = tokio::sync::oneshot::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let mut run = Box::pin(runner.run_with_ready(
            fixture.spec(index),
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
            assert!(created_index < launched_index);
            let output: Option<SystemAppCgroup> =
                serde_json::from_value(events[created_index].payload.clone().unwrap()).unwrap();
            assert_eq!(output, Some(identity.clone()));
        }
        if let Some(id) = previous {
            assert_ne!(id, identity.id);
        }
        previous = Some(identity.id);
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
            assert!(!Path::new(&identity.path).exists());
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
                [
                    i32::from(Action::Remove),
                    i32::from(Action::Create),
                    i32::from(Action::Remove)
                ]
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
#[ignore = "requires root, local Docker with Compose, systemd and cgroup v2"]
async fn misplaced_system_app_prevents_user_start() {
    // Arrange
    let fixture = Fixture::new();
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
            fixture.spec(0),
            std::future::pending(),
            move |event: &Event| {
                if event.invocation.step
                    == WaitForRecordings::ID.unwrap_or(std::any::type_name::<WaitForRecordings>())
                    && event.state == State::Succeeded
                    && !observed.get()
                {
                    let service = format!("{SERVICE_LABEL}=system");
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
}
