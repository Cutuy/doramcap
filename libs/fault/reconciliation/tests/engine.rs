use fault_aggregation::{Config, Freshness, Health};
use fault_protocol::{AggregationPayload, ReporterFrame, ReporterPayload};
use fault_reconciliation::{Engine, Settings};
use fault_reporter::{Definition, Reporter, Text};
use std::time::Duration;

fn settings() -> Settings {
    Settings {
        inputs: vec![("a".into(), "app-a".into()), ("b".into(), "app-b".into())],
        aggregation: Config {
            max_sources: 64,
            max_conditions_per_source: 256,
            max_batch_records: 512,
            output_capacity: 1024,
            dedup_window: 128,
            transport_allowance: Duration::ZERO,
        },
        source_timeout: Duration::from_secs(5),
        snapshot_interval: Duration::from_secs(1),
        maintenance: Duration::from_millis(10),
        max_frame_bytes: 2 * 1024 * 1024,
    }
}
fn frame(payload: ReporterPayload) -> Vec<u8> {
    fault_protocol::encode_reporter(
        &ReporterFrame {
            published_unix_ns: None,
            payload,
        },
        Default::default(),
    )
    .unwrap()
}
fn view(engine: &Engine) -> fault_aggregation::View {
    match engine.snapshot(Duration::ZERO).payload {
        AggregationPayload::Snapshot(v) => v,
        _ => unreachable!(),
    }
}
#[test]
fn host_identities_must_differ() {
    // Act
    let result = Engine::new(settings(), 10, 10);
    // Assert
    assert!(result.is_err());
}
#[test]
fn registration_timeout_restart_and_collision() {
    // Arrange
    let mut engine = Engine::new(settings(), 10, 11).unwrap();
    let (r, mut p) = Reporter::new(1, Default::default()).unwrap();
    let mut def = Definition::new("test", "disk", "write").unwrap();
    def.policy.deadline = Some(Duration::from_millis(100));
    let c = r.register(def).unwrap();
    c.failure(Text::new("io").unwrap()).unwrap();
    // Act / Assert: batches need a registration snapshot, and inputs own one source.
    assert!(
        engine
            .receive(
                "a",
                &frame(ReporterPayload::Batch(p.batch(10).unwrap())),
                Duration::ZERO
            )
            .is_err()
    );
    let bytes = frame(ReporterPayload::Snapshot(p.snapshot()));
    engine.receive("a", &bytes, Duration::ZERO).unwrap();
    assert!(engine.receive("b", &bytes, Duration::ZERO).is_err());
    assert!(engine.receive("a", b"bad", Duration::ZERO).is_err());
    // Act: the failed condition outlives its deadline.
    engine.advance(Duration::from_secs(1)).unwrap();
    // Assert
    let v = view(&engine);
    let source = v.sources.iter().find(|s| s.source == 1).unwrap();
    assert_eq!(source.conditions[0].health, Health::Active);
    assert_eq!(source.conditions[0].freshness, Freshness::Stale);
    // Act: the input exceeds its source timeout.
    engine.advance(Duration::from_secs(6)).unwrap();
    // Assert
    assert!(
        !view(&engine)
            .sources
            .iter()
            .find(|s| s.source == 1)
            .unwrap()
            .connected
    );
    // Act: a new incarnation retires the old source.
    let (_r, p) = Reporter::new(2, Default::default()).unwrap();
    engine
        .receive(
            "a",
            &frame(ReporterPayload::Snapshot(p.snapshot())),
            Duration::from_secs(6),
        )
        .unwrap();
    // Assert
    assert!(!view(&engine).sources.iter().any(|s| s.source == 1));
    assert!(view(&engine).sources.iter().any(|s| s.source == 2));
}
#[test]
fn immediate_failure_publication_and_idle_snapshot() {
    // Arrange
    let mut e = Engine::new(settings(), 10, 11).unwrap();
    let (r, p) = Reporter::new(1, Default::default()).unwrap();
    let c = r
        .register(Definition::new("test", "disk", "write").unwrap())
        .unwrap();
    e.receive(
        "a",
        &frame(ReporterPayload::Snapshot(p.snapshot())),
        Duration::ZERO,
    )
    .unwrap();
    e.advance(Duration::ZERO).unwrap();
    e.publications(Duration::ZERO);
    // Act
    c.failure(Text::new("io").unwrap()).unwrap();
    e.receive(
        "a",
        &frame(ReporterPayload::Snapshot(p.snapshot())),
        Duration::from_millis(1),
    )
    .unwrap();
    let frames = e.publications(Duration::from_millis(1));
    // Assert
    assert!(frames.iter().any(|f|matches!(&f.payload,AggregationPayload::Updates(b) if b.updates.iter().any(|u|u.state.is_some_and(|s|s.health==Health::Active)))));
    assert!(
        e.publications(Duration::from_secs(2))
            .iter()
            .any(|f| matches!(f.payload, AggregationPayload::Snapshot(_)))
    );
}

fn app_name(frame: &fault_protocol::AggregationFrame, source: u128) -> Option<&str> {
    frame
        .source_apps
        .iter()
        .find(|entry| entry.source.as_slice() == source.to_be_bytes())
        .map(|entry| entry.app.as_str())
}

