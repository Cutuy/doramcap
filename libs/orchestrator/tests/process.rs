use orchestrator::{
    Error, LocalExecutor,
    process::{ProcessInput, ProcessStep},
};
use serde_json::json;
fn fixture(mode: &str) -> ProcessInput {
    let mut input = ProcessInput::new("node", json!({"value": 7}));
    input.args = vec![
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/process.ts").into(),
        mode.into(),
    ];
    input
}
#[tokio::test(flavor = "current_thread")]
async fn protocol_and_diagnostics() {
    // Arrange
    let mut executor = LocalExecutor::default();
    executor.register::<ProcessStep>(|| ProcessStep);
    let input = fixture("protocol");
    // Act
    let result = executor.run::<ProcessStep>(input).await.unwrap();
    // Assert
    assert_eq!(result.value().value, json!(10));
    assert_eq!(result.value().stderr, "2\n");
}
#[tokio::test(flavor = "current_thread")]
async fn errors_are_exposed_and_captures_bounded() {
    // Arrange
    let mut executor = LocalExecutor::default();
    executor.register::<ProcessStep>(|| ProcessStep);
    // Act
    let failed = executor.run::<ProcessStep>(fixture("failure")).await;
    // Assert
    assert!(matches!(failed, Err(Error::Process { code: Some(7), stderr }) if stderr == "bad\n"));
    // Act
    let invalid = executor.run::<ProcessStep>(fixture("invalid")).await;
    // Assert
    assert!(matches!(
        invalid,
        Err(Error::Serialization {
            boundary: "external stdout",
            ..
        })
    ));
    // Act
    let mut input = fixture("overflow");
    input.max_stdout_bytes = 32;
    let overflow = executor.run::<ProcessStep>(input).await;
    // Assert
    assert!(matches!(
        overflow,
        Err(Error::Limit {
            boundary: "stdout",
            ..
        })
    ));
    // Act
    let mut input = fixture("timeout");
    input.timeout_ms = 50;
    let timeout = executor.run::<ProcessStep>(input).await;
    // Assert
    assert!(matches!(timeout, Err(Error::Timeout)));
}
