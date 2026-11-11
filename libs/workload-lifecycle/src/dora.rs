//! Dora graph preparation and explicit access resolved from native launch definitions.
use dora_control::protocol::{SessionId, cli_to_coordinator::ControlRequest};
use dora_core::descriptor::{Descriptor, DescriptorExt, validate::check_dataflow_static};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tokio::{io::AsyncReadExt, time::Instant};

/// Native Dora token representation: 32 random bytes encoded as 64 hexadecimal characters.
const TOKEN_HEX_BYTES: usize = 64;
const DEFAULT_INTERFACE: &str = "127.0.0.1";
/// Native CLI session file for the captured `dataflow.yml` graph.
pub const SESSION_FILE: &str = "out/dataflow.dora-session.yaml";

/// Expand and validate an externally launched dynamic graph relative to its source.
/// Build, operator, ROS, Git and hub nodes are rejected before native startup.
pub fn graph(dataflow_file: &str, bytes: &[u8]) -> Result<Descriptor, String> {
    let graph = Descriptor::parse(bytes.to_vec())
        .map_err(|e| format!("invalid Dora graph: {e:#}"))?
        .expand(
            Path::new(dataflow_file)
                .parent()
                .ok_or("graph parent missing")?,
        )
        .map_err(|e| format!("Dora module expansion: {e:#}"))?;
    check_dataflow_static(&graph).map_err(|e| format!("invalid Dora graph: {e:#}"))?;
    if graph.nodes.iter().any(|n| {
        n.path.as_deref() != Some("dynamic")
            || n.operators.is_some()
            || n.operator.is_some()
            || n.ros2.is_some()
            || n.build.is_some()
            || n.git.is_some()
            || n.hub.is_some()
    }) {
        return Err("Dora graph must use externally launched dynamic nodes".into());
    }
    Ok(graph)
}

/// Map the captured dynamic graph to one named Start using explicit session and native paths.
/// Uses the native CLI defaults: no build, uv disabled, and explicit optional event output.
pub fn start(
    graph: Descriptor,
    name: String,
    directory: PathBuf,
    session_id: SessionId,
    write_events_to: Option<PathBuf>,
) -> Result<ControlRequest, String> {
    if name.is_empty() || !directory.is_absolute() {
        return Err("Dora Start requires a run name and absolute runtime working directory".into());
    }
    Ok(ControlRequest::Start {
        build_id: None,
        session_id,
        dataflow: graph,
        name: Some(name),
        local_working_dir: Some(directory),
        uv: false,
        write_events_to,
    })
}

/// Native dynamic-graph session artifact, serialized as JSON (also valid YAML).
/// Backend writers persist this with the same session ID supplied to Start.
pub fn session_artifact(session_id: SessionId) -> serde_json::Value {
    serde_json::json!({"build_id": null, "session_id": session_id,
        "git_sources": {}, "local_build": null})
}

/// Resolve the owned coordinator's loopback endpoint; CLI interface takes precedence over native env.
/// Only loopback or wildcard listeners are supported; port must be explicit and nonzero.
pub fn endpoint(argv: &[String], native_interface: Option<&str>) -> Result<SocketAddr, String> {
    let explicit = argv
        .iter()
        .any(|arg| arg == "--interface" || arg.starts_with("--interface="));
    let interface: IpAddr = if explicit {
        option(argv, "--interface")?
    } else {
        native_interface.unwrap_or(DEFAULT_INTERFACE)
    }
    .parse()
    .map_err(|_| "Dora coordinator interface must be an IP address")?;
    let loopback = match interface {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip if ip.is_loopback() => ip,
        _ => return Err("Dora coordinator interface must include host loopback".into()),
    };
    Ok(SocketAddr::new(loopback, port(argv, "--port")?))
}

/// Read an authenticated coordinator's private run-local token before the startup deadline.
/// Watches creation without polling; rejects symlinks, foreign ownership, loose modes and invalid tokens.
/// `authenticated=false` is the native coordinator's default mode and performs no credential discovery.
pub async fn token(
    directory: &Path,
    authenticated: bool,
    deadline: Instant,
) -> Result<Option<String>, String> {
    if !authenticated {
        return Ok(None);
    }
    let read = async {
        use std::os::unix::fs::MetadataExt;
        let (_watcher, mut changes) =
            crate::files::watch_changes([directory]).map_err(|e| e.to_string())?;
        let path = dora_control::protocol::auth::token_path(directory);
        loop {
            let opened = tokio::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .await;
            let file = match opened {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    crate::files::wait_for_change(&mut changes)
                        .await
                        .map_err(|e| e.to_string())?;
                    continue;
                }
                Err(_) => return Err("Dora run-local token is unreadable".into()),
            };
            let metadata = file
                .metadata()
                .await
                .map_err(|_| "Dora token metadata unavailable")?;
            if !metadata.is_file()
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.mode() & 0o077 != 0
                || metadata.len() > (TOKEN_HEX_BYTES + 1) as u64
            {
                return Err("Dora run-local token requires current-user ownership and private file permissions".into());
            }
            let mut bytes = Vec::new();
            file.take((TOKEN_HEX_BYTES + 2) as u64)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| "Dora run-local token is unreadable")?;
            let value = std::str::from_utf8(&bytes)
                .map_err(|_| "invalid Dora run-local token")?
                .trim();
            if bytes.len() > TOKEN_HEX_BYTES + 1
                || value.len() != TOKEN_HEX_BYTES
                || !value.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("invalid Dora run-local token".into());
            }
            return Ok(Some(value.to_owned()));
        }
    };
    tokio::time::timeout_at(deadline, read)
        .await
        .map_err(|_| "Dora token startup deadline expired")?
}

