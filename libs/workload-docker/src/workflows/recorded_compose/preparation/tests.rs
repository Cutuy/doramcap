use super::{validation::*, *};

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
