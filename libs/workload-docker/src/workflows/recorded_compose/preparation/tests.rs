use super::{model::*, *};

#[test]
fn merge_warns_about_unhandled_fields_and_preserves_service_settings() {
    // Arrange
    let mut model = json!({"services":{}});
    let mut diagnostics = Vec::new();
    let service = json!({"image":"app:local", "x-app-setting":"retained"});
    let input = json!({
        "name":"input-project", "version":"3.9",
        "services":{"app":service.clone()},
        "networks":{"default":{}}, "volumes":{"data":{}},
        "configs":{"settings":{"file":"settings.txt"}},
        "secrets":{"token":{"file":"token.txt"}},
        "future-section":{"option":true}, "x-project-setting":"omitted"
    });

    // Act
    diagnostics.extend(merge(&mut model, input).unwrap());

    // Assert
    assert_eq!(model["services"]["app"], service);
    assert!(model["volumes"]["data"].is_object());
    assert!(model["configs"]["settings"].is_object());
    assert!(model["secrets"]["token"].is_object());
    diagnostics.sort();
    assert_eq!(
        diagnostics,
        [
            "unsupported Compose top-level field 'future-section' is ignored",
            "unsupported Compose top-level field 'x-project-setting' is ignored",
        ]
    );
    assert!(model.get("future-section").is_none());

    // Act: identical default networks remain shareable across app inputs.
    diagnostics.extend(merge(&mut model, json!({"networks":{"default":{}}})).unwrap());

    // Assert
    assert_eq!(model["networks"]["default"], json!({}));
    assert_eq!(diagnostics.len(), 2);

    // Act: duplicate service names still fail.
    let result = merge(&mut model, json!({"services":{"app":service}}));

    // Assert
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Compose naming conflict: services/app")
    );
}

fn runtime() -> (
    crate::workflows::dora::DoraApps,
    workload_lifecycle::launch::DoraRuntime,
) {
    crate::workflows::dora::fixture("/run/experiment", [54001, 54002])
}

#[tokio::test]
async fn coordinator_mounts_the_frozen_graph_beside_writable_session_output() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.yml");
    tokio::fs::write(&source, "nodes: [{id: worker, path: dynamic}]")
        .await
        .unwrap();
    let (mut apps, mut shared) = runtime();
    let original = apps.coordinator().unwrap().1.clone();
    // Act
    crate::workflows::shared_dora::prepare(
        &mut apps,
        Some(&mut shared),
        source.to_str().unwrap(),
        directory.path(),
    )
    .await
    .unwrap();
    // Assert
    let coordinator = apps.coordinator().unwrap().1;
    assert_eq!(coordinator["user"], original["user"]);
    assert_eq!(coordinator["working_dir"], original["working_dir"]);
    assert_eq!(coordinator["volumes"][0], original["volumes"][0]);
    assert!(
        coordinator["volumes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|mount| mount["target"] == "/runtime/dataflow.yml" && mount["read_only"] == true)
    );
}

fn app(key: &str) -> ComposeProject {
    serde_json::from_value(json!({"key":key,"directory":"/apps/demo","files":["compose.yml"]}))
        .unwrap()
}

fn request() -> ComposeRun {
    ComposeRun {
        host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
        id: "run".into(),
        artifacts: "/runs/run".into(),
        filesystem: None,
        repository_root: "/tmp".into(),
        system_apps: vec![app("system")],
        projects: vec![app("adopted")],
        dora_apps: vec![],
        dataflow_file: String::new(),
        recordings: vec![],
        mcap_writers: Default::default(),
        readiness_seconds: 60,
        stop_seconds: 15,
    }
}

