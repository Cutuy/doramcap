//! Opt-in qualification of app-owned Compose deployments through the gallery.
use project_explorer_server::{
    lifecycle::Status, manager::Manager, registry::Registry, runtime::Service,
};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
use workload_docker::compose::DockerAdapter;

#[test]
#[ignore = "requires Docker access and unprivileged-prepared image contexts"]
fn build_deployment_images() {
    // Arrange
    let contexts = PathBuf::from(std::env::var_os("DORAMCAP_DEPLOY_CONTEXTS").unwrap());
    // Act / Assert
    for app in [
        "dora",
        "fault-demo",
        "fault-watchdog",
        "cpu-viewer",
        "mcap-recorder",
    ] {
        let status = std::process::Command::new("docker")
            .args(["build", "--tag", &format!("doramcap/{app}:local")])
            .arg(contexts.join(app))
            .status()
            .unwrap();
        assert!(status.success(), "{app} image build");
    }
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()));
        } else {
            assert!(entry.file_type().unwrap().is_file());
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}

async fn curl(
    address: std::net::SocketAddr,
    route: &str,
    body: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut command = tokio::process::Command::new("curl");
    command.args(["--silent", "--show-error", "--fail", "--max-time", "5"]);
    if let Some(body) = body {
        command.args(["--header", "Content-Type: application/json", "--data", body]);
    }
    let output = command
        .arg(format!("http://{address}{route}"))
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    Ok(output.stdout)
}

