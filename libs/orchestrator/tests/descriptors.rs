use orchestrator::{
    Event, LocalExecutor, Observer, ProgressKind, ProgressRetention, Step, StepDescriptor,
    telemetry::TelemetryObserver,
};
use std::{cell::RefCell, rc::Rc};

struct RenamedImplementation;
impl Step for RenamedImplementation {
    type Input = ();
    type Output = ();
    fn descriptor() -> Option<StepDescriptor> {
        Some(StepDescriptor::new(
            "example.prepare",
            "Prepare example",
            ProgressRetention::Keep,
            ProgressKind::Operation,
        ))
    }
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        Ok(())
    }
}
struct AnotherImplementation;
impl Step for AnotherImplementation {
    type Input = ();
    type Output = ();
    fn descriptor() -> Option<StepDescriptor> {
        RenamedImplementation::descriptor()
    }
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        panic!("duplicate metadata must fail before body")
    }
}
struct InvalidMetadata;
impl Step for InvalidMetadata {
    type Input = ();
    type Output = ();
    fn descriptor() -> Option<StepDescriptor> {
        Some(StepDescriptor::new(
            "",
            "Missing ID",
            ProgressRetention::Keep,
            ProgressKind::Operation,
        ))
    }
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        panic!("invalid metadata must fail before body")
    }
}

#[tokio::test(flavor = "current_thread")]
async fn stable_metadata_survives_protobuf_independently_of_rust_type_names() {
    // Arrange
    let packets = Rc::new(RefCell::new(Vec::new()));
    let sink = packets.clone();
    let observer = TelemetryObserver::new(4096, move |bytes| {
        sink.borrow_mut().push(bytes.to_vec());
        Ok(())
    });
    let mut executor = LocalExecutor::new(move |event: &Event| observer.observe(event));
    executor.register::<RenamedImplementation>(|| RenamedImplementation);
    // Act
    executor.run::<RenamedImplementation>(()).await.unwrap();
    // Assert
    for packet in packets.borrow().iter() {
        let event = workflow_protocol::decode(packet).unwrap();
        assert!(event.step.ends_with("RenamedImplementation"));
        let descriptor = event.descriptor.unwrap();
        assert_eq!(descriptor.id, "example.prepare");
        assert_eq!(descriptor.display_name, "Prepare example");
        assert_eq!(descriptor.retention, ProgressRetention::Keep as i32);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn duplicate_or_incomplete_descriptors_fail_before_factory_or_observation() {
    for incomplete in [false, true] {
        // Arrange
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let mut executor =
            LocalExecutor::new(move |event: &Event| sink.borrow_mut().push(event.clone()));
        executor.register::<RenamedImplementation>(|| {
            panic!("invalid registry must fail before factory")
        });
        if incomplete {
            executor.register::<InvalidMetadata>(|| InvalidMetadata);
        } else {
            executor.register::<AnotherImplementation>(|| AnotherImplementation);
        }
        // Act
        let result = executor.run::<RenamedImplementation>(()).await;
        // Assert
        assert!(matches!(result, Err(orchestrator::Error::Configuration(_))));
        assert!(events.borrow().is_empty());
    }
}

#[test]
fn legacy_invocation_json_has_optional_metadata() {
    // Arrange
    let legacy = r#"{"session":"run","id":1,"step":"legacy::Step","version":1,"executor":"local","parent":null,"dependencies":[]}"#;
    // Act
    let invocation: orchestrator::Invocation = serde_json::from_str(legacy).unwrap();
    // Assert
    assert!(invocation.descriptor.is_none());
    assert_eq!(invocation.step, "legacy::Step");
}