#[test]
fn app_input_preserves_source_settings_and_rejects_a_conflicting_runtime() {
    // Arrange
    let mut source = app("adopted");
    source.env_files.push("inputs.env".into());
    source.profiles.push("viewer".into());
    source
        .environment
        .insert("SETTING".into(), "explicit".into());
    let original = serde_json::to_value(&source).unwrap();

    // Act
    let input = app_input(&source, "/run/experiment").unwrap();

    // Assert
    assert_eq!(input.key, "experiment");
    assert_eq!(input.directory, source.directory);
    assert_eq!(input.files, source.files);
    assert_eq!(input.env_files, source.env_files);
    assert_eq!(input.profiles, source.profiles);
    assert_eq!(input.environment["SETTING"], "explicit");
    assert_eq!(
        input.environment["DORAMCAP_DORA_RUNTIME_DIR"],
        "/run/experiment"
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), original);

    // Act / Assert: an identical explicit setting is accepted, a conflicting one fails.
    source
        .environment
        .insert("DORAMCAP_DORA_RUNTIME_DIR".into(), "/run/experiment".into());
    assert!(app_input(&source, "/run/experiment").is_ok());
    source
        .environment
        .insert("DORAMCAP_DORA_RUNTIME_DIR".into(), "/other".into());
    assert!(
        app_input(&source, "/run/experiment")
            .unwrap_err()
            .to_string()
            .contains("conflicting Dora interpolation")
    );
}

#[test]
fn app_merge_tracks_services_and_preserves_external_volume_policy() {
    for (permission, expected) in [(false, "false"), (true, "true")] {
        // Arrange
        let mut model = json!({"services":{}});
        let mut services = vec!["previous".into()];
        let part = json!({
            "services":{"monitor":{"image":"app:test"},"recorder":{"image":"app:test"}},
            "volumes":{"data":{"labels":{"custom":"kept"}},
                "external-data":{"external":true,"name":"external-data","labels":{"io.doramcap.remove-volume":"false"}}}
        });

        // Act
        let warnings = merge_app(&mut model, part, permission, &mut services).unwrap();

        // Assert
        services.sort();
        assert_eq!(services, ["monitor", "previous", "recorder"]);
        assert!(warnings.is_empty());
        assert_eq!(
            model["volumes"]["data"]["labels"]["io.doramcap.remove-volume"],
            expected
        );
        assert_eq!(model["volumes"]["data"]["labels"]["custom"], "kept");
        assert_eq!(
            model["volumes"]["external-data"]["labels"]["io.doramcap.remove-volume"],
            "false"
        );
    }
}

#[test]
fn app_merge_rejects_reserved_volume_policy_and_missing_services() {
    for part in [
        json!({}),
        json!({"services":null}),
        json!({"services":{"app":{}},"volumes":{"data":{"labels":{"io.doramcap.remove-volume":"true"}}}}),
    ] {
        // Arrange
        let mut model = json!({"services":{}});

        // Act
        let result = merge_app(&mut model, part, false, &mut Vec::new());

        // Assert
        assert!(result.is_err());
        assert_eq!(model, json!({"services":{}}));
    }
}

#[test]
fn dora_wiring_preserves_native_ipc_mounts_networks_and_limits() {
    // Arrange
    let mut model: Value =
        serde_json::from_str(include_str!("fixtures/dora-wiring.input.json")).unwrap();
    let (dora, shared) = runtime();
    for (name, service) in &dora.services {
        model["services"][name] = service.clone();
    }
    let original = model.clone();
    // Act
    configure_dora(&mut model, &dora, Some(&shared), "experiment-owner".into()).unwrap();
    // Assert
    for (name, app) in original["services"].as_object().unwrap() {
        assert_eq!(model["services"][name]["network_mode"], app["network_mode"]);
        assert_eq!(model["services"][name]["ulimits"], app["ulimits"]);
        if !dora.services.contains_key(name) {
            assert_eq!(model["services"][name]["ipc"], app["ipc"]);
            assert_eq!(model["services"][name]["shm_size"], app["shm_size"]);
            assert_eq!(
                model["services"][name]["environment"][dora_app::environment::DAEMON_PORT],
                "54002"
            );
        }
    }
}

#[test]
fn stop_grace_defaults_to_stop_seconds_and_keeps_declared_periods() {
    // Arrange
    let mut model = json!({"services":{
        "unset": {},
        "short": {"stop_grace_period": "2s"},
        "minimum": {"stop_grace_period": "1s"},
        "limit": {"stop_grace_period": "5m0s"},
    }});

    // Act
    configure_stop_grace(&mut model, 30).unwrap();

    // Assert
    assert_eq!(model["services"]["unset"]["stop_grace_period"], "30s");
    assert_eq!(model["services"]["short"]["stop_grace_period"], "2s");
    assert_eq!(model["services"]["minimum"]["stop_grace_period"], "1s");
    assert_eq!(model["services"]["limit"]["stop_grace_period"], "5m0s");
}

