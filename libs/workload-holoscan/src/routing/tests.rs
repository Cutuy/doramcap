use super::*;
use serde_json::json;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// Native SHA-256 context-directory name for the literal context "local".
const LOCAL_CONTEXT: &str = "25bf8e1a2393f1108d37029b3df5593236c755742ec93465bbafa9b290bddcf6";
struct Fixture {
    root: PathBuf,
    cwd: PathBuf,
    config: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-routing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let cwd = root.join("project");
        let config = root.join("home/.docker");
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&config).unwrap();
        Self { root, cwd, config }
    }
    fn environment(&self) -> NativeEnvironment {
        NativeEnvironment {
            home: Some(self.root.join("home")),
            ..Default::default()
        }
    }
    fn context(&self, host: &str) {
        let path = self.config.join("contexts/meta").join(LOCAL_CONTEXT);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("meta.json"),
            json!({"Name":"local","Endpoints":{"docker":{"Host":host}}}).to_string(),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[tokio::test]
async fn missing_config_and_virtual_default_use_native_linux_socket() {
    // Arrange
    let fixture = Fixture::new();
    let mut environment = fixture.environment();
    // Act / Assert
    assert_eq!(
        resolve(&environment, &fixture.cwd).await.unwrap(),
        Path::new(DEFAULT_SOCKET)
    );
    fs::write(
        fixture.config.join("config.json"),
        r#"{"currentContext":"default"}"#,
    )
    .unwrap();
    assert_eq!(
        resolve(&environment, &fixture.cwd).await.unwrap(),
        Path::new(DEFAULT_SOCKET)
    );
    environment.context = Some("default".into());
    assert_eq!(
        resolve(&environment, &fixture.cwd).await.unwrap(),
        Path::new(DEFAULT_SOCKET)
    );
}

#[tokio::test]
async fn selected_native_context_uses_its_hashed_metadata_and_ignores_auth_fields() {
    // Arrange
    let fixture = Fixture::new();
    fixture.context("unix:///run/user/1000/docker.sock");
    fs::write(
        fixture.config.join("config.json"),
        r#"{"currentContext":"local","auths":{"registry":{"auth":"do-not-disclose"}}}"#,
    )
    .unwrap();
    // Act
    let socket = resolve(&fixture.environment(), &fixture.cwd).await.unwrap();
    // Assert
    assert_eq!(socket, Path::new("/run/user/1000/docker.sock"));
}

#[tokio::test]
async fn docker_host_wins_over_context_as_observed_in_docker_29_9_1() {
    // Arrange: Docker 29.9.1 context show returns "default" for this pair.
    let fixture = Fixture::new();
    fs::write(
        fixture.config.join("config.json"),
        "malformed unused config",
    )
    .unwrap();
    let environment = NativeEnvironment {
        host: Some(" unix:///nonexistent/holoscan-daemon.sock ".into()),
        context: Some("nonexistent-holoscan-context".into()),
        tls: true,
        ..fixture.environment()
    };
    // Act
    let socket = resolve(&environment, &fixture.cwd).await.unwrap();
    // Assert: neither the named context nor unused config can change the host selection.
    assert_eq!(socket, Path::new("/nonexistent/holoscan-daemon.sock"));
}

#[tokio::test]
async fn docker_context_overrides_current_context_without_reading_unused_root_config() {
    // Arrange
    let fixture = Fixture::new();
    fixture.context("unix:///tmp/selected.sock");
    fs::write(
        fixture.config.join("config.json"),
        "malformed unused config",
    )
    .unwrap();
    let environment = NativeEnvironment {
        context: Some("local".into()),
        ..fixture.environment()
    };
    // Act / Assert
    assert_eq!(
        resolve(&environment, &fixture.cwd).await.unwrap(),
        Path::new("/tmp/selected.sock")
    );
}

#[tokio::test]
async fn relative_docker_config_and_home_resolve_against_the_cli_child_cwd() {
    // Arrange
    let fixture = Fixture::new();
    fixture.context("unix:///tmp/selected.sock");
    fs::write(
        fixture.config.join("config.json"),
        r#"{"currentContext":"local"}"#,
    )
    .unwrap();
    let configured = NativeEnvironment {
        config: Some("../home/.docker".into()),
        home: None,
        ..Default::default()
    };
    let home = NativeEnvironment {
        home: Some("../home".into()),
        ..Default::default()
    };
    // Act / Assert
    for environment in [configured, home] {
        assert_eq!(
            resolve(&environment, &fixture.cwd).await.unwrap(),
            Path::new("/tmp/selected.sock")
        );
    }
}

#[tokio::test]
async fn tls_presence_changes_only_the_implicit_default_host() {
    // Arrange
    let fixture = Fixture::new();
    // Act / Assert
    for (tls, tls_verify) in [(true, false), (false, true)] {
        let mut environment = NativeEnvironment {
            tls,
            tls_verify,
            ..fixture.environment()
        };
        assert!(
            resolve(&environment, &fixture.cwd)
                .await
                .unwrap_err()
                .to_string()
                .contains("TCP default")
        );
        environment.host = Some("unix://".into());
        assert_eq!(
            resolve(&environment, &fixture.cwd).await.unwrap(),
            Path::new(DEFAULT_SOCKET)
        );
        environment.host = Some("   ".into());
        assert!(resolve(&environment, &fixture.cwd).await.is_err());
        environment.host = None;
        environment.context = Some("local".into());
        fixture.context("unix:///tmp/context.sock");
        assert_eq!(
            resolve(&environment, &fixture.cwd).await.unwrap(),
            Path::new("/tmp/context.sock")
        );
    }
}

#[tokio::test]
async fn remote_relative_and_non_unix_selected_endpoints_fail_without_credential_output() {
    // Arrange
    let fixture = Fixture::new();
    // Act / Assert
    for host in [
        "tcp://user:secret@host:2376",
        "ssh://host",
        "https://host",
        "fd://3",
        "npipe:////./pipe/docker_engine",
        "unix://relative",
        "unix:///tmp/x\0",
        "/tmp/docker.sock",
    ] {
        let environment = NativeEnvironment {
            host: Some(host.into()),
            ..fixture.environment()
        };
        let error = resolve(&environment, &fixture.cwd)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("absolute Unix"), "{error}");
        assert!(!error.contains("secret"));
        fixture.context(host);
        let environment = NativeEnvironment {
            context: Some("local".into()),
            ..fixture.environment()
        };
        assert!(resolve(&environment, &fixture.cwd).await.is_err());
    }
}