#[test]
fn source_metadata_names_existing_condition_updates_and_snapshots() {
    // Arrange: reporter components differ from the upstream app name.
    let mut config = settings();
    config.inputs = vec![("a".into(), "camera".into()), ("b".into(), "camera".into())];
    let mut engine = Engine::new(config, 10, 11).unwrap();
    assert!(engine.snapshot(Duration::ZERO).source_apps.is_empty());
    for (input, source, fail) in [("a", 1, true), ("b", 2, false)] {
        let (reporter, publisher) = Reporter::new(source, Default::default()).unwrap();
        let check = reporter
            .register(Definition::new("library-internal", "device", "capture").unwrap())
            .unwrap();
        if fail {
            check.failure(Text::new("capture failed").unwrap()).unwrap();
        } else {
            check.success().unwrap();
        }
        engine
            .receive(
                input,
                &frame(ReporterPayload::Snapshot(publisher.snapshot())),
                Duration::ZERO,
            )
            .unwrap();
    }
    // Act / Assert: attribution is metadata; health remains in the original condition state.
    let snapshot = engine.snapshot(Duration::ZERO);
    assert_eq!(app_name(&snapshot, 1), Some("camera"));
    assert_eq!(app_name(&snapshot, 2), Some("camera"));
    let AggregationPayload::Snapshot(view) = snapshot.payload else {
        panic!("snapshot");
    };
    assert_eq!(view.sources[0].conditions[0].health, Health::Active);
    assert_eq!(view.sources[1].conditions[0].health, Health::Healthy);
    for publication in engine.publications(Duration::ZERO) {
        let bytes = fault_protocol::encode_aggregation(&publication, engine.limits()).unwrap();
        let decoded = fault_protocol::decode_aggregation(&bytes, engine.limits()).unwrap();
        assert_eq!(decoded.source_apps, publication.source_apps);
        if let AggregationPayload::Updates(batch) = decoded.payload {
            assert!(
                batch
                    .updates
                    .iter()
                    .all(|update| app_name(&publication, update.source) == Some("camera"))
            );
        }
    }
    // Act / Assert: disconnection retains the name and the existing stale condition evidence.
    engine.disconnect("a", Duration::ZERO).unwrap();
    let snapshot = engine.snapshot(Duration::ZERO);
    assert_eq!(app_name(&snapshot, 1), Some("camera"));
    let AggregationPayload::Snapshot(view) = snapshot.payload else {
        panic!("snapshot");
    };
    let source = view
        .sources
        .iter()
        .find(|source| source.source == 1)
        .unwrap();
    assert!(!source.connected);
    assert_eq!(source.conditions[0].health, Health::Active);
    assert_eq!(source.conditions[0].freshness, Freshness::Stale);
}

#[test]
fn restart_keeps_attribution_for_queued_history_and_replaces_snapshot_identity() {
    // Arrange: drain one record per frame, with a restart before the first drain.
    let mut config = settings();
    config.inputs = vec![("a".into(), "camera".into())];
    config.aggregation.max_batch_records = 1;
    let mut engine = Engine::new(config, 10, 11).unwrap();
    for source in [1, 2] {
        let (reporter, publisher) = Reporter::new(source, Default::default()).unwrap();
        reporter
            .register(Definition::new("internal", "device", "capture").unwrap())
            .unwrap()
            .failure(Text::new("failed").unwrap())
            .unwrap();
        engine
            .receive(
                "a",
                &frame(ReporterPayload::Snapshot(publisher.snapshot())),
                Duration::ZERO,
            )
            .unwrap();
    }
    // Act
    let publications = engine.publications(Duration::ZERO);
    // Assert: old-source records, including retirement, carry the correct app name.
    let mut retired = false;
    for publication in &publications {
        if let AggregationPayload::Updates(batch) = &publication.payload {
            for update in &batch.updates {
                assert_eq!(app_name(publication, update.source), Some("camera"));
                retired |=
                    update.source == 1 && update.kind == fault_aggregation::UpdateKind::Retired;
            }
        }
    }
    assert!(retired);
    let snapshot = engine.snapshot(Duration::ZERO);
    assert_eq!(app_name(&snapshot, 1), None);
    assert_eq!(app_name(&snapshot, 2), Some("camera"));
    let AggregationPayload::Snapshot(view) = snapshot.payload else {
        panic!("snapshot");
    };
    assert_eq!(view.sources.len(), 1);
    assert_eq!(view.sources[0].source, 2);
}

#[test]
fn overflow_repairs_existing_state_with_current_source_attribution() {
    // Arrange
    let mut config = settings();
    config.inputs = vec![("a".into(), "camera".into())];
    config.aggregation.output_capacity = 1;
    let mut engine = Engine::new(config, 100, 101).unwrap();
    for source in 1..=20 {
        let (reporter, publisher) = Reporter::new(source, Default::default()).unwrap();
        reporter
            .register(Definition::new("internal", "device", "capture").unwrap())
            .unwrap()
            .failure(Text::new("failed").unwrap())
            .unwrap();
        engine
            .receive(
                "a",
                &frame(ReporterPayload::Snapshot(publisher.snapshot())),
                Duration::ZERO,
            )
            .unwrap();
    }
    // Act
    let publications = engine.publications(Duration::ZERO);
    // Assert: the existing repair snapshot contains the latest source and its name.
    let snapshot = publications
        .iter()
        .find(|publication| matches!(publication.payload, AggregationPayload::Snapshot(_)))
        .unwrap();
    assert_eq!(snapshot.source_apps.len(), 1);
    assert_eq!(app_name(snapshot, 20), Some("camera"));
    let AggregationPayload::Snapshot(view) = &snapshot.payload else {
        unreachable!();
    };
    assert!(view.stats.dropped_updates > 0);
    assert_eq!(view.sources.len(), 1);
    assert_eq!(view.sources[0].source, 20);
    assert_eq!(view.sources[0].conditions[0].health, Health::Active);
}
