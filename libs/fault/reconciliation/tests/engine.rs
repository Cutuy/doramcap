use fault_aggregation::{Config, Freshness, Health};
use fault_protocol::{AggregationPayload, ReporterFrame, ReporterPayload};
use fault_reconciliation::{Engine, Settings};
use fault_reporter::{Definition, Reporter, Text};
use std::time::Duration;

fn settings() -> Settings {
    Settings {
        inputs: vec!["a".into(), "b".into()],
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