async fn qualify(app: &str, registration: &str) {
    // Arrange
    let source = PathBuf::from(std::env::var_os("DORAMCAP_TEST_WORKSPACE").unwrap());
    let root = std::env::temp_dir().join(format!(
        "gallery-dora-{}-{app}-{registration}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let workspace = root.join("workspace");
    let deploy = workspace.join("apps").join(app).join("deploy");
    copy_tree(&source.join("apps").join(app).join("deploy"), &deploy);
    copy_tree(
        &source.join("apps/fault-watchdog/config"),
        &workspace.join("apps/fault-watchdog/config"),
    );
    if registration != "registration.pbtxt" {
        std::fs::copy(deploy.join(registration), deploy.join("registration.pbtxt")).unwrap();
    }
    let data = root.join("data");
    std::fs::create_dir(&data).unwrap();
    let registry = Registry::load(&workspace, &[app.into()]).unwrap();
    let adapter = Rc::new(
        DockerAdapter::connect(std::env::var("DORAMCAP_TEST_SOCKET").unwrap())
            .await
            .unwrap(),
    );
    let manager = Manager::discover_at(registry, &adapter, Duration::from_secs(300), &data)
        .await
        .unwrap();
    // SAFETY: UID/GID access has no side effects.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let collector = if registration.contains("otel") {
        let received = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = received.clone();
        let router = axum::Router::new().route(
            "/v1/logs",
            axum::routing::post(move |bytes: axum::body::Bytes| {
                let counter = counter.clone();
                async move {
                    if !bytes.is_empty() {
                        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    (
                        [("Content-Type", "application/x-protobuf")],
                        Vec::<u8>::new(),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:4318")
            .await
            .unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Some((task, received))
    } else {
        None
    };
    let service = Service::spawn_local(manager, adapter.clone(), data.clone(), uid, gid);

    // Act: join a generation, verify live traffic, then idle-stop its last viewers.
    let launch = service.launch(app).await.unwrap();
    let mut logs = None;
    let validation = async {
        let joined = service.launch(app).await.map_err(|e| format!("{e:?}"))?;
        if joined.start || joined.generation != launch.generation {
            return Err("viewer did not join".to_owned());
        }
        let deadline = Instant::now() + Duration::from_secs(100);
        loop {
            match service.status(app).await.map_err(|e| format!("{e:?}"))? {
                Status::Ready { .. } => break,
                Status::Starting { .. } if Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                status => return Err(format!("startup: {status:?}")),
            }
        }
        if registration.contains("mcap") {
            let containers = adapter
                .discover_runs(&format!("gal-{:032x}", launch.generation))
                .await
                .map_err(|e| e.to_string())?;
            let watchdog = containers
                .iter()
                .find(|r| {
                    r.kind == "container"
                        && r.artifacts
                            .as_ref()
                            .is_some_and(|a| Path::new(a).starts_with(&data))
                })
                .ok_or("watchdog container missing")?;
            logs = Some(
                tokio::process::Command::new("docker")
                    .args([
                        "logs",
                        "--follow",
                        &format!("{}-watchdog-1", watchdog.project.as_deref().unwrap()),
                    ])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                    .map_err(|e| e.to_string())?,
            );
        }
        if let Some(address) = service
            .upstream(app, launch.generation)
            .await
            .map_err(|e| format!("{e:?}"))?
        {
            curl(address, "/", None).await?;
            let route = if app == "cpu-viewer" {
                "/health/ready"
            } else {
                "/api/state"
            };
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                let result = curl(address, route, None).await;
                if let Ok(bytes) = &result
                    && (app == "cpu-viewer"
                        || serde_json::from_slice::<serde_json::Value>(bytes)
                            .is_ok_and(|v| v["complete"] == true))
                {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(format!("live data unavailable: {result:?}"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if app != "cpu-viewer" {
                curl(
                    address,
                    "/api/control",
                    Some(r#"{"source":"a","mode":"fail"}"#),
                )
                .await?;
                tokio::time::sleep(Duration::from_millis(700)).await;
                let state: serde_json::Value =
                    serde_json::from_slice(&curl(address, "/api/state", None).await?)
                        .map_err(|e| e.to_string())?;
                if !state.to_string().contains("simulated I/O error") {
                    return Err(format!("control did not reach reporter: {state}"));
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok::<_, String>(())
    }
    .await;
    service
        .expire(Instant::now() + Duration::from_secs(600))
        .await;
    let expired = service.status(app).await.unwrap();
    let stopped = service.shutdown().await;

    if let Some(logs) = logs
        && let Ok(Ok(output)) =
            tokio::time::timeout(Duration::from_secs(5), logs.wait_with_output()).await
    {
        eprintln!(
            "writer stdout: {}\nwriter stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if let Some((task, received)) = collector {
        task.abort();
        assert!(
            received.load(std::sync::atomic::Ordering::Relaxed) > 0,
            "OTLP collector received no logs"
        );
    }
    // Assert: clean producer/drain completion precedes result publication and removal.
    assert!(
        validation.is_ok(),
        "{app}/{registration}: {validation:?}; shutdown={stopped:?}"
    );
    assert!(
        matches!(expired, Status::Stopping { .. } | Status::Stopped { .. }),
        "idle expiry: {expired:?}"
    );
    let state = service.status(app).await.unwrap();
    assert_eq!(state, Status::Stopped { error: None }, "{root:?}");
    let (complete, result, _) = service.recording_paths(app, launch.generation).unwrap();
    assert!(complete.is_file() && result.is_file(), "{root:?}");
    assert!(
        adapter
            .discover_runs(&format!("gal-{:032x}", launch.generation))
            .await
            .unwrap()
            .is_empty()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker, BPF capabilities, built images and Dora CLI"]
async fn gallery_dora_deployments() {
    tokio::task::LocalSet::new()
        .run_until(async {
            for (app, registration) in [
                ("fault-demo", "registration.pbtxt"),
                ("fault-watchdog", "registration.pbtxt"),
                ("fault-watchdog", "registration.mcap.pbtxt"),
                ("fault-watchdog", "registration.otel.pbtxt"),
                ("cpu-viewer", "registration.pbtxt"),
                ("mcap-recorder", "registration.pbtxt"),
            ] {
                if std::env::var("DORAMCAP_GALLERY_CASE")
                    .is_ok_and(|case| case != format!("{app}/{registration}"))
                {
                    continue;
                }
                Box::pin(qualify(app, registration)).await;
            }
        })
        .await;
}
