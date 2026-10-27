use orchestrator::{
    Error, LocalExecutor,
    process::{ProcessInput, ProcessStep},
};
use serde_json::json;
fn script(code: &str) -> ProcessInput {
    let mut input = ProcessInput::new("python3", json!({"value": 7}));
    input.args = vec!["-c".into(), code.into()];
    input
}
#[tokio::test(flavor = "current_thread")]
async fn protocol_and_diagnostics() {
    // Arrange
    let mut executor = LocalExecutor::default();
    executor.register::<ProcessStep>(|| ProcessStep);
    let input = script(
        "import json,sys; r=json.load(sys.stdin); print(json.dumps(r['input']['value']+3)); print(r['protocol'],file=sys.stderr)",
    );
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
    let failed = executor
        .run::<ProcessStep>(script(
            "import sys; print('bad',file=sys.stderr); sys.exit(7)",
        ))
        .await;
    // Assert
    assert!(matches!(failed, Err(Error::Process { code: Some(7), stderr }) if stderr == "bad\n"));
    // Act
    let invalid = executor
        .run::<ProcessStep>(script("print('invalid')"))
        .await;
    // Assert
    assert!(matches!(
        invalid,
        Err(Error::Serialization {
            boundary: "external stdout",
            ..
        })
    ));
    // Act
    let mut input = script("print('x'*100000)");
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
    let mut input = script("import time; time.sleep(30)");
    input.timeout_ms = 50;
    let timeout = executor.run::<ProcessStep>(input).await;
    // Assert
    assert!(matches!(timeout, Err(Error::Timeout)));
}