/// Read one explicit native CLI option; duplicate, missing and empty values fail.
pub fn option<'a>(argv: &'a [String], name: &str) -> Result<&'a str, String> {
    let prefix = format!("{name}=");
    let mut found = None;
    for (index, arg) in argv.iter().enumerate() {
        let value = if arg == name {
            Some(
                argv.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("missing {name} value"))?,
            )
        } else {
            arg.strip_prefix(&prefix)
        };
        if let Some(value) = value
            && (value.is_empty() || value.starts_with("--") || found.replace(value).is_some())
        {
            return Err(format!("ambiguous or empty {name}"));
        }
    }
    found.ok_or_else(|| format!("Dora app command must declare {name}"))
}

/// Read a nonzero TCP port from an app's native CLI option.
pub fn port(argv: &[String], name: &str) -> Result<u16, String> {
    option(argv, name)?
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| format!("{name} must be in 1..=65535"))
}

/// Select per-run endpoints for zero ports and rewrite the two native commands.
/// Explicit coordinator references must agree; the two resolved ports must differ.
pub fn resolve_ports(
    coordinator: &mut [String],
    daemon: &mut [String],
) -> Result<[bool; 2], String> {
    let read = |args: &[String], flag| {
        option(args, flag)?
            .parse::<u16>()
            .map_err(|e| e.to_string())
    };
    let mut ports = [
        read(coordinator, "--port")?,
        read(daemon, "--local-listen-port")?,
    ];
    if read(daemon, "--coordinator-port")? != ports[0] {
        return Err("Dora app coordinator endpoints disagree".into());
    }
    let automatic = ports.map(|port| port == 0);
    let mut listeners = Vec::new();
    for port in &mut ports {
        if *port == 0 {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .map_err(|e| e.to_string())?;
            *port = listener.local_addr().map_err(|e| e.to_string())?.port();
            listeners.push(listener);
        }
    }
    if ports[0] == ports[1] {
        return Err("Dora ports must differ".into());
    }
    fn replace(args: &mut [String], flag: &str, port: u16) {
        let prefix = format!("{flag}=");
        for index in 0..args.len() {
            if args[index] == flag {
                args[index + 1] = port.to_string();
            } else if args[index].starts_with(&prefix) {
                args[index] = format!("{prefix}{port}");
            }
        }
    }
    replace(coordinator, "--port", ports[0]);
    replace(daemon, "--coordinator-port", ports[0]);
    replace(daemon, "--local-listen-port", ports[1]);
    Ok(automatic)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dynamic_start_preserves_graph_paths_and_session_artifact() {
        // Arrange
        let bytes = b"nodes: [{id: external, path: dynamic, outputs: [data]}]";
        let descriptor = graph("/source/dataflow.yml", bytes).unwrap();
        let session = SessionId::generate();
        // Act
        let request = start(
            descriptor,
            "run-1".into(),
            "/native/runtime".into(),
            session,
            Some("events.jsonl".into()),
        )
        .unwrap();
        let artifact = session_artifact(session);
        // Assert
        let ControlRequest::Start {
            build_id,
            session_id,
            dataflow,
            name,
            local_working_dir,
            uv,
            write_events_to,
        } = request
        else {
            panic!("Start expected");
        };
        assert!(build_id.is_none());
        assert_eq!(session_id, session);
        assert_eq!(
            artifact["session_id"],
            serde_json::to_value(session).unwrap()
        );
        assert_eq!(artifact["git_sources"], serde_json::json!({}));
        assert_eq!(dataflow.nodes[0].id.as_ref(), "external");
        assert_eq!(name.as_deref(), Some("run-1"));
        assert_eq!(
            local_working_dir.as_deref(),
            Some(Path::new("/native/runtime"))
        );
        assert!(!uv);
        assert_eq!(write_events_to.as_deref(), Some(Path::new("events.jsonl")));
        assert!(
            graph(
                "/source/dataflow.yml",
                b"nodes: [{id: built, path: dynamic, build: make}]"
            )
            .is_err()
        );
    }

    #[test]
    fn direct_endpoint_obeys_native_interface_precedence() {
        // Arrange
        let args = |extra: &[&str]| {
            ["dora", "coordinator", "--port=6013"]
                .into_iter()
                .chain(extra.iter().copied())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        // Act / Assert
        assert_eq!(
            endpoint(&args(&[]), None).unwrap().to_string(),
            "127.0.0.1:6013"
        );
        assert_eq!(
            endpoint(&args(&["--interface=::"]), None)
                .unwrap()
                .to_string(),
            "[::1]:6013"
        );
        assert_eq!(
            endpoint(&args(&["--interface=127.0.0.1"]), Some("192.0.2.1"))
                .unwrap()
                .to_string(),
            "127.0.0.1:6013"
        );
        assert!(endpoint(&args(&[]), Some("192.0.2.1")).is_err());
        assert!(endpoint(&args(&["--interface=127.0.0.1", "--interface=::1"]), None).is_err());
    }

    #[tokio::test]
    async fn token_access_is_explicit_private_and_bounded() {
        use std::{
            os::unix::fs::{OpenOptionsExt, PermissionsExt},
            time::Duration,
        };
        // Arrange
        let directory = std::env::temp_dir().join(format!(
            "dora-token-{}",
            dora_control::protocol::DataflowId::new_v4()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = dora_control::protocol::auth::token_path(&directory);
        let deadline = || Instant::now() + Duration::from_secs(2);
        let secret = "a".repeat(TOKEN_HEX_BYTES);
        std::fs::write(&path, &secret).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        // Act / Assert: disabled auth does no discovery; enabled auth rejects loose files.
        assert_eq!(token(&directory, false, deadline()).await.unwrap(), None);
        assert!(
            token(&directory, true, deadline())
                .await
                .unwrap_err()
                .contains("private file")
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            token(&directory, true, deadline()).await.unwrap(),
            Some(secret.clone())
        );
        // Act / Assert: creation after subscription wakes the same wait, without sampling.
        std::fs::remove_file(&path).unwrap();
        let create = async {
            tokio::task::yield_now().await;
            let temporary = directory.join("token.tmp");
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temporary)
                .unwrap();
            use std::io::Write;
            (&file).write_all(secret.as_bytes()).unwrap();
            drop(file);
            std::fs::rename(temporary, &path).unwrap();
        };
        let (result, ()) = tokio::join!(token(&directory, true, deadline()), create);
        assert_eq!(result.unwrap(), Some(secret));
        // Act / Assert: missing credentials exhaust the caller's deadline, never establish no-auth.
        std::fs::remove_file(&path).unwrap();
        assert!(
            token(&directory, true, Instant::now())
                .await
                .unwrap_err()
                .contains("deadline")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ports_are_automatic_or_preserved_and_invalid_pairs_fail() {
        // Arrange
        let coordinator = |port| {
            vec![
                "dora".into(),
                "coordinator".into(),
                format!("--port={port}"),
            ]
        };
        let daemon = |target, port| {
            vec![
                "dora".into(),
                "daemon".into(),
                format!("--coordinator-port={target}"),
                format!("--local-listen-port={port}"),
            ]
        };
        let (mut c, mut d) = (coordinator(0), daemon(0, 0));
        // Act
        let automatic = resolve_ports(&mut c, &mut d).unwrap();
        // Assert
        assert_eq!(automatic, [true, true]);
        assert_eq!(port(&c, "--port"), port(&d, "--coordinator-port"));
        assert_ne!(port(&c, "--port"), port(&d, "--local-listen-port"));
        // Act / Assert: explicit settings remain literal configuration.
        let (mut c, mut d) = (coordinator(6013), daemon(6013, 6014));
        assert_eq!(resolve_ports(&mut c, &mut d).unwrap(), [false, false]);
        assert_eq!(port(&c, "--port"), Ok(6013));
        assert_eq!(port(&d, "--local-listen-port"), Ok(6014));
        assert!(resolve_ports(&mut coordinator(6013), &mut daemon(6014, 6015)).is_err());
        assert!(resolve_ports(&mut coordinator(6013), &mut daemon(6013, 6013)).is_err());
    }

    #[test]
    fn native_options_accept_both_forms_and_reject_ambiguous_endpoints() {
        // Arrange / Act / Assert
        for args in [
            vec!["dora", "coordinator", "--port", "6013"],
            vec!["dora", "coordinator", "--port=6013"],
        ] {
            assert_eq!(
                port(
                    &args.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                    "--port"
                ),
                Ok(6013)
            );
        }
        for args in [
            vec!["--port"],
            vec!["--port", "--store"],
            vec!["--port=0"],
            vec!["--port=65536"],
            vec!["--port=6013", "--port", "6014"],
        ] {
            assert!(
                port(
                    &args.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                    "--port"
                )
                .is_err()
            );
        }
    }
}
