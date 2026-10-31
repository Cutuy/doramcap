//! Shared Compose messages across protobuf transports and local step execution.
use orchestrator::{LocalExecutor, Step};
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use workload_docker::compose::*;

struct InspectFixture;
impl Step for InspectFixture {
    type Input = StartInput;
    type Output = ComposeProjectState;
    async fn run(&mut self, input: StartInput) -> orchestrator::Result<Self::Output> {
        Ok(ComposeProjectState {
            plan: input.plan,
            containers: vec![Container {
                id: "container".into(),
                service: "worker".into(),
                image_id: "sha256:image".into(),
                status: "exited".into(),
                health: Some(String::new()),
                exit_code: Some(0),
                ..Default::default()
            }],
        })
    }
}

#[tokio::test]
async fn protobuf_contract_survives_text_binary_and_local_step_boundaries() {
    // Arrange: fixed-width counts and present-zero status must survive each boundary.
    let pool = DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/workflow_config.bin")).as_slice(),
    )
    .unwrap();
    let request: StartInput = DynamicMessage::parse_text_format(
        pool.get_message_by_name("doramcap.compose.v1.StartInput")
            .unwrap(),
        r#"services: "worker" readiness_seconds: 15 plan {
            run: "run" key: "app" name: "dc-run-app"
            manifest: "/tmp/space in path/compose.json" sha256: "digest"
            services { key: "worker" value { image_id: "sha256:image"
                replicas: 18446744073709551615 completed: true } }
        }"#,
    )
    .unwrap()
    .transcode_to()
    .unwrap();
    let mut executor = LocalExecutor::default();
    executor.register(|| InspectFixture);

    // Act: use protobuf transport and the framework's JSON step boundary.
    let request = StartInput::decode(request.encode_to_vec().as_slice()).unwrap();
    let output = executor
        .run::<InspectFixture>(request)
        .await
        .unwrap()
        .into_value();
    let output = ComposeProjectState::decode(output.encode_to_vec().as_slice()).unwrap();

    // Assert
    let plan = output.plan.unwrap();
    assert_eq!(plan.services["worker"].replicas, u64::MAX);
    assert!(plan.services["worker"].completed);
    assert_eq!(plan.manifest, "/tmp/space in path/compose.json");
    assert_eq!(output.containers[0].exit_code, Some(0));
    assert_eq!(output.containers[0].health.as_deref(), Some(""));
}

#[test]
fn generated_launch_config_preserves_defaults_and_rejects_unknown_fields() {
    // Arrange: existing serialized configuration may omit optional launch settings.
    let input = serde_json::json!({"key":"app", "directory":"/tmp", "files":["compose.yml"]});

    // Act
    let project: ComposeProject = serde_json::from_value(input.clone()).unwrap();
    let mut unknown = input;
    unknown["typo"] = true.into();

    // Assert
    assert!(project.environment.is_empty());
    assert!(project.env_files.is_empty());
    assert!(!project.remove_volumes);
    assert!(serde_json::from_value::<ComposeProject>(unknown).is_err());
    assert!(ComposeProjectState::default().ready().is_err());
}

#[test]
fn volume_deletion_permission_defaults_off_across_serialization_formats() {
    // Arrange
    let pool = DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/workflow_config.bin")).as_slice(),
    )
    .unwrap();
    let descriptor = pool
        .get_message_by_name("doramcap.compose.v1.StopInput")
        .unwrap();
    for permission in [false, true] {
        let text = if permission {
            "allow_volume_deletion: true"
        } else {
            ""
        };
        let json = if permission {
            serde_json::json!({"allow_volume_deletion": true, "stop_seconds": 0})
        } else {
            serde_json::json!({"stop_seconds": 0})
        };

        // Act
        let protobuf: StopInput = DynamicMessage::parse_text_format(descriptor.clone(), text)
            .unwrap()
            .transcode_to()
            .unwrap();
        let protobuf = StopInput::decode(protobuf.encode_to_vec().as_slice()).unwrap();
        let serialized: StopInput = serde_json::from_value(json).unwrap();

        // Assert
        assert_eq!(protobuf.allow_volume_deletion, permission);
        assert_eq!(serialized.allow_volume_deletion, permission);
    }
}
