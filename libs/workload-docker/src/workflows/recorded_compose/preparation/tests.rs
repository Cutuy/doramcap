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

fn runtime() -> crate::workflows::dora::DoraRuntime {
    crate::workflows::dora::DoraRuntime {
        image: "dora:test".into(),
        coordinator_port: 54001,
        daemon_port: 54002,
        runtime_directory: "/run/experiment".into(),
        zenoh_overlay: "/run/experiment/zenoh.json5".into(),
        shm_bytes: 134217728,
        memlock_bytes: 67108864,
        participant_uid: Some(1000),
        participant_gid: Some(1000),
        ..Default::default()
    }
}

fn app(key: &str) -> ComposeProject {
    serde_json::from_value(json!({"key":key,"directory":"/apps/demo","files":["compose.yml"]}))
        .unwrap()
}

fn request() -> ComposeRun {
    ComposeRun {
        id: "run".into(),
        artifacts: "/runs/run".into(),
        filesystem: None,
        repository_root: "/tmp".into(),
        system_apps: vec![app("system")],
        projects: vec![app("adopted")],
        dora: runtime(),
        recordings: vec![],
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
    let input = app_input(&source, &runtime()).unwrap();

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
    assert!(app_input(&source, &runtime()).is_ok());
    source
        .environment
        .insert("DORAMCAP_DORA_RUNTIME_DIR".into(), "/other".into());
    assert!(
        app_input(&source, &runtime())
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
fn dora_wiring_shares_ipc_preserves_networks_and_limits_environment_overrides() {
    // Arrange
    let mut model: Value =
        serde_json::from_str(include_str!("fixtures/dora-wiring.input.json")).unwrap();
    // The controller scope varies with the host user; all other expected values are literal.
    let scope = super::super::reset::directory();
    let expected: Value = serde_json::from_str(
        &include_str!("fixtures/dora-wiring.expected.json")
            .replace("{{controller_scope}}", scope.to_str().unwrap()),
    )
    .unwrap();

    // Act
    let warnings = configure_dora(&mut model, &runtime(), "experiment-owner".into()).unwrap();

    // Assert
    assert_eq!(json!({"model":model,"warnings":warnings}), expected);
}

#[test]
fn dora_wiring_rejects_reserved_mounts_labels_and_service_names() {
    for target in ["/", "/dev", "/dev/shm", "/run/experiment/zenoh.json5"] {
        // Arrange
        let mut model = json!({"services":{"app":{"volumes":[{"type":"bind","source":"/data","target":target}]}}});

        // Act
        let result = configure_dora(&mut model, &runtime(), "owner".into());

        // Assert
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("app mount overrides")
        );
    }
    for part in [
        json!({"app":{"labels":{"io.doramcap.experiment-owner":"true"}}}),
        json!({"app":{"labels":{"io.doramcap.controller":"foreign"}}}),
        json!({"dora-daemon":{}}),
        json!({"dora-coordinator":{}}),
    ] {
        // Arrange
        let mut model = json!({"services":part});

        // Act
        let result = configure_dora(&mut model, &runtime(), "owner".into());

        // Assert
        assert!(result.is_err());
    }
}

#[test]
fn graph_bindings_require_matching_services_and_host_networking() {
    // Arrange
    let graph = crate::workflows::shared_dora::graph(
        &crate::workflows::dora::DoraRuntime {
            graph: "/flow.yml".into(),
            ..runtime()
        },
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
        let result = configure_dora(&mut model, &runtime(), "owner".into());

        // Assert
        assert!(result.unwrap_err().to_string().contains("invalid Compose"));
    }
}

#[test]
fn dora_wiring_preserves_an_absent_network_section() {
    // Arrange
    let mut model = json!({"services":{"app":{"network_mode":"host"}}});
    // Act
    configure_dora(&mut model, &runtime(), "owner".into()).unwrap();
    // Assert: Compose requires an absent optional section to stay absent.
    assert!(model.get("networks").is_none());
}
