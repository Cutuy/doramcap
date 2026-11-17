use super::*;
use crate::{BuildAssessment, BuildRecord};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn recorded_image_presence_absence_mismatch_and_errors_remain_distinct() {
    // Arrange: exercise the shared observation with supplied facts; Console loads them from its store.
    for (status, observed, expected, build_state) in [
        (
            200,
            "sha256:built",
            RecordedImageState::Present,
            BuildState::Fresh,
        ),
        (
            200,
            "sha256:other",
            RecordedImageState::Mismatch,
            BuildState::RebuildNeeded,
        ),
        (404, "", RecordedImageState::Missing, BuildState::NotBuilt),
        (500, "", RecordedImageState::Failed, BuildState::Fresh),
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
        let mut result = Preparation {
            build: Some(BuildAssessment {
                state: BuildState::Fresh as i32,
                ..Default::default()
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
        assert_eq!(result.build.unwrap().state, build_state as i32);
        assert_eq!(image.unwrap(), observed);
        assert_eq!(!result.rejections.is_empty(), status == 500);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
