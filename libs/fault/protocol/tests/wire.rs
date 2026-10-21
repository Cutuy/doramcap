use fault_aggregation as a;
use fault_protocol::*;
use fault_reporter as r;
use prost::Message;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
#[derive(Default)]
struct Clock(AtomicU64);
impl r::Clock for Clock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::Relaxed))
    }
}
fn observation() -> r::Observation {
    r::Observation {
        key: r::Key {
            source: 0x0102030405060708090a0b0c0d0e0f10,
            condition: 0,
        },
        sequence: 2,
        observed_at: Duration::from_secs(1),
        outcome: r::Outcome::Success,
        successes: 1,
        failures: 1,
        first_failure_at: Some(Duration::ZERO),
        last_failure_at: Some(Duration::ZERO),
        success_since: Some(Duration::from_secs(1)),
        consecutive_successes: 1,
        max_success_span: Duration::ZERO,
        detail: r::Text::new("recovered 🟢").unwrap(),
        trace: Some(r::TraceContext {
            trace_id: [3; 16],
            span_id: [4; 8],
        }),
    }
}
fn decode_snapshot(frame: ReporterFrame) -> r::Snapshot {
    let bytes = encode_reporter(&frame, Limits::default()).unwrap();
    let decoded = decode_reporter(&bytes, Limits::default()).unwrap();
    assert_eq!(decoded.published_unix_ns, frame.published_unix_ns);
    let ReporterPayload::Snapshot(s) = decoded.payload else {
        panic!("snapshot")
    };
    s
}
#[test]
fn ipc_to_aggregation_to_storage_roundtrip() {
    let clock = Arc::new(Clock::default());
    let (reporter, mut publisher) =
        r::Reporter::with_clock(42, r::Config::default(), clock.clone()).unwrap();
    let mut def = r::Definition::new("recorder", "disk", "write_failed").unwrap();
    def.policy.deadline = Some(Duration::from_secs(5));
    let check = reporter.register(def).unwrap();
    let observability = std::sync::Arc::new(fault_aggregation::FaultObservability::new());
    let mut agg = a::Aggregator::new(99, a::Config::default(), observability.clone()).unwrap();
    let registration = decode_snapshot(ReporterFrame {
        published_unix_ns: Some(0),
        payload: ReporterPayload::Snapshot(publisher.snapshot()),
    });
    assert_eq!(registration.conditions[0].definition, def);
    assert!(registration.conditions[0].latest.is_none());
    agg.reconcile(&registration, Duration::ZERO).unwrap();
    for (ms, outcome) in [
        (0, r::Outcome::Failure),
        (10, r::Outcome::Success),
        (1010, r::Outcome::Success),
    ] {
        clock.0.store(ms, Ordering::Relaxed);
        check
            .observe(
                outcome,
                r::Text::new("diagnostic").unwrap(),
                Some(r::TraceContext {
                    trace_id: [7; 16],
                    span_id: [8; 8],
                }),
            )
            .unwrap();
        let original = publisher.batch(32).unwrap();
        let expected = original.observations.clone();
        let bytes = encode_reporter(
            &ReporterFrame {
                published_unix_ns: Some(1234),
                payload: ReporterPayload::Batch(original),
            },
            Limits::default(),
        )
        .unwrap();
        let frame = decode_reporter(&bytes, Limits::default()).unwrap();
        let ReporterPayload::Batch(batch) = frame.payload else {
            panic!("batch")
        };
        assert_eq!(batch.observations, expected);
        agg.ingest_batch(&batch, Duration::from_millis(ms)).unwrap();
    }
    assert_eq!(agg.state(check.key()).unwrap().health, a::Health::Healthy);
    agg.set_connected(42, false, Duration::from_secs(2))
        .unwrap();
    let updates: Vec<_> = std::iter::from_fn(|| agg.pop_update()).collect();
    let bytes = encode_aggregation(
        &AggregationFrame {
            published_unix_ns: Some(9876),
            emitted_at: Duration::from_secs(2),
            payload: AggregationPayload::Updates(Updates {
                epoch: 99,
                updates: updates.clone(),
                stats: observability.stats(),
            }),
        },
        Limits::default(),
    )
    .unwrap();
    let frame = decode_aggregation(&bytes, Limits::default()).unwrap();
    assert_eq!(frame.published_unix_ns, Some(9876));
    assert_eq!(frame.emitted_at, Duration::from_secs(2));
    let AggregationPayload::Updates(batch) = frame.payload else {
        panic!("updates")
    };
    assert_eq!(batch.updates, updates);
    assert_eq!(batch.stats, observability.stats());
    let expected = agg.snapshot();
    let bytes = encode_aggregation(
        &AggregationFrame {
            published_unix_ns: None,
            emitted_at: Duration::from_secs(2),
            payload: AggregationPayload::Snapshot(expected.clone()),
        },
        Limits::default(),
    )
    .unwrap();
    let frame = decode_aggregation(&bytes, Limits::default()).unwrap();
    let AggregationPayload::Snapshot(view) = frame.payload else {
        panic!("view")
    };
    assert_eq!(view.epoch, expected.epoch);
    assert_eq!(view.revision, expected.revision);
    assert_eq!(view.sources[0].conditions, expected.sources[0].conditions);
    assert!(!view.sources[0].connected);
    agg.retire_source(42).unwrap();
    let retired = agg.pop_update().unwrap();
    assert_eq!(
        a::Update::from_proto(retired.to_proto().unwrap()).unwrap(),
        retired
    );
}
#[test]
fn exact_identity_zero_timestamps_and_optional_fields() {
    let o = observation();
    let p = o.to_proto().unwrap();
    assert_eq!(
        p.key.as_ref().unwrap().source,
        (1..=16).collect::<Vec<u8>>()
    );
    assert_eq!(p.first_failure_at_ns, Some(0));
    assert_eq!(r::Observation::from_proto(p).unwrap(), o);
    for deadline in [None, Some(Duration::from_secs(5))] {
        for recovery in [Duration::ZERO, Duration::from_secs(1)] {
            let p = r::Policy {
                deadline,
                recovery,
                ..Default::default()
            };
            assert_eq!(r::Policy::from_proto(p.to_proto().unwrap()).unwrap(), p);
        }
    }
}
#[test]
fn malformed_ids_enums_messages_and_evidence_are_rejected() {
    let original = observation().to_proto().unwrap();
    let mut p = original.clone();
    p.key.as_mut().unwrap().source = vec![1; 15];
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.key.as_mut().unwrap().source = vec![0; 16];
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.outcome = 99;
    assert!(matches!(
        r::Observation::from_proto(p),
        Err(Error::UnknownEnum(_, 99))
    ));
    let mut p = original.clone();
    p.outcome = 0;
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.key = None;
    assert!(matches!(
        r::Observation::from_proto(p),
        Err(Error::Missing("key"))
    ));
    let mut p = original.clone();
    p.trace.as_mut().unwrap().span_id = vec![1; 7];
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.detail = "x".repeat(193);
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.successes = 123;
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original.clone();
    p.first_failure_at_ns = None;
    assert!(r::Observation::from_proto(p).is_err());
    let mut p = original;
    p.success_since_ns = Some(u64::MAX);
    assert!(r::Observation::from_proto(p).is_err());
    let mut policy = r::Policy::default().to_proto().unwrap();
    policy.recovery_ns = None;
    assert_eq!(
        r::Policy::from_proto(policy),
        Err(Error::Missing("recovery_ns"))
    );
}
#[test]
fn version_frame_cardinality_and_duration_limits() {
    let frame = ReporterFrame {
        published_unix_ns: None,
        payload: ReporterPayload::Batch(r::Batch {
            source: observation().key.source,
            emitted_at: Duration::from_secs(1),
            observations: vec![observation()],
            losses: r::Losses::default(),
        }),
    };
    let bytes = encode_reporter(&frame, Limits::default()).unwrap();
    assert!(matches!(
        decode_reporter(
            &bytes,
            Limits {
                max_frame_bytes: bytes.len() - 1,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        decode_reporter(
            &bytes,
            Limits {
                max_records: 0,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        encode_reporter(
            &frame,
            Limits {
                max_records: 0,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    let mut p = pb::ReporterPublication::decode(bytes.as_slice()).unwrap();
    p.version = 2;
    assert!(matches!(
        decode_reporter(&p.encode_to_vec(), Limits::default()),
        Err(Error::UnsupportedVersion(2))
    ));
    p.version = 1;
    p.payload = None;
    assert!(matches!(
        decode_reporter(&p.encode_to_vec(), Limits::default()),
        Err(Error::Missing("payload"))
    ));
    assert!(matches!(
        decode_reporter(&[255], Limits::default()),
        Err(Error::Decode(_))
    ));
    assert_eq!(
        r::Policy {
            recovery: Duration::MAX,
            ..Default::default()
        }
        .to_proto(),
        Err(Error::DurationOverflow)
    );
}
#[test]
fn snapshots_reject_duplicate_and_mismatched_identities() {
    let definition = r::Definition::new("writer", "disk", "io").unwrap();
    let o = observation();
    let c = r::ConditionSnapshot {
        key: o.key,
        definition,
        latest: Some(o),
    };
    let mut snapshot = r::Snapshot {
        source: o.key.source,
        emitted_at: o.observed_at,
        conditions: vec![c, c],
        losses: r::Losses::default(),
    };
    assert!(snapshot.to_proto().is_err());
    snapshot.conditions.pop();
    snapshot.source = 9;
    assert!(snapshot.to_proto().is_err());
    snapshot.source = o.key.source;
    snapshot.emitted_at = Duration::ZERO;
    assert!(snapshot.to_proto().is_err());
}
#[test]
fn all_enum_variants_translate_and_late_evidence_survives() {
    for mode in [r::Mode::Stateful, r::Mode::EventOnly] {
        let policy = r::Policy {
            mode,
            ..Default::default()
        };
        assert_eq!(
            r::Policy::from_proto(policy.to_proto().unwrap()).unwrap(),
            policy
        );
    }
    for severity in [
        r::Severity::Warning,
        r::Severity::Error,
        r::Severity::Critical,
    ] {
        let mut d = r::Definition::new("component", "", "code").unwrap();
        d.severity = severity;
        assert_eq!(r::Definition::from_proto(d.to_proto().unwrap()).unwrap(), d);
    }
    let o = observation();
    for health in [
        a::Health::Unknown,
        a::Health::Healthy,
        a::Health::Active,
        a::Health::Recovering,
    ] {
        for freshness in [
            a::Freshness::Unknown,
            a::Freshness::Fresh,
            a::Freshness::Stale,
        ] {
            let s = a::State {
                key: o.key,
                definition: r::Definition::new("c", "", "code").unwrap(),
                health,
                freshness,
                latest: Some(o),
                revision: 9,
            };
            assert_eq!(a::State::from_proto(s.to_proto().unwrap()).unwrap(), s);
        }
    }
    for kind in [
        a::UpdateKind::Registered,
        a::UpdateKind::Observation,
        a::UpdateKind::Transition,
        a::UpdateKind::Reconciled,
        a::UpdateKind::Late,
        a::UpdateKind::Event,
        a::UpdateKind::Freshness,
        a::UpdateKind::Source,
        a::UpdateKind::Retired,
    ] {
        let source_wide = matches!(kind, a::UpdateKind::Source | a::UpdateKind::Retired);
        let has_obs = !matches!(
            kind,
            a::UpdateKind::Registered
                | a::UpdateKind::Freshness
                | a::UpdateKind::Source
                | a::UpdateKind::Retired
        );
        let s = a::State {
            key: o.key,
            definition: r::Definition::new("c", "", "code").unwrap(),
            health: a::Health::Recovering,
            freshness: a::Freshness::Fresh,
            latest: Some(o),
            revision: 9,
        };
        let u = a::Update {
            epoch: 8,
            revision: 9,
            kind,
            key: (!source_wide).then_some(o.key),
            state: (!source_wide).then_some(s),
            observation: has_obs.then_some(o),
            source: o.key.source,
            connected: true,
            losses: r::Losses {
                failures: 1,
                successes: 2,
                coalesced_successes: 3,
            },
        };
        assert_eq!(a::Update::from_proto(u.to_proto().unwrap()).unwrap(), u);
        let batch = Updates {
            epoch: 8,
            updates: vec![u, u],
            stats: a::Stats::default(),
        };
        assert!(batch.to_proto().is_err());
    }
}
#[test]
fn descriptor_set_has_both_mcap_roots_and_dependencies() {
    let descriptors = prost_types::FileDescriptorSet::decode(FILE_DESCRIPTOR_SET).unwrap();
    for file in ["common.proto", "reporter.proto", "aggregation.proto"] {
        assert!(
            descriptors
                .file
                .iter()
                .any(|f| f.name.as_deref() == Some(file))
        );
    }
    for root in [REPORTER_SCHEMA_NAME, AGGREGATION_SCHEMA_NAME] {
        assert!(
            descriptors
                .file
                .iter()
                .any(|f| f.message_type.iter().any(|m| format!(
                    "{}.{}",
                    f.package.as_deref().unwrap(),
                    m.name.as_deref().unwrap()
                ) == root))
        );
    }
}

#[test]
fn aggregation_rejects_wrong_version_missing_payload_and_bad_revisions() {
    let agg = a::Aggregator::new(
        2,
        a::Config::default(),
        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
    )
    .unwrap();
    let frame = AggregationFrame {
        published_unix_ns: Some(0),
        emitted_at: Duration::ZERO,
        payload: AggregationPayload::Snapshot(agg.snapshot()),
    };
    let bytes = encode_aggregation(&frame, Limits::default()).unwrap();
    let decoded = decode_aggregation(&bytes, Limits::default()).unwrap();
    assert_eq!(decoded.published_unix_ns, Some(0));
    let AggregationPayload::Snapshot(s) = decoded.payload else {
        panic!("snapshot")
    };
    assert_eq!(s.revision, 0);
    assert!(s.sources.is_empty());
    let mut p = pb::AggregationPublication::decode(bytes.as_slice()).unwrap();
    p.version = 2;
    assert!(matches!(
        decode_aggregation(&p.encode_to_vec(), Limits::default()),
        Err(Error::UnsupportedVersion(2))
    ));
    p.version = 1;
    p.payload = None;
    assert!(matches!(
        decode_aggregation(&p.encode_to_vec(), Limits::default()),
        Err(Error::Missing("payload"))
    ));
    assert!(matches!(
        decode_aggregation(
            &bytes,
            Limits {
                max_frame_bytes: 0,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    let o = observation();
    let state = a::State {
        key: o.key,
        definition: r::Definition::new("x", "", "code").unwrap(),
        health: a::Health::Recovering,
        freshness: a::Freshness::Fresh,
        latest: Some(o),
        revision: 2,
    };
    let view = a::View {
        epoch: 2,
        revision: 1,
        stats: a::Stats::default(),
        sources: vec![a::SourceView {
            source: o.key.source,
            connected: true,
            losses: r::Losses::default(),
            conditions: vec![state],
        }],
    };
    assert!(view.to_proto().is_err());
}

#[test]
fn snapshot_and_update_counters_roundtrip_without_truncation() {
    let (reporter, publisher) = r::Reporter::new(1, r::Config::default()).unwrap();
    reporter
        .register(r::Definition::new("x", "", "code").unwrap())
        .unwrap();
    let mut s = publisher.snapshot();
    s.losses = r::Losses {
        failures: u64::MAX,
        successes: u64::MAX - 1,
        coalesced_successes: u64::MAX - 2,
    };
    let decoded = decode_snapshot(ReporterFrame {
        published_unix_ns: None,
        payload: ReporterPayload::Snapshot(s.clone()),
    });
    assert_eq!(decoded.losses, s.losses);
    assert_eq!(decoded.conditions, s.conditions);
    let update = a::Update {
        epoch: 2,
        revision: 7,
        kind: a::UpdateKind::Source,
        key: None,
        state: None,
        observation: None,
        source: 1,
        connected: false,
        losses: s.losses,
    };
    let updates = Updates {
        epoch: 2,
        updates: vec![update],
        stats: a::Stats {
            dropped_updates: u64::MAX,
            rejected_inputs: 42,
            too_old: 9,
        },
    };
    let mut p = updates.to_proto().unwrap();
    p.epoch = 3_u128.to_be_bytes().to_vec();
    assert!(Updates::from_proto(p).is_err());
    let decoded = Updates::from_proto(updates.to_proto().unwrap()).unwrap();
    assert_eq!(decoded.stats, updates.stats);
    assert_eq!(decoded.updates, updates.updates);
}

#[test]
fn additive_unknown_fields_remain_compatible() {
    let frame = ReporterFrame {
        published_unix_ns: None,
        payload: ReporterPayload::Batch(r::Batch {
            source: observation().key.source,
            emitted_at: Duration::from_secs(1),
            observations: vec![observation()],
            losses: r::Losses::default(),
        }),
    };
    let mut bytes = encode_reporter(&frame, Limits::default()).unwrap();
    bytes.extend_from_slice(&[0xa0, 0x06, 0x01]); // Unknown varint field 100 = 1.
    let ReporterPayload::Batch(batch) = decode_reporter(&bytes, Limits::default()).unwrap().payload
    else {
        panic!("batch")
    };
    assert_eq!(batch.observations, vec![observation()]);
}
