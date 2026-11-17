use super::*;
use crate::{BuildAssessment, BuildRecord};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn recording_review_requires_actual_system_inputs() {
    // Arrange
    let mut run = RunPlan {
        repository_root: "/repository".into(),
        artifacts: "/records/attempt".into(),
        readiness_seconds: 30,
        stop_seconds: 10,
        host_shm_access: HostShmAccess::Passthrough as i32,
        ..Default::default()
    };
    run.recordings.push(Default::default());
    let selection = Selection {
        application: "native unrestricted / application".into(),
        ..Default::default()
    };
    // Act / Assert: app-only recordings retain their preparation rejection.
    assert!(
        validate(&run, "/cli", &selection, &[])
            .unwrap_err()
            .contains("configured system applications")
    );
    run.system_apps.push("monitor".into());
    let project = compose_apps::compose::ComposeProject {
        key: "monitor".into(),
        ..Default::default()
    };
    // Act / Assert: actual validated system inputs carry the recording to shared resolution.
    let input = validate(&run, "/cli", &selection, &[project]).unwrap();
    assert_eq!(input.run_plan.recordings, run.recordings);
    assert_eq!(input.system_apps.len(), 1);
}

#[test]
fn existing_json_plans_default_to_no_system_projects() {
    // Arrange / Act: the new repeated protobuf field preserves the existing JSON caller shape.
    let plan: crate::HoloscanPlan = serde_json::from_value(serde_json::json!({
        "cli": "/cli", "selection": null, "extra_data_folders": []
    }))
    .unwrap();
    // Assert
    assert!(plan.system_projects.is_empty());
}

