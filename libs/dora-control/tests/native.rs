//! Real coordinator/daemon qualification using the repository's existing Dora test input.
use dora_control::{
    DoraCoordinatorClient,
    protocol::{DataflowId, SessionId, cli_to_coordinator::ControlRequest},
};
use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    process::Stdio,
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    time::Instant,
};

const STARTUP_BUDGET: Duration = Duration::from_secs(30);
const STOP_BUDGET: Duration = Duration::from_secs(10);
const RUN_NAME: &str = "dora-control-native";
struct Runtime {
    directory: PathBuf,
    children: Vec<Child>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.start_kill();
        }
    }
}

#[tokio::test]
#[ignore = "requires DORAMCAP_TEST_DORA to name a Dora 1.1.0 executable"]
async fn real_coordinator_registers_inspects_and_stops_a_dynamic_graph() {
    // Arrange
    let executable =
        std::env::var_os("DORAMCAP_TEST_DORA").expect("DORAMCAP_TEST_DORA names a Dora binary");
    let directory =
        std::env::temp_dir().join(format!("dora-control-native-{}", DataflowId::new_v4()));
    fs::create_dir(&directory).unwrap();
    let mut runtime = Runtime {
        directory,
        children: Vec::new(),
    };
    let coordinator = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let daemon = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let endpoint = coordinator.local_addr().unwrap();
    let daemon_port = daemon.local_addr().unwrap().port();
    drop((coordinator, daemon));
    for (name, args) in [
        (
            "coordinator",
            vec![
                "coordinator".to_owned(),
                "--interface".into(),
                Ipv4Addr::LOCALHOST.to_string(),
                "--port".into(),
                endpoint.port().to_string(),
                "--store".into(),
                "memory".into(),
            ],
        ),
        (
            "daemon",
            vec![
                "daemon".to_owned(),
                "--coordinator-port".into(),
                endpoint.port().to_string(),
                "--local-listen-port".into(),
                daemon_port.to_string(),
            ],
        ),
    ] {
        let log = fs::File::create(runtime.directory.join(format!("{name}.log"))).unwrap();
        runtime.children.push(
            Command::new(&executable)
                .kill_on_drop(true)
                .args(args)
                .env_clear()
                .env("HOME", &runtime.directory)
                .current_dir(&runtime.directory)
                .stdin(Stdio::null())
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
    }
    let startup = Instant::now() + STARTUP_BUDGET;
    let mut client = DoraCoordinatorClient::new(endpoint, None);
    // Act
    client
        .wait_for_daemon(startup)
        .await
        .unwrap_or_else(|error| panic!("{error}; logs: {}", runtime.directory.display()));
    let uuid = client.start_graph(ControlRequest::Start {
        build_id: None,
        session_id: SessionId::generate(),
        dataflow: serde_json::from_value(serde_json::json!({"nodes": [{"id": "external", "path": "dynamic", "outputs": ["messages"]}]})).unwrap(),
        name: Some(RUN_NAME.into()),
        local_working_dir: Some(runtime.directory.clone()),
        uv: false,
        write_events_to: None,
    }, startup).await.unwrap_or_else(|error| panic!("{error}; logs: {}", runtime.directory.display()));
    // Assert
    assert_eq!(client.graph().unwrap().uuid, Some(uuid));
    client.inspect_graph(startup).await.unwrap();
    // Act
    client
        .shutdown(Instant::now() + STOP_BUDGET)
        .await
        .unwrap_or_else(|error| panic!("{error}; logs: {}", runtime.directory.display()));
    // Assert: protocol shutdown completes before native children are reaped.
    for child in &mut runtime.children {
        let status = tokio::time::timeout(STOP_BUDGET, child.wait())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "native child did not exit; logs: {}",
                    runtime.directory.display()
                )
            })
            .unwrap();
        assert!(status.success(), "logs: {}", runtime.directory.display());
    }
}