#[tokio::test]
async fn selected_malformed_unreadable_and_missing_context_inputs_fail_explicitly() {
    // Arrange
    let fixture = Fixture::new();
    let environment = fixture.environment();
    // Act / Assert: malformed routing type; opaque auth fields never enter the error.
    fs::write(
        fixture.config.join("config.json"),
        r#"{"currentContext":17,"auths":{"secret":"do-not-disclose"}}"#,
    )
    .unwrap();
    let error = resolve(&environment, &fixture.cwd)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("config.json"));
    assert!(!error.contains("do-not-disclose"));
    // Act / Assert: selected root path is unreadable as a regular JSON file.
    fs::remove_file(fixture.config.join("config.json")).unwrap();
    fs::create_dir(fixture.config.join("config.json")).unwrap();
    assert!(
        resolve(&environment, &fixture.cwd)
            .await
            .unwrap_err()
            .to_string()
            .contains("cannot read Docker routing input")
    );
    fs::remove_dir(fixture.config.join("config.json")).unwrap();
    // Act / Assert: selected context has no metadata; no fallback to another daemon.
    let environment = NativeEnvironment {
        context: Some("local".into()),
        ..environment
    };
    assert!(
        resolve(&environment, &fixture.cwd)
            .await
            .unwrap_err()
            .to_string()
            .contains("meta.json")
    );
    fixture.context("unix:///tmp/context.sock");
    fs::write(
        fixture
            .config
            .join("contexts/meta")
            .join(LOCAL_CONTEXT)
            .join("meta.json"),
        r#"{"Endpoints":{"docker":{"Host":17}},"opaque":"do-not-disclose"}"#,
    )
    .unwrap();
    let error = resolve(&environment, &fixture.cwd)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("routing fields"));
    assert!(!error.contains("do-not-disclose"));
}