#[test]
fn stop_grace_refuses_periods_out_of_range_or_unreadable() {
    for period in ["5m1s", "1h0m0s", "500ms", "0s", "soon"] {
        // Arrange
        let mut model = json!({"services":{"perception":{"stop_grace_period": period}}});

        // Act
        let error = configure_stop_grace(&mut model, 30)
            .unwrap_err()
            .to_string();

        // Assert: the error names the service to change.
        assert!(error.contains("perception"), "{error}");
    }
}

#[test]
fn dora_wiring_rejects_reserved_mounts_labels_and_service_names() {
    {
        let target = "/config/zenoh.json5";
        // Arrange
        let mut model = json!({"services":{"app":{"volumes":[{"type":"bind","source":"/data","target":target}]}}});

        // Act
        let (dora, shared) = runtime();
        let result = configure_dora(&mut model, &dora, Some(&shared), "owner".into());

        // Assert
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("app mount overrides")
        );
    }
    {
        let part = json!({"app":{"labels":{"io.doramcap.experiment-owner":"true"}}});
        // Arrange
        let mut model = json!({"services":part});

        // Act
        let (dora, shared) = runtime();
        let result = configure_dora(&mut model, &dora, Some(&shared), "owner".into());

        // Assert
        assert!(result.is_err());
    }
}

#[test]
fn graph_bindings_require_matching_services_and_host_networking() {
    // Arrange
    let graph = crate::workflows::shared_dora::graph(
        "/flow.yml",
        b"nodes:\n  - id: monitor\n    path: dynamic\n",
    )
    .unwrap();
    let mut model = json!({"services":{}});

    // Act / Assert: missing service fails, preserved bridge networking fails, host networking succeeds.
    assert!(
        validate_graph(&model, &graph)
            .unwrap_err()
            .to_string()
            .contains("same name")
    );
    model["services"]["monitor"] = json!({"networks":{"users":{}}});
    assert!(
        validate_graph(&model, &graph)
            .unwrap_err()
            .to_string()
            .contains("host networking")
    );
    model["services"]["monitor"] = json!({"network_mode":"host"});
    assert!(validate_graph(&model, &graph).is_ok());
}

#[test]
fn startup_dependencies_follow_system_apps_then_adopted_order() {
    // Arrange
    let system = vec!["monitor".into(), "recorder".into()];
    let users = vec!["adopted".into()];
    let mut model = json!({"services":{
        "monitor":{},"recorder":{"depends_on":{"monitor":{}}},
        "adopted":{"depends_on":{"recorder":{}}}
    }});

    // Act / Assert: same-stage dependencies and adopted-to-system dependencies are valid.
    assert!(validate_dependencies(&model, &system, &users).is_ok());
    model["services"]["monitor"]["depends_on"] = json!({"adopted":{}});
    assert!(
        validate_dependencies(&model, &system, &users)
            .unwrap_err()
            .to_string()
            .contains("cannot depend on adopted")
    );
}

#[test]
fn request_validation_rejects_invalid_paths_keys_and_budgets() {
    // Arrange
    let cases: [fn(&mut ComposeRun); 8] = [
        |s| s.id = "bad/id".into(),
        |s| s.artifacts = "relative".into(),
        |s| s.repository_root = "relative".into(),
        |s| {
            s.system_apps.clear();
            s.projects.clear();
        },
        |s| s.projects[0].key = s.system_apps[0].key.clone(),
        |s| s.projects[0].directory = "relative".into(),
        |s| s.readiness_seconds = 0,
        |s| s.stop_seconds = 301,
    ];
    assert!(validate(&request()).is_ok());
    for change in cases {
        let mut spec = request();
        change(&mut spec);

        // Act / Assert
        assert!(validate(&spec).is_err(), "{spec:?}");
    }
}

#[tokio::test]
async fn preparation_creates_missing_binds_docker_would_create_only_for_overlays() {
    for bind in [json!(null), json!({}), json!({"create_host_path":true})] {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("nested/data");
        let model = json!({"services":{"app":{"volumes":[
            {"type":"bind","source":source,"read_only":true,"bind":bind}
        ]}}});
        let binds = ResolvedBind::from_manifest(&model).unwrap();
        let mut frozen = model.clone();

        // Act: Review and plain runs leave the path for Docker.
        freeze_files(&mut frozen, root.path()).await.unwrap();
        prepare_sources(&binds, false).await.unwrap();

        // Assert
        assert_eq!(frozen, model);
        assert!(!source.exists());

        // Act
        prepare_sources(&binds, true).await.unwrap();

        // Assert
        assert!(source.is_dir());
    }
}

