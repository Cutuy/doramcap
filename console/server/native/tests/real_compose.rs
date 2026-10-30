use std::{collections::BTreeMap, process::Command};
use workload_docker::compose::{ComposeProject, DockerAdapter};

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker Compose and access to its socket"]
async fn default_discovery_matches_explicit_inputs_and_profiles() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("compose.yaml"), "services:\n  worker:\n    image: '${IMAGE}'\n    environment:\n      VALUE: '${VALUE}'\n  extra:\n    image: '${IMAGE}'\n    profiles: [optional]\n").unwrap();
    std::fs::write(
        root.path().join("compose.override.yaml"),
        "services:\n  worker:\n    labels:\n      example: overridden\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join(".env"),
        "IMAGE=example:local\nVALUE=from-file\n",
    )
    .unwrap();
    let project = ComposeProject {
        key: "workload".into(),
        directory: root.path().into(),
        files: vec!["compose.yaml".into(), "compose.override.yaml".into()],
        env_files: vec![".env".into()],
        environment: BTreeMap::new(),
        profiles: vec![],
        services: vec![],
        completed_services: vec![],
        remove_volumes: false,
    };
    let adapter = DockerAdapter::connect("/var/run/docker.sock")
        .await
        .unwrap();
    let compose = |explicit: bool| {
        let mut cmd = Command::new("docker");
        cmd.current_dir(root.path())
            .env_clear()
            .env("PATH", "/usr/bin:/bin");
        cmd.args(["compose", "--project-name", "console-discovery-test"]);
        if explicit {
            cmd.args([
                "--env-file",
                "/dev/null",
                "--env-file",
                ".env",
                "-f",
                "compose.yaml",
                "-f",
                "compose.override.yaml",
            ]);
        }
        let output = cmd.args(["config", "--format", "json"]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    // Act
    let profiles = adapter.profiles(&project).await.unwrap();
    let implicit = compose(false);
    let explicit = compose(true);
    // Assert
    assert_eq!(profiles, ["optional"]);
    assert_eq!(implicit, explicit);
    assert_eq!(
        explicit["services"]["worker"]["environment"]["VALUE"],
        "from-file"
    );
    assert_eq!(
        explicit["services"]["worker"]["labels"]["example"],
        "overridden"
    );
}

#[test]
#[ignore = "requires local Docker Compose"]
fn default_and_explicit_discovery_match_for_supported_names() {
    // Arrange
    for base in [
        "compose.yaml",
        "compose.yml",
        "docker-compose.yaml",
        "docker-compose.yml",
    ] {
        for suffix in ["yaml", "yml"] {
            let root = tempfile::tempdir().unwrap();
            let prefix = if base.starts_with("docker-") {
                "docker-compose"
            } else {
                "compose"
            };
            let override_name = format!("{prefix}.override.{suffix}");
            std::fs::write(root.path().join(base), "include: [included.yaml]\nservices:\n  worker:\n    image: '${IMAGE}'\n    env_file: service.env\n").unwrap();
            std::fs::write(
                root.path().join("included.yaml"),
                "services:\n  extra:\n    image: '${IMAGE}'\n    profiles: [optional]\n",
            )
            .unwrap();
            std::fs::write(
                root.path().join(&override_name),
                "services:\n  worker:\n    labels:\n      source: override\n",
            )
            .unwrap();
            std::fs::write(root.path().join(".env"), "IMAGE=example:local\n").unwrap();
            std::fs::write(root.path().join("service.env"), "FROM_FILE=present\n").unwrap();
            let resolve = |explicit: bool, profile: bool| {
                let mut command = Command::new("docker");
                command
                    .current_dir(root.path())
                    .env_clear()
                    .env("PATH", "/usr/bin:/bin");
                command.args(["compose", "--project-name", "console-discovery-test"]);
                if explicit {
                    command.args([
                        "--env-file",
                        "/dev/null",
                        "--env-file",
                        ".env",
                        "-f",
                        base,
                        "-f",
                        &override_name,
                    ]);
                }
                if profile {
                    command.args(["--profile", "optional"]);
                }
                let output = command
                    .args(["config", "--format", "json"])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
            };
            // Act / Assert
            for profile in [false, true] {
                let implicit = resolve(false, profile);
                assert_eq!(implicit, resolve(true, profile), "{base} / {override_name}");
                assert_eq!(
                    implicit["services"]["worker"]["environment"]["FROM_FILE"],
                    "present"
                );
                assert_eq!(implicit["services"].get("extra").is_some(), profile);
            }
            // Act / Assert: adding a service is reflected without a runner change.
            std::fs::write(
                root.path().join("included.yaml"),
                "services:\n  added:\n    image: '${IMAGE}'\n",
            )
            .unwrap();
            let added = resolve(false, false);
            assert_eq!(added, resolve(true, false));
            assert!(added["services"].get("added").is_some());
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker, a prebuilt image and overlay namespace access"]
async fn rejected_preflight_creates_no_owned_resources_or_mounts() {
    use std::{future, rc::Rc};
    use workload_docker::{
        compose::Error,
        workflows::{
            recorded_compose::{ComposeRun, ComposeRunner},
            system_apps::SystemAppsConfig,
        },
    };
    // Arrange
    let images = Command::new("docker")
        .args(["image", "ls", "--no-trunc", "--format", "{{.ID}}"])
        .output()
        .unwrap();
    assert!(images.status.success());
    let image_list = String::from_utf8(images.stdout).unwrap();
    let image = image_list
        .lines()
        .find(|id| !id.is_empty())
        .expect("at least one prebuilt local image required");
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("compose.yaml"),
        format!("services:\n  worker:\n    image: '{image}'\n"),
    )
    .unwrap();
    let run_id = format!("consoleguard{}", std::process::id());
    let adapter = Rc::new(
        DockerAdapter::connect("/var/run/docker.sock")
            .await
            .unwrap(),
    );
    let mut runner = ComposeRunner::new(adapter.clone());
    let spec = ComposeRun {
        filesystem: Some(Default::default()),
        id: run_id.clone(),
        artifacts: root.path().join("run"),
        recovery_anchor_dir: None,
        projects: vec![ComposeProject {
            key: "workload".into(),
            directory: root.path().into(),
            files: vec!["compose.yaml".into()],
            env_files: vec![],
            environment: BTreeMap::new(),
            profiles: vec![],
            services: vec![],
            completed_services: vec![],
            remove_volumes: false,
        }],
        first_party: vec![],
        system_apps: SystemAppsConfig::default(),
        readiness_seconds: 5,
        stop_seconds: 1,
    };
    assert!(adapter.discover_runs(&run_id).await.unwrap().is_empty());
    // Act
    let result = runner
        .run_with_preflight(
            spec,
            future::pending(),
            |_: &orchestrator::Event| {},
            None,
            |_| Err(Error::Invalid("rejected".into())),
        )
        .await;
    // Assert
    assert!(result.is_err());
    assert!(!runner.blocked());
    assert!(adapter.discover_runs(&run_id).await.unwrap().is_empty());
    let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    assert!(!mounts.contains(root.path().to_str().unwrap()));
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker and a prebuilt image"]
async fn review_resolves_selected_profiles_and_changed_environment() {
    use console_native::{
        prepare,
        protocol::{ConsoleConfig, PrepareRequest},
    };
    use std::rc::Rc;
    use workload_docker::workflows::system_apps::{CpuMonitor, SystemAppsConfig};
    // Arrange
    let images = Command::new("docker")
        .args(["image", "ls", "--no-trunc", "--format", "{{.ID}}"])
        .output()
        .unwrap();
    let image_list = String::from_utf8(images.stdout).unwrap();
    let image = image_list
        .lines()
        .find(|id| !id.is_empty())
        .expect("prebuilt image required");
    let root = tempfile::tempdir().unwrap();
    let project_dir = root.path().join("project");
    let data_dir = root.path().join("data");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(project_dir.join("compose.yaml"), format!("services:\n  worker:\n    image: '{image}'\n    environment:\n      VALUE: '${{VALUE}}'\n  optional:\n    image: '{image}'\n    profiles: [extra]\n")).unwrap();
    std::fs::write(project_dir.join(".env"), "VALUE=before\n").unwrap();
    let config = ConsoleConfig {
        runtime: None,
        listen_address: "127.0.0.1:8080".into(),
        public_origin: "http://127.0.0.1:8080".into(),
        project_root: root.path().to_string_lossy().into_owned(),
        data_root: data_dir.to_string_lossy().into_owned(),
        docker_socket: "/var/run/docker.sock".into(),
        access_secret_file: "/tmp/secret".into(),
        system_apps: Some(SystemAppsConfig {
            cpu_monitor: Some(CpuMonitor {
                image: image.into(),
            }),
            ..Default::default()
        }),
    };
    let adapter = Rc::new(
        DockerAdapter::connect("/var/run/docker.sock")
            .await
            .unwrap(),
    );
    let request = PrepareRequest {
        system_apps: None,
        directory: project_dir.to_string_lossy().into_owned(),
        profiles: vec!["extra".into()],
    };
    // Act
    let invalid = PrepareRequest {
        system_apps: None,
        directory: request.directory.clone(),
        profiles: vec!["not-declared".into()],
    };
    assert!(
        prepare::prepare(adapter.clone(), &config, invalid)
            .await
            .is_err()
    );
    let first = prepare::prepare(adapter.clone(), &config, request.clone())
        .await
        .unwrap()
        .into_launchable()
        .unwrap();
    let launch_dir = root.path().join("launch-resolution");
    std::fs::create_dir(&launch_dir).unwrap();
    let unchanged = adapter
        .resolve(workload_docker::compose::ResolveInput {
            run: first.review.run_id.clone(),
            artifacts: launch_dir.clone(),
            project: first.project.clone(),
        })
        .await
        .unwrap();
    // Assert: routine diagnostics stay private and do not become a generic banner.
    assert!(first.review.warnings.is_empty());
    assert!(
        unchanged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("process-level readiness only"))
    );
    assert_eq!(unchanged.sha256, first.workload_digest);
    std::fs::write(project_dir.join(".env"), "VALUE=after\n").unwrap();
    let changed_dir = root.path().join("changed-resolution");
    std::fs::create_dir(&changed_dir).unwrap();
    let changed = adapter
        .resolve(workload_docker::compose::ResolveInput {
            run: first.review.run_id.clone(),
            artifacts: changed_dir,
            project: first.project.clone(),
        })
        .await
        .unwrap();
    assert_ne!(changed.sha256, first.workload_digest);
    let second = prepare::prepare(adapter.clone(), &config, request.clone())
        .await
        .unwrap()
        .into_launchable()
        .unwrap();
    // Act: deselecting the app must not resolve even a missing monitor image.
    let mut no_monitor = config.clone();
    no_monitor
        .system_apps
        .as_mut()
        .unwrap()
        .cpu_monitor
        .as_mut()
        .unwrap()
        .image = "console-missing-monitor:selection-test".into();
    let disabled = prepare::prepare(
        adapter,
        &no_monitor,
        PrepareRequest {
            system_apps: Some(console_native::protocol::SystemAppSelection::default()),
            ..request
        },
    )
    .await
    .unwrap()
    .into_launchable()
    .unwrap();
    // Assert
    assert!(disabled.monitor_image.is_none());
    assert!(disabled.system_apps.cpu_monitor.is_none());
    assert!(!disabled.review.system_apps[0].selected);
    assert!(!disabled.directory.join("system-services.yaml").exists());
    assert!(first.review.system_apps[0].selected);
    // Assert
    assert_eq!(first.review.available_profiles, ["extra"]);
    assert_eq!(first.review.services.len(), 2);
    assert_eq!(first.monitor_image, second.monitor_image);
    // Run IDs affect labels and project names, so compare resolved environment directly.
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(first.directory.join("workload.compose.json")).unwrap(),
    )
    .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(
        &std::fs::read(second.directory.join("workload.compose.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["services"]["worker"]["environment"]["VALUE"],
        "before"
    );
    assert_eq!(
        updated["services"]["worker"]["environment"]["VALUE"],
        "after"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires local Docker Compose"]
async fn profile_only_project_reports_available_profiles() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("compose.yaml"),
        "services:\n  worker:\n    image: example:local\n    profiles: [extra]\n",
    )
    .unwrap();
    let adapter = DockerAdapter::connect("/var/run/docker.sock")
        .await
        .unwrap();
    let project = console_native::project::discover(root.path(), root.path(), vec![]).unwrap();
    // Act
    let profiles = adapter.profiles(&project).await.unwrap();
    let error = adapter
        .resolve(workload_docker::compose::ResolveInput {
            run: "profileonly".into(),
            artifacts: root.path().into(),
            project,
        })
        .await
        .unwrap_err();
    // Assert
    assert_eq!(profiles, ["extra"]);
    assert_eq!(error.to_string(), "selection contains no services");
    let config = console_native::protocol::ConsoleConfig {
        runtime: None,
        listen_address: "127.0.0.1:8080".into(),
        public_origin: "http://127.0.0.1:8080".into(),
        project_root: root.path().to_string_lossy().into_owned(),
        data_root: root.path().join("data").to_string_lossy().into_owned(),
        docker_socket: "/var/run/docker.sock".into(),
        access_secret_file: "/tmp/secret".into(),
        system_apps: Some(workload_docker::workflows::system_apps::SystemAppsConfig {
            cpu_monitor: Some(workload_docker::workflows::system_apps::CpuMonitor {
                image: "unused".into(),
            }),
            ..Default::default()
        }),
    };
    let review = console_native::prepare::prepare(
        std::rc::Rc::new(adapter),
        &config,
        console_native::protocol::PrepareRequest {
            system_apps: None,
            directory: root.path().to_string_lossy().into_owned(),
            profiles: vec![],
        },
    )
    .await
    .unwrap();
    let console_native::prepare::Preparation::NeedsProfiles(review) = review else {
        panic!("expected profile selection")
    };
    assert!(review.prepared_id.is_empty());
    assert_eq!(review.available_profiles, ["extra"]);
}