#[tokio::test]
async fn exact_absolute_child_cwd_and_config_origin_are_required() {
    // Arrange
    let fixture = Fixture::new();
    let environment = fixture.environment();
    // Act / Assert
    assert!(
        resolve(&environment, Path::new("relative"))
            .await
            .unwrap_err()
            .to_string()
            .contains("working directory")
    );
    assert!(
        resolve(&NativeEnvironment::default(), &fixture.cwd)
            .await
            .unwrap_err()
            .to_string()
            .contains("configuration directory")
    );
}

struct Daemon {
    socket: PathBuf,
    task: tokio::task::JoinHandle<()>,
}
impl Daemon {
    fn new(root: &Path, name: &str, id: Option<&str>) -> Self {
        let socket = root.join(name);
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let id = id.map(str::to_owned);
        let task = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                // Namespace peer checks connect without making an HTTP request.
                if stream.read_line(&mut line).await.unwrap() == 0 {
                    continue;
                }
                let path = line.split_whitespace().nth(1).unwrap().to_owned();
                loop {
                    line.clear();
                    if stream.read_line(&mut line).await.unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                }
                let value = if path.ends_with("/version") {
                    json!({"ApiVersion":"1.56","MinAPIVersion":"1.24","Version":"29.9.1"})
                } else {
                    assert!(path.ends_with("/info"), "unexpected Engine request: {path}");
                    json!({"ID":id,"OSType":"linux"})
                };
                let body = value.to_string();
                stream.get_mut().write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        Self { socket, task }
    }
    fn engine(&self) -> Engine {
        Engine::new(self.socket.to_str().unwrap(), 1).unwrap()
    }
}
impl Drop for Daemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn matching_daemon_ids_are_read_directly_from_both_unix_sockets() {
    // Arrange: distinct aliases may connect to the same daemon; equality is by nonempty ID.
    let fixture = Fixture::new();
    let native = Daemon::new(&fixture.root, "native.sock", Some("same-daemon"));
    let console = Daemon::new(&fixture.root, "console.sock", Some("same-daemon"));
    // Act
    let id = verify_socket(&native.socket, &console.engine())
        .await
        .unwrap();
    // Assert
    assert_eq!(id, "same-daemon");
}

#[tokio::test]
async fn mismatched_or_missing_daemon_ids_block_before_any_mutation() {
    // Arrange
    let fixture = Fixture::new();
    let console = Daemon::new(&fixture.root, "console.sock", Some("console-daemon"));
    // Act / Assert
    for (index, id) in [Some("different-daemon"), Some(""), Some("  "), None]
        .into_iter()
        .enumerate()
    {
        let native = Daemon::new(&fixture.root, &format!("native-{index}.sock"), id);
        let error = verify_socket(&native.socket, &console.engine())
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("different daemon IDs") || error.contains("missing or empty"),
            "{error}"
        );
    }
}

#[tokio::test]
#[ignore = "requires access to the native Linux Docker socket and its mount namespace; read-only"]
async fn real_native_daemon_preflight_matches_the_local_engine() {
    // Arrange
    let console = Engine::connect(DEFAULT_SOCKET).await.unwrap();
    let cwd = std::env::current_dir().unwrap();
    // Act
    let id = verify_native_daemon(&cwd, &console).await.unwrap();
    // Assert
    assert!(!id.trim().is_empty());
}