#[tokio::test]
async fn preparation_names_every_refused_bind_and_creates_nothing() {
    for overlays in [false, true] {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file");
        std::fs::write(&file, b"contents").unwrap();
        let refused = root.path().join("refused");
        let below_file = file.join("child");
        let creatable = root.path().join("creatable");
        let model = json!({"services":{
            "app":{"volumes":[{"type":"bind","source":refused,"bind":{"create_host_path":false}}]},
            "worker":{"volumes":[{"type":"bind","source":below_file}]},
            "other":{"volumes":[{"type":"bind","source":creatable}]}
        }});

        // Act
        let binds = ResolvedBind::from_manifest(&model).unwrap();
        let error = prepare_sources(&binds, overlays)
            .await
            .unwrap_err()
            .to_string();

        // Assert
        assert!(error.contains(&format!("app: bind mount source {}", refused.display())));
        assert!(error.contains(&format!(
            "worker: bind mount source {}",
            below_file.display()
        )));
        assert!(!error.contains("other"));
        assert!(!refused.exists());
        assert!(!creatable.exists());
    }
}

#[tokio::test]
async fn freezing_names_every_unreadable_file_input() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    std::fs::write(&file, b"contents").unwrap();
    let below_file = file.join("child");
    let below_file = below_file.to_str().unwrap();
    let source = "/nonexistent/doramcap/data";
    let mut model = json!({
        "services":{"app":{"volumes":[{"type":"bind","source":below_file,"read_only":true}]}},
        "configs":{"settings":{"file":source}},
        "secrets":{"token":{"file":source},"ambient":{"environment":"TOKEN"}}
    });

    // Act
    let error = freeze_files(&mut model, root.path())
        .await
        .unwrap_err()
        .to_string();

    // Assert
    assert!(error.contains(&format!("app: bind mount source {below_file}: ")));
    assert!(error.contains("configs/settings: file /nonexistent/doramcap/data: "));
    assert!(error.contains("secrets/token: file /nonexistent/doramcap/data: "));
    assert!(!error.contains("ambient"));
}

