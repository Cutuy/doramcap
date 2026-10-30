use orchestrator::{
    Event, Invocation, LocalExecutor, Observer, State, telemetry::TelemetryObserver,
};
use std::{cell::RefCell, rc::Rc};
use workflow_protocol as workflow;
fn event(state: State) -> Event {
    Event {
        invocation: Invocation {
            session: "run".into(),
            id: 9_007_199_254_740_995,
            step: "Audit".into(),
            display_name: None,
            version: 2,
            executor: "local".into(),
            parent: Some(7),
            dependencies: vec![8],
        },
        state,
        elapsed_ns: 20,
        duration_ns: 10,
        payload: Some(serde_json::json!({"path":"/capture"})),
        error: Some("audit failed".into()),
    }
}
#[test]
fn protobuf_preserves_metadata_and_terminal_states() {
    // Arrange
    let packets = Rc::new(RefCell::new(Vec::new()));
    let sink = packets.clone();
    let observer = TelemetryObserver::new(4096, move |bytes| {
        sink.borrow_mut().push(bytes.to_vec());
        Ok(())
    });
    // Act
    for state in [
        State::Started,
        State::Succeeded,
        State::Failed,
        State::Cancelled,
        State::Panicked,
    ] {
        observer.observe(&event(state));
    }
    // Assert
    for (index, bytes) in packets.borrow().iter().enumerate() {
        let decoded = workflow::decode(bytes).unwrap();
        assert_eq!(decoded.sequence, index as u64 + 1);
        assert_eq!(decoded.state, index as i32 + 1);
        assert_eq!(decoded.invocation, 9_007_199_254_740_995);
        assert_eq!(decoded.parent, Some(7));
        assert_eq!(decoded.dependencies, vec![8]);
        assert_eq!(
            decoded.payload_json.as_deref(),
            Some("{\"path\":\"/capture\"}")
        );
    }
    assert_eq!(observer.dropped_events(), 0);
}
#[test]
fn admission_and_sink_failures_count_loss_without_retry() {
    // Arrange
    let observer = TelemetryObserver::new(1, |_| panic!("oversized event reached sink"));
    // Act
    observer.observe(&event(State::Started));
    // Assert
    assert_eq!(observer.dropped_events(), 1);
    // Arrange
    let observer = TelemetryObserver::new(4096, |_| Err(std::io::Error::other("queue full")));
    // Act
    observer.observe(&event(State::Started));
    // Assert
    assert_eq!(observer.dropped_events(), 1);
}
#[derive(Default)]
struct Noop;
impl orchestrator::Step for Noop {
    type Input = ();
    type Output = ();
    async fn run(&mut self, _: ()) -> orchestrator::Result<()> {
        Ok(())
    }
}
#[tokio::test(flavor = "current_thread")]
async fn panicking_sink_counts_loss_and_executor_isolates_it() {
    // Arrange
    let observer = TelemetryObserver::new(4096, |_| panic!("sink panic"));
    let mut executor = LocalExecutor::new(observer.clone());
    executor.register::<Noop>(|| Noop);
    // Act
    let result = executor.run::<Noop>(()).await;
    // Assert
    assert!(result.is_ok());
    assert_eq!(observer.dropped_events(), 2);
    assert_eq!(executor.observer_failures(), 2);
}
#[test]
fn delivery_failure_leaves_a_sequence_gap() {
    // Arrange
    let packets = Rc::new(RefCell::new(Vec::new()));
    let sink = packets.clone();
    let calls = std::cell::Cell::new(0);
    let observer = TelemetryObserver::new(4096, move |bytes| {
        calls.set(calls.get() + 1);
        if calls.get() == 1 {
            return Err(std::io::Error::other("full"));
        }
        sink.borrow_mut().push(bytes.to_vec());
        Ok(())
    });
    // Act
    observer.observe(&event(State::Started));
    observer.observe(&event(State::Succeeded));
    // Assert
    assert_eq!(workflow::decode(&packets.borrow()[0]).unwrap().sequence, 2);
    assert_eq!(observer.dropped_events(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn distinct_executor_labels_namespace_invocations_in_a_shared_stream() {
    // Arrange
    let packets = Rc::new(RefCell::new(Vec::new()));
    let sink = packets.clone();
    let observer = TelemetryObserver::new(4096, move |bytes| {
        sink.borrow_mut().push(bytes.to_vec());
        Ok(())
    });
    let mut first = LocalExecutor::new(observer.clone()).with_label("first-run");
    let mut second = LocalExecutor::new(observer.clone()).with_label("second-run");
    first.register::<Noop>(|| Noop);
    second.register::<Noop>(|| Noop);

    // Act
    first.run::<Noop>(()).await.unwrap();
    second.run::<Noop>(()).await.unwrap();

    // Assert
    let decoded: Vec<_> = packets
        .borrow()
        .iter()
        .map(|bytes| workflow::decode(bytes).unwrap())
        .collect();
    assert_eq!(decoded.len(), 4);
    assert_eq!(
        decoded
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let (invocations, _) = decoded.as_chunks::<2>();
    for (events, session) in invocations.iter().zip(["first-run", "second-run"]) {
        assert!(events.iter().all(|event| {
            event.session == session && event.executor == "local" && event.invocation == 1
        }));
        assert_eq!(events[0].state, workflow::event::State::Started as i32);
        assert_eq!(events[1].state, workflow::event::State::Succeeded as i32);
    }
    assert_eq!(observer.dropped_events(), 0);
    assert_eq!(first.observer_failures(), 0);
    assert_eq!(second.observer_failures(), 0);
}
