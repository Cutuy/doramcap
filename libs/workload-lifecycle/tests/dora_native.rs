//! Qualified native CLI reads the shared session artifact and accepts the prepared Start.
use dora_control::{
    DoraCoordinatorClient,
    protocol::{DataflowId, SessionId},
};
use std::{
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    process::Stdio,
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    time::Instant,
};
use workload_lifecycle::dora;
const STARTUP_BUDGET: Duration = Duration::from_secs(30);
const STOP_BUDGET: Duration = Duration::from_secs(10);
struct Runtime {
    children: Vec<Child>,
    directory: PathBuf,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.start_kill();
        }
    }
}
#[tokio::test]
#[ignore = "requires DORAMCAP_TEST_DORA naming the qualified Dora 1.1.0 executable"]
async fn native_start_session_and_authentication_match_the_cli() {
    let executable = std::env::var_os("DORAMCAP_TEST_DORA").expect("Dora executable required");
    for authenticated in [false, true] {
        // Arrange: the coordinator and daemon share the native run-local working directory.
        let directory =
            std::env::temp_dir().join(format!("dora-shared-native-{}", DataflowId::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let mut runtime = Runtime {
            children: vec![],
            directory,
        };
        let coordinator = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let daemon = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = coordinator.local_addr().unwrap();
        let daemon_port = daemon.local_addr().unwrap().port();
        drop((coordinator, daemon));
        let deadline = Instant::now() + STARTUP_BUDGET;
        let coordinator_log =
            std::fs::File::create(runtime.directory.join("coordinator.log")).unwrap();
        let mut command = Command::new(&executable);
        command.args([
            "coordinator",
            "--interface",
            "127.0.0.1",
            "--port",
            &endpoint.port().to_string(),
            "--store",
            "memory",
        ]);
        if authenticated {
            command.arg("--auth");
        }
        runtime.children.push(
            command
                .kill_on_drop(true)
                .env_clear()
                .env("HOME", &runtime.directory)
                .current_dir(&runtime.directory)
                .stdin(Stdio::null())
                .stdout(coordinator_log.try_clone().unwrap())
                .stderr(coordinator_log)
                .spawn()
                .unwrap(),
        );
        // Act: resolve exactly the native run-local credential, without ambient discovery.
        let token = dora::token(&runtime.directory, authenticated, deadline)
            .await
            .unwrap_or_else(|e| panic!("{e}; logs: {}", runtime.directory.display()));
        // Assert
        assert_eq!(token.is_some(), authenticated);
        if authenticated {
            let mut wrong = DoraCoordinatorClient::new(endpoint, Some("0".repeat(64)));
            assert!(wrong.connect(deadline).await.is_err());
            assert!(wrong.graph().is_none());
        }
        let daemon_log = std::fs::File::create(runtime.directory.join("daemon.log")).unwrap();
        runtime.children.push(
            Command::new(&executable)
                .kill_on_drop(true)
                .args([
                    "daemon",
                    "--coordinator-port",
                    &endpoint.port().to_string(),
                    "--local-listen-port",
                    &daemon_port.to_string(),
                ])
                .env_clear()
                .env("HOME", &runtime.directory)
                .current_dir(&runtime.directory)
                .stdin(Stdio::null())
                .stdout(daemon_log.try_clone().unwrap())
                .stderr(daemon_log)
                .spawn()
                .unwrap(),
        );
        let mut client = DoraCoordinatorClient::new(endpoint, token.clone());
        client
            .wait_for_daemon(deadline)
            .await
            .unwrap_or_else(|e| panic!("{e}; logs: {}", runtime.directory.display()));
        let bytes = b"nodes: [{id: external, path: dynamic, outputs: [data]}]";
        let graph_path = runtime.directory.join("dataflow.yml");
        std::fs::write(&graph_path, bytes).unwrap();
        let session = SessionId::generate();
        let start = dora::start(
            dora::graph(graph_path.to_str().unwrap(), bytes).unwrap(),
            "shared-preparation".into(),
            runtime.directory.clone(),
            session,
            None,
        )
        .unwrap();
        let artifact = runtime.directory.join(dora::SESSION_FILE);
        std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        std::fs::write(
            &artifact,
            serde_json::to_vec_pretty(&dora::session_artifact(session)).unwrap(),
        )
        .unwrap();
        // Act / Assert: prepared Start registers a real graph and retains its UUID.
        let uuid = client
            .start_graph(start, deadline)
            .await
            .unwrap_or_else(|e| panic!("{e}; logs: {}", runtime.directory.display()));
        assert_eq!(client.graph().unwrap().uuid, Some(uuid));
        client.inspect_graph(deadline).await.unwrap();
        // Act: an independent native CLI reads the generated artifact for a second named graph.
        let mut cli = Command::new(&executable);
        cli.args([
            "start",
            graph_path.to_str().unwrap(),
            "--name",
            "cli-parity",
            "--detach",
            "--coordinator-port",
            &endpoint.port().to_string(),
        ])
        .env_clear()
        .env("HOME", &runtime.directory)
        .current_dir(&runtime.directory)
        .stdin(Stdio::null())
        .kill_on_drop(true);
        if let Some(token) = token {
            cli.env("DORA_AUTH_TOKEN", token);
        }
        let output = tokio::time::timeout_at(deadline, cli.output())
            .await
            .unwrap()
            .unwrap();
        // Assert: the CLI accepts our artifact and retains the exact generated session ID.
        assert!(
            output.status.success(),
            "{}; logs: {}",
            String::from_utf8_lossy(&output.stderr),
            runtime.directory.display()
        );
        let native_session = std::fs::read_to_string(&artifact).unwrap();
        assert!(
            native_session
                .lines()
                .any(|line| line == format!("session_id: {}", session.uuid()))
        );
        // Act / Assert: exclusive-runtime shutdown stops both named graphs and native processes.
        client.shutdown(Instant::now() + STOP_BUDGET).await.unwrap();
        for child in &mut runtime.children {
            let status = tokio::time::timeout(STOP_BUDGET, child.wait())
                .await
                .unwrap()
                .unwrap();
            assert!(status.success(), "logs: {}", runtime.directory.display());
        }
        std::fs::remove_dir_all(&runtime.directory).unwrap();
    }
}