#[tokio::test]
async fn freezing_copies_files_and_permissions_while_writable_mounts_and_directories_stay_live() {
    use std::os::unix::fs::PermissionsExt;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let live = root.path().join("live");
    let directory = root.path().join("directory");
    std::fs::write(&source, b"original").unwrap();
    std::fs::write(&live, b"live").unwrap();
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut model = json!({"services":{"app":{"volumes":[
        {"type":"bind","source":source,"target":"/bin/app","read_only":true},
        {"type":"bind","source":live,"target":"/live"},
        {"type":"bind","source":directory,"target":"/directory","read_only":true}
    ]}},"configs":{"settings":{"file":source}},"secrets":{"token":{"file":source}}});

    // Act
    freeze_files(&mut model, root.path()).await.unwrap();
    std::fs::write(&source, b"changed").unwrap();

    // Assert
    let frozen = Path::new(
        model["services"]["app"]["volumes"][0]["source"]
            .as_str()
            .unwrap(),
    );
    assert!(frozen.starts_with(root.path().join("inputs")));
    assert_eq!(std::fs::read(frozen).unwrap(), b"original");
    assert_eq!(
        std::fs::metadata(frozen).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(
        std::fs::metadata(root.path().join("inputs"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        model["services"]["app"]["volumes"][1]["source"],
        live.to_str().unwrap()
    );
    assert_eq!(
        model["services"]["app"]["volumes"][2]["source"],
        directory.to_str().unwrap()
    );
    for kind in ["configs", "secrets"] {
        let entry = model[kind].as_object().unwrap().values().next().unwrap();
        assert_eq!(Path::new(entry["file"].as_str().unwrap()), frozen);
    }

    // Act / Assert: existing artifact directories are never reused.
    assert!(freeze_files(&mut model, root.path()).await.is_err());
}

#[test]
fn every_resource_namespace_rejects_duplicate_names() {
    for section in ["services", "networks", "volumes", "configs", "secrets"] {
        // Arrange
        let input = json!({section:{"shared":{}}});
        let mut model = json!({});
        merge(&mut model, input.clone()).unwrap();

        // Act
        let result = merge(&mut model, input);

        // Assert
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Compose naming conflict")
        );
    }
}

#[test]
fn malformed_app_environment_and_mounts_return_errors() {
    for settings in [json!({"environment":[]}), json!({"volumes":{}})] {
        // Arrange
        let mut model = json!({"services":{"app":settings}});

        // Act
        let (dora, shared) = runtime();
        let result = configure_dora(&mut model, &dora, Some(&shared), "owner".into());

        // Assert
        assert!(result.unwrap_err().to_string().contains("invalid Compose"));
    }
}

#[test]
fn dora_wiring_preserves_an_absent_network_section() {
    // Arrange
    let mut model = json!({"services":{"app":{"network_mode":"host"}}});
    // Act
    let (dora, shared) = runtime();
    configure_dora(&mut model, &dora, Some(&shared), "owner".into()).unwrap();
    // Assert: Compose requires an absent optional section to stay absent.
    assert!(model.get("networks").is_none());
}

#[test]
fn mcap_writer_needs_linux_6_1() {
    // Act / Assert
    for (release, supported) in [
        ("6.1.0-13-amd64", true),
        ("7.0.0-38-generic", true),
        ("6.0.19", false),
        ("5.15.0-91-generic", false),
        ("unknown", false),
    ] {
        assert_eq!(kernel_supports_mcap_writer(release), supported, "{release}");
    }
}

#[tokio::test]
async fn recording_host_check_ignores_other_runs_leftover_probes() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let leftover = root.path().join(".direct-io-probe-crashed");
    std::fs::write(&leftover, b"").unwrap();
    // Act
    validate_recording_host(&[root.path().to_owned()], "run")
        .await
        .unwrap();
    // Assert: only the other run's file remains.
    let names: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(names, [leftover]);
    // Act / Assert: a missing directory fails.
    assert!(
        validate_recording_host(&[root.path().join("missing")], "run")
            .await
            .is_err()
    );
}

#[test]
fn mcap_writers_get_unconfined_seccomp_and_memlock() {
    // Arrange
    let mut model = json!({"services":{"writer":{
        "security_opt":["no-new-privileges:true","seccomp=other.json"],
        "ulimits":{"memlock":{"soft":1,"hard":1}},
    },"other":{}}});
    // Act
    configure_mcap_writers(&mut model, &["writer".into()]).unwrap();
    // Assert
    assert_eq!(
        model["services"]["writer"]["security_opt"],
        json!(["no-new-privileges:true", "seccomp=unconfined"])
    );
    assert_eq!(
        model["services"]["writer"]["ulimits"]["memlock"]["hard"],
        268435456
    );
    assert!(model["services"]["other"].get("security_opt").is_none());
    // Act / Assert: an unknown service is refused.
    assert!(configure_mcap_writers(&mut model, &["absent".into()]).is_err());
}

#[test]
fn host_shm_access_requires_an_explicit_supported_mode() {
    // Arrange
    let mut spec = request();
    for mode in [0, -1, i32::MAX] {
        // Arrange
        spec.host_shm_access = mode;
        // Act
        let error = validate(&spec).unwrap_err().to_string();
        // Assert
        assert!(error.contains("host_shm_access"));
    }
    for mode in [
        workload_lifecycle::launch::HostShmAccess::Blocked,
        workload_lifecycle::launch::HostShmAccess::Passthrough,
    ] {
        // Arrange
        spec.host_shm_access = mode as i32;
        // Act
        let result = validate(&spec);
        let plan = spec.run_plan();
        // Assert
        assert!(result.is_ok());
        assert_eq!(plan.host_shm_access, mode as i32);
    }
}

#[test]
fn run_ipc_defaults_preserve_native_private_groups() {
    use workload_lifecycle::launch::HostShmAccess;
    // Arrange
    let original = json!({"services":{
        SHM_SERVICE:{"ipc":"shareable"},
        "default":{},
        "private":{"ipc":"private","shm_size":1234},
        "native-group":{"ipc":"service:private"},
        "private-mount":{"volumes":[{"type":"tmpfs","target":"/dev/shm"}]},
        "private-tmpfs":{"tmpfs":["/dev/shm:size=16384"]},
        "selective":{"volumes":[{"type":"bind","source":"/dev/shm/camera","target":"/dev/shm/camera","read_only":true}]}
    }});

    for mode in [HostShmAccess::Blocked, HostShmAccess::Passthrough] {
        // Arrange
        let mut model = original.clone();
        let mut systems = vec![SHM_SERVICE.into(), "default".into()];
        // Act
        configure_shm(&mut model, &mut systems, mode as i32).unwrap();
        // Assert
        assert_eq!(systems, ["default"]);
        for service in ["private", "native-group", "private-mount", "private-tmpfs"] {
            assert_eq!(model["services"][service], original["services"][service]);
        }
        let expected = if mode == HostShmAccess::Blocked {
            format!("service:{SHM_SERVICE}")
        } else {
            "host".into()
        };
        assert_eq!(model["services"]["default"]["ipc"], expected);
        assert_eq!(model["services"]["selective"]["ipc"], expected);
        assert_eq!(
            model["services"].get(SHM_SERVICE).is_some(),
            mode == HostShmAccess::Blocked
        );
    }

    // Arrange: only participants using run memory need the placeholder.
    let mut private = json!({"services":{"private":{"ipc":"private"}}});
    let mut missing = json!({"services":{"default":{}}});
    let mut conflict = json!({"services":{"sized":{"shm_size":1234}}});
    // Act
    let private_result = configure_shm(&mut private, &mut vec![], HostShmAccess::Blocked as i32);
    let missing_error = configure_shm(&mut missing, &mut vec![], HostShmAccess::Blocked as i32)
        .unwrap_err()
        .to_string();
    let conflict_error = configure_shm(
        &mut conflict,
        &mut vec![],
        HostShmAccess::Passthrough as i32,
    )
    .unwrap_err()
    .to_string();
    // Assert
    assert!(private_result.is_ok());
    assert!(missing_error.contains(SHM_SERVICE));
    assert!(conflict_error.contains("sized: shm_size"));
}

#[tokio::test]
async fn read_only_host_shm_stays_live_through_preparation_at_alternate_targets() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir_in("/dev/shm").unwrap();
    let source = shared.path().join("segment");
    let alias = root.path().join("alias");
    std::fs::write(&source, b"initial").unwrap();
    std::os::unix::fs::symlink(&source, &alias).unwrap();
    let mut model = json!({"services":{"app":{"volumes":[
        {"type":"bind","source":source,"target":"/dev/shm/segment","read_only":true},
        {"type":"bind","source":alias,"target":"/input/camera","read_only":true}
    ]}}});
    let original = model.clone();

    // Act
    freeze_files(&mut model, root.path()).await.unwrap();
    std::fs::write(&source, b"updated").unwrap();

    // Assert
    assert_eq!(model, original);
    assert_eq!(std::fs::read(alias).unwrap(), b"updated");
    assert_eq!(
        std::fs::read_dir(root.path().join("inputs"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn blocked_rejects_whole_host_access_but_keeps_selective_native_binds() {
    use workload_lifecycle::launch::HostShmAccess;
    // Arrange
    let shared = tempfile::tempdir_in("/dev/shm").unwrap();
    let alias_root = tempfile::tempdir().unwrap();
    let alias = alias_root.path().join("host-shm");
    std::os::unix::fs::symlink("/dev/shm", &alias).unwrap();

    for source in [
        Path::new("/dev/shm"),
        Path::new("/dev"),
        Path::new("/"),
        alias.as_path(),
    ] {
        // Arrange
        let model = json!({"services":{"camera":{"volumes":[
            {"type":"bind","source":source,"target":"/input/host"}
        ]}}});
        // Act
        let error = validate_host_shm(&model, HostShmAccess::Blocked as i32)
            .unwrap_err()
            .to_string();
        let passthrough = validate_host_shm(&model, HostShmAccess::Passthrough as i32);
        // Assert
        assert!(
            error.contains("camera") && error.contains("PASSTHROUGH"),
            "{error}"
        );
        assert!(passthrough.is_ok());
    }
    // Arrange
    let host_ipc = json!({"services":{"camera":{"ipc":"host"}}});
    // Act
    let error = validate_host_shm(&host_ipc, HostShmAccess::Blocked as i32)
        .unwrap_err()
        .to_string();
    let passthrough = validate_host_shm(&host_ipc, HostShmAccess::Passthrough as i32);
    // Assert
    assert!(error.contains("ipc: host"));
    assert!(passthrough.is_ok());

    for access in [HostShmAccess::Blocked, HostShmAccess::Passthrough] {
        // Arrange
        let selective = json!({"services":{"camera":{"ipc":"private","volumes":[
            {"type":"bind","source":shared.path(),"target":"/input/camera","read_only":true}
        ]}}});
        // Act
        let result = validate_host_shm(&selective, access as i32);
        // Assert
        assert!(result.is_ok());
    }
}

#[test]
fn blocked_checks_local_named_volume_host_sources() {
    use workload_lifecycle::launch::HostShmAccess;
    // Arrange
    let shared = tempfile::tempdir_in("/dev/shm").unwrap();
    let mut model = json!({"services":{"camera":{"volumes":[
        {"type":"volume","source":"host-memory","target":"/input"}
    ]}},"volumes":{"host-memory":{"driver":"local","driver_opts":{
        "type":"none","o":"ro,bind","device":"/dev/shm"
    }}}});

    // Act
    let error = validate_host_shm(&model, HostShmAccess::Blocked as i32)
        .unwrap_err()
        .to_string();
    let passthrough = validate_host_shm(&model, HostShmAccess::Passthrough as i32);
    // Assert
    assert!(
        error.contains("camera") && error.contains("PASSTHROUGH"),
        "{error}"
    );
    assert!(passthrough.is_ok());

    // Act: restrict the named volume to a selected host directory.
    model["volumes"]["host-memory"]["driver_opts"]["device"] =
        shared.path().to_str().unwrap().into();
    let selective = validate_host_shm(&model, HostShmAccess::Blocked as i32);
    // Assert
    assert!(selective.is_ok());
}

#[tokio::test]
#[ignore = "requires a local Docker Engine; run the built test executable with capabilities"]
async fn blocked_inspects_external_local_volume_sources_on_the_real_engine() {
    use std::collections::HashMap;
    use workload_lifecycle::launch::HostShmAccess;
    // Arrange
    let adapter = DockerAdapter::connect("/var/run/docker.sock")
        .await
        .unwrap();
    let unique = tempfile::tempdir().unwrap();
    let selected = tempfile::tempdir_in("/dev/shm").unwrap();
    let identity = unique
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .trim_start_matches('.');
    for (suffix, source, blocked) in [
        ("whole", Some(Path::new("/dev/shm")), true),
        ("selected", Some(selected.path()), false),
        ("ordinary", None, false),
    ] {
        let name = format!("codex-shm-{identity}-{suffix}");
        adapter
            .docker
            .create_volume(bollard::models::VolumeCreateRequest {
                name: Some(name.clone()),
                driver: Some("local".into()),
                driver_opts: source.map(|source| {
                    HashMap::from([
                        ("type".into(), "none".into()),
                        ("o".into(), "bind".into()),
                        ("device".into(), source.to_string_lossy().into_owned()),
                    ])
                }),
                ..Default::default()
            })
            .await
            .unwrap();
        let model = json!({"services":{"camera":{"volumes":[
            {"type":"volume","source":"external-shm","target":"/input"}
        ]}},"volumes":{"external-shm":{"external":true,"name":name}}});

        // Act
        let result =
            validate_external_volume_shm(&adapter, &model, HostShmAccess::Blocked as i32).await;
        let passthrough =
            validate_external_volume_shm(&adapter, &model, HostShmAccess::Passthrough as i32).await;
        adapter
            .docker
            .remove_volume(
                &name,
                None::<bollard::query_parameters::RemoveVolumeOptions>,
            )
            .await
            .unwrap();

        let removed = adapter.docker.inspect_volume(&name).await;

        // Assert
        assert_eq!(result.is_err(), blocked);
        if let Err(error) = result {
            assert!(
                error.to_string().contains("camera") && error.to_string().contains("PASSTHROUGH")
            );
        }
        passthrough.unwrap();
        assert!(removed.is_err());
    }
}