#[tokio::test]
#[ignore = "requires local Docker Engine and installed test image; launches no containers"]
async fn system_review_pins_configuration_and_reuses_one_artifact_resolution() {
    use compose_apps::{compose::ComposeProject, preparation::prepare_resolved};
    use std::path::PathBuf;

    // Arrange: simulated native query envelopes beside actual Compose resolution and Engine reads.
    let root = std::env::temp_dir().join(format!("holoscan-system-review-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("app")).unwrap();
    std::fs::write(root.join("app/metadata.json"), br#"{"application":{}}"#).unwrap();
    std::fs::write(root.join("response.sh"), r#"
if [ "$3" = list ]; then
    printf '{"schema_version":1,"projects":[{"name":"-demo","project_type":"application","source_folder":"%s/app","language":["Python"],"modes":["-headless"]}]}' "$PWD"
elif [ "$3" = modes ]; then
    [ "$4" = --language ] && [ "$5" = python ] && [ "$6" = --json ] && [ "$7" = -- ] && [ "$8" = -demo ] || exit 16
    printf '{"schema_version":1,"project":"-demo","language":["Python"],"modes":{"-headless":{"run":{"command":"native command"}}}}'
elif [ "$3" = run ]; then
    [ "$HOLOSCAN_CLI_BUILD_LOCAL" = 0 ] || exit 17
    [ "${10}" = -- ] && [ "${11}" = -demo ] && [ "${12}" = -headless ] || exit 19
    printf 'native diagnostic only\n'
else exit 18
fi
"#).unwrap();
    let cli = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli.sh");
    let image = std::env::var("DORAMCAP_TEST_IMAGE").expect("supply the installed test image");
    let socket =
        std::env::var("DORAMCAP_TEST_SOCKET").unwrap_or_else(|_| "/var/run/docker.sock".into());
    let engine = Engine::new(&socket, 30).unwrap();
    let image_id = engine.image_id(&image).await.unwrap().unwrap();
    let _admission = workload_lifecycle::admission::Admission::acquire(&root).unwrap();
    let selection = Selection {
        application: "-demo".into(),
        language: "python".into(),
        mode: "-headless".into(),
    };
    let mut snapshots = Vec::new();
    for (attempt, value) in ["same", "same", "changed"].into_iter().enumerate() {
        let manifest = root.join("compose.json");
        std::fs::write(&manifest, serde_json::to_vec(&serde_json::json!({"services":{"monitor":{"image":image,"environment":{"REVIEW_SETTING":value}}}})).unwrap()).unwrap();
        let artifacts = root.join(format!("attempt-{attempt}"));
        let run = RunPlan {
            id: format!("review-{attempt}"),
            repository_root: root.to_str().unwrap().into(),
            artifacts: artifacts.to_str().unwrap().into(),
            system_apps: vec!["monitor".into()],
            host_shm_access: HostShmAccess::Passthrough as i32,
            readiness_seconds: 30,
            stop_seconds: 10,
            ..Default::default()
        };
        let request = PrepareRequest {
            plan: Some(crate::HoloscanPlan {
                cli: cli.to_str().unwrap().into(),
                selection: Some(selection.clone()),
                system_projects: vec![ComposeProject {
                    key: "monitor".into(),
                    directory: root.to_str().unwrap().into(),
                    files: vec![manifest.to_str().unwrap().into()],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            build_record: Some(BuildRecord {
                selection: Some(selection.clone()),
                image_id: image_id.clone(),
                ..Default::default()
            }),
            ..Default::default()
        };
        // Act
        let (review, resolved) = PrepareLaunch::new(engine.clone())
            .run((run, request))
            .await
            .unwrap();
        // Assert: system facts and native evidence coexist in one authoritative snapshot.
        assert!(review.rejections.is_empty(), "{:?}", review.rejections);
        let resolved = resolved.unwrap();
        let snapshot = review.snapshot.unwrap();
        assert_eq!(
            snapshot.system_configuration.as_ref(),
            Some(&resolved.snapshot)
        );
        assert_eq!(resolved.artifacts, artifacts);
        assert!(!artifacts.join("filesystem").exists());
        // Act / Assert: reuse the actual resolution; no second artifact or source baseline is created.
        let prepared = prepare_resolved(resolved).await.unwrap();
        assert_eq!(prepared.artifacts, artifacts);
        assert_eq!(prepared.source_input.artifacts, artifacts);
        assert!(!artifacts.join("filesystem").exists());
        snapshots.push(snapshot);
    }
    // Assert: generated attempt identity is normalized while changed system input changes Review.
    assert_eq!(
        snapshots[0].system_configuration,
        snapshots[1].system_configuration
    );
    assert_eq!(snapshots[0].sha256, snapshots[1].sha256);
    assert_ne!(
        snapshots[1].system_configuration,
        snapshots[2].system_configuration
    );
    assert_ne!(snapshots[1].sha256, snapshots[2].sha256);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn recorded_image_presence_absence_mismatch_and_errors_remain_distinct() {
    // Arrange: exercise the shared observation with supplied facts; Console loads them from its store.
    for (status, observed, expected, build_state, initial_state) in [
        (
            200,
            "sha256:built",
            RecordedImageState::Present,
            BuildState::Fresh,
            BuildState::Fresh,
        ),
        (
            200,
            "sha256:other",
            RecordedImageState::Mismatch,
            BuildState::RebuildNeeded,
            BuildState::Fresh,
        ),
        (
            404,
            "",
            RecordedImageState::Missing,
            BuildState::NotBuilt,
            BuildState::Fresh,
        ),
        (
            500,
            "",
            RecordedImageState::Failed,
            BuildState::Fresh,
            BuildState::Fresh,
        ),
        (
            200,
            "sha256:other",
            RecordedImageState::Mismatch,
            BuildState::RebuildNeeded,
            BuildState::Unverified,
        ),
        (
            404,
            "",
            RecordedImageState::Missing,
            BuildState::NotBuilt,
            BuildState::Unverified,
        ),
        (
            500,
            "",
            RecordedImageState::Failed,
            BuildState::Unverified,
            BuildState::Unverified,
        ),
    ] {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("holoscan-image-{}-{unique}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let socket = directory.join("docker.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let engine = Engine::new(socket.to_str().unwrap(), 1).unwrap();
        let mut prepare = PrepareLaunch::new(engine);
        let record = Some(BuildRecord {
            image_id: "sha256:built".into(),
            ..Default::default()
        });
        let informational = if initial_state == BuildState::Fresh {
            "Known clean source facts match Build"
        } else {
            "Current source state is Unknown; freshness is unverified"
        };
        let mut result = Preparation {
            build: Some(BuildAssessment {
                state: initial_state as i32,
                diagnostics: vec![informational.into()],
            }),
            ..Default::default()
        };
        let server = async {
            let (mut connection, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 1024];
            while !bytes.ends_with(b"\r\n\r\n") {
                let count = connection.read(&mut chunk).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&chunk[..count]);
            }
            assert!(
                String::from_utf8(bytes)
                    .unwrap()
                    .contains("/images/sha256:built/json")
            );
            let body = if status == 200 {
                serde_json::json!({"Id":observed})
            } else {
                serde_json::json!({"message":"image observation failed"})
            }
            .to_string();
            let response = format!(
                "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            connection.write_all(response.as_bytes()).await.unwrap();
        };
        // Act
        let (image, ()) = tokio::join!(prepare.observe_image(&record, &mut result), server);
        // Assert
        assert_eq!(
            result.recorded_image.as_ref().unwrap().state,
            expected as i32
        );
        let build = result.build.as_ref().unwrap();
        assert_eq!(build.state, build_state as i32);
        assert!(
            build
                .diagnostics
                .iter()
                .any(|message| message == informational)
        );
        assert!(
            !result
                .rejections
                .iter()
                .any(|message| message == informational)
        );
        assert_eq!(image.unwrap(), observed);
        assert_eq!(
            result.rejections.len(),
            usize::from(expected != RecordedImageState::Present)
        );
        if status == 500 {
            assert!(result.rejections[0].contains("observation failed"));
            assert!(!result.recorded_image.as_ref().unwrap().error.is_empty());
        } else if expected != RecordedImageState::Present {
            assert!(result.rejections[0].starts_with("Recorded Build image"));
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
