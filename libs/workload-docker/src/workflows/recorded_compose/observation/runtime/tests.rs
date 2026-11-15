use super::*;
use compose_apps::compose::ComposeService;
use prost::Message;

fn fixture() -> (RuntimeTracker, watch::Receiver<Option<RuntimeObservation>>) {
    let (sender, receiver) = watch::channel(None);
    let plan = ResolvedComposeProject {
        name: "experiment".into(),
        run: "current-run".into(),
        services: BTreeMap::from([(
            "worker".into(),
            ComposeService {
                replicas: 2,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    (RuntimeTracker::new(plan, sender), receiver)
}
fn event(action: &str, id: &str, replica: &str, code: Option<&str>) -> EventMessage {
    let mut attributes = std::collections::HashMap::from([
        (PROJECT_LABEL.into(), "experiment".into()),
        (RUN_LABEL.into(), "current-run".into()),
        (SERVICE_LABEL.into(), "worker".into()),
        (REPLICA_LABEL.into(), replica.into()),
    ]);
    if let Some(code) = code {
        attributes.insert("exitCode".into(), code.into());
    }
    EventMessage {
        typ: Some(EventMessageTypeEnum::CONTAINER),
        action: Some(action.into()),
        actor: Some(bollard::models::EventActor {
            id: Some(id.into()),
            attributes: Some(attributes),
        }),
        ..Default::default()
    }
}
fn observation(receiver: &watch::Receiver<Option<RuntimeObservation>>) -> RuntimeObservation {
    RuntimeObservation::decode(
        receiver
            .borrow()
            .as_ref()
            .unwrap()
            .encode_to_vec()
            .as_slice(),
    )
    .unwrap()
}

#[test]
fn owned_starts_and_exits_are_available_before_run_completion() {
    // Arrange
    let (mut tracker, receiver) = fixture();
    assert!(observation(&receiver).containers.is_empty());
    // Act
    tracker.observe(&event("start", "first", "1", None));
    // Assert
    let running = observation(&receiver);
    assert!(running.error.is_empty());
    assert_eq!(running.containers[0].service, "worker");
    assert_eq!(running.containers[0].id, "first");
    assert_eq!(running.containers[0].state, Some(State::Running(true)));
    // Act
    tracker.observe(&event("die", "first", "1", Some("0")));
    tracker.observe(&event("stop", "first", "1", None));
    tracker.observe(&event("destroy", "first", "1", None));
    // Assert: removal keeps the known zero exit status for display.
    let exited = observation(&receiver);
    assert_eq!(exited.containers[0].state, Some(State::ExitCode(0)));
    assert!(exited.error.is_empty());
}

#[test]
fn foreign_ownership_and_exec_activity_cannot_supply_runtime_facts() {
    // Arrange
    let (mut tracker, receiver) = fixture();
    let mut foreign_project = event("start", "other-project", "1", None);
    foreign_project
        .actor
        .as_mut()
        .unwrap()
        .attributes
        .as_mut()
        .unwrap()
        .insert(PROJECT_LABEL.into(), "another-project".into());
    let mut foreign_run = event("start", "old-run", "1", None);
    foreign_run
        .actor
        .as_mut()
        .unwrap()
        .attributes
        .as_mut()
        .unwrap()
        .insert(RUN_LABEL.into(), "previous-run".into());
    let mut unowned = event("start", "unowned", "1", None);
    unowned
        .actor
        .as_mut()
        .unwrap()
        .attributes
        .as_mut()
        .unwrap()
        .remove(RUN_LABEL);
    // Act
    for event in [
        foreign_project,
        foreign_run,
        unowned,
        event("exec_start", "exec", "1", None),
    ] {
        tracker.observe(&event);
    }
    // Assert
    let observed = observation(&receiver);
    assert!(observed.containers.is_empty());
    assert!(observed.error.is_empty());
}

#[test]
fn owned_network_and_volume_events_do_not_mark_container_observation_incomplete() {
    // Arrange: Engine network/volume events carry project ownership, but no service slot.
    let (mut tracker, receiver) = fixture();
    let events = [
        r#"{"Type":"network","Action":"destroy","Actor":{"ID":"network","Attributes":{"com.docker.compose.project":"experiment","io.doramcap.run":"current-run"}}}"#,
        r#"{"Type":"volume","Action":"create","Actor":{"ID":"volume","Attributes":{"com.docker.compose.project":"experiment","io.doramcap.run":"current-run"}}}"#,
    ];
    // Act
    for json in events {
        tracker.observe(&serde_json::from_str(json).unwrap());
    }
    // Assert
    let observed = observation(&receiver);
    assert!(observed.containers.is_empty());
    assert!(observed.error.is_empty());
}

#[test]
fn malformed_exit_is_unknown_and_cannot_become_a_clean_exit() {
    for code in [None, Some("bad"), Some("2147483648"), Some("-1")] {
        // Arrange
        let (mut tracker, receiver) = fixture();
        tracker.observe(&event("start", "first", "1", None));
        // Act
        tracker.observe(&event("die", "first", "1", code));
        // Assert
        let observed = observation(&receiver);
        assert_eq!(observed.containers[0].state, None);
        assert!(observed.error.contains("exit code"));
        // Act: later valid facts survive, while the incomplete run stays labeled.
        tracker.observe(&event("start", "first", "1", None));
        // Assert
        let repaired = observation(&receiver);
        assert_eq!(repaired.containers[0].state, Some(State::Running(true)));
        assert!(!repaired.error.is_empty());
    }
}

#[test]
fn replica_slots_bound_replacements_and_keep_independent_process_states() {
    // Arrange
    let (mut tracker, receiver) = fixture();
    tracker.observe(&event("start", "replica-one", "1", None));
    tracker.observe(&event("start", "replica-two", "2", None));
    // Act
    tracker.observe(&event("die", "replica-one", "1", Some("17")));
    // Assert
    let observed = observation(&receiver);
    assert_eq!(observed.containers.len(), 2);
    assert_eq!(observed.containers[0].state, Some(State::ExitCode(17)));
    assert_eq!(observed.containers[1].state, Some(State::Running(true)));
    // Act: a replacement owns the first slot; old removal and exit arrive later.
    tracker.observe(&event("create", "replacement", "1", None));
    tracker.observe(&event("start", "replacement", "1", None));
    tracker.observe(&event("destroy", "replica-one", "1", None));
    tracker.observe(&event("die", "replica-one", "1", Some("9")));
    // Assert
    let replacement = observation(&receiver);
    assert_eq!(replacement.containers.len(), 2);
    assert_eq!(replacement.containers[0].id, "replacement");
    assert_eq!(replacement.containers[0].state, Some(State::Running(true)));
    assert_eq!(replacement.containers[1].id, "replica-two");
    assert!(replacement.error.is_empty());
}

#[test]
fn invalid_owned_membership_is_partial_without_growing_the_frozen_slots() {
    for replica in ["", "0", "3", "invalid"] {
        // Arrange
        let (mut tracker, receiver) = fixture();
        tracker.observe(&event("start", "first", "1", None));
        // Act
        tracker.observe(&event("start", "extra", replica, None));
        // Assert
        let observed = observation(&receiver);
        assert_eq!(observed.containers.len(), 1);
        assert!(observed.error.contains("replica"));
    }
    // Arrange
    let (mut tracker, receiver) = fixture();
    let mut unknown = event("start", "extra", "1", None);
    unknown
        .actor
        .as_mut()
        .unwrap()
        .attributes
        .as_mut()
        .unwrap()
        .insert(SERVICE_LABEL.into(), "unknown".into());
    // Act
    tracker.observe(&unknown);
    // Assert
    let observed = observation(&receiver);
    assert!(observed.containers.is_empty());
    assert!(observed.error.contains("service"));
}

#[test]
fn source_failure_preserves_exit_evidence_and_lost_exit_is_unknown() {
    // Arrange
    let (mut tracker, receiver) = fixture();
    tracker.observe(&event("die", "first", "1", Some("137")));
    // Act
    tracker.fail("Docker event capture failed");
    // Assert
    let failed = observation(&receiver);
    assert_eq!(failed.containers[0].state, Some(State::ExitCode(137)));
    assert_eq!(failed.error, "Docker event capture failed");
    // Arrange
    let (mut tracker, receiver) = fixture();
    tracker.observe(&event("start", "first", "1", None));
    // Act
    tracker.observe(&event("destroy", "first", "1", None));
    // Assert
    let lost = observation(&receiver);
    assert_eq!(lost.containers[0].state, None);
    assert!(!lost.error.is_empty());
}

#[test]
fn docker_stop_before_die_waits_for_exit_evidence_without_losing_completeness() {
    // Arrange: this ordering was captured from the real stop-grace Docker scenario.
    let (mut tracker, receiver) = fixture();
    tracker.observe(&event("create", "first", "1", None));
    tracker.observe(&event("start", "first", "1", None));
    // Act
    tracker.observe(&event("stop", "first", "1", None));
    // Assert: Docker stop carries no exit code; its later die event supplies it.
    let pending = observation(&receiver);
    assert_eq!(pending.containers[0].state, None);
    assert!(pending.error.is_empty());
    // Act
    tracker.observe(&event("die", "first", "1", Some("137")));
    tracker.observe(&event("destroy", "first", "1", None));
    // Assert
    let exited = observation(&receiver);
    assert_eq!(exited.containers[0].state, Some(State::ExitCode(137)));
    assert!(exited.error.is_empty());
}

#[test]
fn removing_never_started_containers_requires_no_exit_but_started_ones_do() {
    // Arrange
    let (mut tracker, receiver) = fixture();
    tracker.observe(&event("create", "first", "1", None));
    // Act
    tracker.observe(&event("destroy", "first", "1", None));
    // Assert
    let unstarted = observation(&receiver);
    assert_eq!(unstarted.containers[0].state, None);
    assert!(unstarted.error.is_empty());
    // Act
    tracker.observe(&event("start", "replacement", "1", None));
    tracker.observe(&event("stop", "replacement", "1", None));
    tracker.observe(&event("destroy", "replacement", "1", None));
    // Assert: removal cannot silently erase the pending exit of a started process.
    let missing = observation(&receiver);
    assert_eq!(missing.containers[0].state, None);
    assert!(missing.error.contains("exit code"));
}
