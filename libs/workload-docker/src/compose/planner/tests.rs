use super::*;
use crate::compose::{
    ComposeProject,
    model::{ComposeModel, ComposeServiceDefinition},
};
use serde_json::json;

fn input(document: &str) -> AnalyzeInput {
    AnalyzeInput {
        run: "run".into(),
        project_name: "dc-project".into(),
        artifacts: "/runs/run".into(),
        adopted_apps: vec![ComposeDocument {
            settings: Some(ComposeProject {
                key: "app".into(),
                ..Default::default()
            }),
            compose_json: document.as_bytes().to_vec(),
        }],
        ..Default::default()
    }
}
#[tokio::test]
async fn embedded_planner_returns_typed_services_and_lossless_docker_json() {
    // Arrange
    let input = input(
        r#"{"services":{"app":{"image":"local:app","environment":{"UNSET":null,"EMPTY":"","UNICODE":"机器人 🚀 café"},"entrypoint":null,"command":[],"shm_size":9007199254740993,"ulimits":{"memlock":-1},"x-future":{"integer":9007199254740993,"decimal":1.234567890123456789}}}}"#,
    );
    // Act
    let required = analyze(&input).await.unwrap();
    let output = complete(CompleteInput {
        input: Some(input),
        images: vec![ImageFact {
            reference: "local:app".into(),
            image_id: "sha256:pinned".into(),
            declared_volumes: vec![],
        }],
        ..Default::default()
    })
    .await
    .unwrap();
    let model = output.compose.unwrap();
    let rendered = String::from_utf8(render(&model).await.unwrap()).unwrap();
    // Assert
    assert_eq!(required.image_references, ["local:app"]);
    assert_eq!(output.services["app"].image_id, "sha256:pinned");
    assert_eq!(model.services["app"].shm_size, Some(9007199254740993));
    assert!(rendered.contains("9007199254740993"));
    assert!(rendered.contains("1.234567890123456789"));
    let document: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(document["services"]["app"]["image"], "sha256:pinned");
    assert_eq!(
        document["services"]["app"]["environment"],
        json!({"UNSET":null,"EMPTY":"","UNICODE":"机器人 🚀 café"})
    );
    assert_eq!(document["services"]["app"]["command"], json!([]));
    assert!(document["services"]["app"]["entrypoint"].is_null());
    assert_eq!(document["services"]["app"]["ulimits"]["memlock"], -1);
}
#[tokio::test]
async fn embedded_errors_are_reported_and_the_next_call_is_isolated() {
    // Arrange
    let invalid = input(r#"{"services":{"app":{}}}"#);
    // Act / Assert
    assert!(
        analyze(&invalid)
            .await
            .unwrap_err()
            .to_string()
            .contains("named local image")
    );
    // Act / Assert: an exception leaves no runtime state for the following call.
    assert_eq!(
        analyze(&input(r#"{"services":{"app":{"image":"valid"}}}"#))
            .await
            .unwrap()
            .image_references,
        ["valid"]
    );
}
#[tokio::test]
async fn docker_export_preserves_omitted_optional_sections() {
    // Arrange
    let model = ComposeModel {
        services: std::collections::BTreeMap::from([(
            "app".into(),
            ComposeServiceDefinition {
                image: "pinned".into(),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    // Act
    let rendered: serde_json::Value =
        serde_json::from_slice(&render(&model).await.unwrap()).unwrap();
    // Assert
    assert_eq!(rendered, json!({"services":{"app":{"image":"pinned"}}}));
}

#[test]
fn execution_deadline_cancellation_and_memory_limit_stop_the_engine() {
    for cancelled in [false, true] {
        // Arrange
        let signal = Arc::new(AtomicBool::new(cancelled));
        let start = Instant::now();
        // Act
        let result = evaluate(
            "analyze",
            vec![],
            signal,
            Duration::from_millis(10),
            "globalThis.doramcapPlanner={analyze(){while(true){}}}",
        );
        // Assert
        assert!(result.is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    // Arrange
    let signal = Arc::new(AtomicBool::new(false));
    // Act
    let result = evaluate(
        "analyze",
        vec![],
        signal,
        Duration::from_secs(1),
        "globalThis.doramcapPlanner={analyze(){return new Uint8Array(512*1024*1024)}}",
    );
    // Assert
    assert!(result.is_err());
}
