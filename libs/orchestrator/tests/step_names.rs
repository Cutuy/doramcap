use orchestrator::{Event, LocalExecutor, Observer, Step, telemetry::TelemetryObserver};
use std::{cell::RefCell, rc::Rc};

struct RenamedImplementation;
impl Step for RenamedImplementation {
    type Input = ();
    type Output = ();
    const ID: Option<&'static str> = Some("example.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare example");
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        Ok(())
    }
}
struct AnotherImplementation;
impl Step for AnotherImplementation {
    type Input = ();
    type Output = ();
    const ID: Option<&'static str> = RenamedImplementation::ID;
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        panic!("duplicate identity must fail before body")
    }
}
struct InvalidName;
impl Step for InvalidName {
    type Input = ();
    type Output = ();
    const ID: Option<&'static str> = Some("");
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        panic!("invalid identity must fail before body")
    }
}

#[tokio::test(flavor = "current_thread")]
async fn invocation_ids_are_distinct_while_step_identity_and_label_stay_stable() {
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
    executor.run::<RenamedImplementation>(()).await.unwrap();
    // Assert
    let events: Vec<_> = packets
        .borrow()
        .iter()
        .map(|packet| workflow_protocol::decode(packet).unwrap())
        .collect();
    assert_eq!(
        events
            .iter()
            .map(|event| event.invocation)
            .collect::<Vec<_>>(),
        [1, 1, 2, 2]
    );
    for event in events {
        assert_eq!(event.step, "example.prepare");
        assert_eq!(event.display_name.as_deref(), Some("Prepare example"));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn duplicate_or_empty_step_names_fail_before_factory_or_observation() {
    for invalid in [false, true] {
        // Arrange
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let mut executor =
            LocalExecutor::new(move |event: &Event| sink.borrow_mut().push(event.clone()));
        executor.register::<RenamedImplementation>(|| {
            panic!("invalid registry must fail before factory")
        });
        if invalid {
            executor.register::<InvalidName>(|| InvalidName);
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
fn invocation_json_without_a_label_remains_readable() {
    // Arrange
    let input = r#"{"session":"run","id":1,"step":"legacy::Step","version":1,"executor":"local","parent":null,"dependencies":[]}"#;
    // Act
    let invocation: orchestrator::Invocation = serde_json::from_str(input).unwrap();
    // Assert
    assert!(invocation.display_name.is_none());
    assert_eq!(invocation.step, "legacy::Step");
}
