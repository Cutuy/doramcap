use prost::Message;
use step_provenance::{Engine, Limits, VERSION, pb::*};

fn reference(step: &str, sequence: u64) -> GenerationRef {
    GenerationRef {
        producer: step.into(),
        epoch: "boot-1".into(),
        step: step.into(),
        generation: format!("opaque-{sequence}"),
        sequence,
    }
}
fn patch(
    r: &GenerationRef,
    start: Option<u64>,
    end: Option<u64>,
    inputs: Option<Vec<GenerationRef>>,
) -> StepUpdate {
    StepUpdate {
        version: VERSION,
        generation: Some(r.clone()),
        clock_domain: "host/boot/monotonic".into(),
        invocation: "run-1".into(),
        start_ns: start,
        end_ns: end,
        inputs: inputs.map(|refs| Inputs { refs }),
    }
}
fn probe(step: &str, boundary: Boundary, selector: Selector) -> Probe {
    Probe {
        step: Some(reference(step, 0).step_key()),
        boundary: boundary as i32,
        selector: selector as i32,
    }
}
fn definition(name: &str, anchor: &str, start: Probe, end: Probe) -> MeasurementDefinition {
    MeasurementDefinition {
        name: name.into(),
        anchor: Some(reference(anchor, 0).step_key()),
        start: Some(start),
        end: Some(end),
        missing_data: MissingDataPolicy::WaitUntilEviction as i32,
    }
}
fn latency() -> MeasurementDefinition {
    definition(
        "capture-to-detection",
        "detect",
        probe("camera", Boundary::Start, Selector::ConsumedAncestor),
        probe("detect", Boundary::End, Selector::ConsumedAncestor),
    )
}
fn engine(defs: Vec<MeasurementDefinition>, n: usize) -> Engine {
    Engine::new(
        Definitions {
            version: VERSION,
            measurements: defs,
        },
        Limits {
            generations_per_stream: n,
            ..Limits::default()
        },
    )
    .unwrap()
}
fn permutations<T: Clone>(xs: &[T]) -> Vec<Vec<T>> {
    if xs.is_empty() {
        return vec![vec![]];
    }
    let mut result = Vec::new();
    for i in 0..xs.len() {
        let mut rest = xs.to_vec();
        let first = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, first.clone());
            result.push(tail);
        }
    }
    result
}

#[test]
fn all_partial_and_dependency_arrival_orders_have_one_identical_result() {
    let c = reference("camera", 1);
    let d = reference("detect", 9);
    let m = reference("middle", 8);
    let updates = [
        patch(&c, Some(0), None, Some(vec![])),
        patch(&d, Some(10), None, None),
        patch(&d, None, Some(25), Some(vec![m.clone()])),
        patch(&m, Some(5), Some(9), Some(vec![c.clone()])),
    ];
    for order in permutations(&updates) {
        let mut e = engine(vec![latency()], 10);
        let results: Vec<_> = order
            .into_iter()
            .flat_map(|u| e.ingest(u).unwrap())
            .collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].duration_ns, Some(25));
        assert_eq!(
            results[0].start.as_ref().unwrap().generation.as_ref(),
            Some(&c)
        );
        assert_eq!(e.stats().pending, 0);
        assert_eq!(e.stats().wait_keys, 0);
        for u in &updates {
            assert!(e.ingest(u.clone()).unwrap().is_empty());
        }
    }
}

#[test]
fn ambiguity_waits_for_unknown_branch_before_resolving() {
    let mut e = engine(vec![latency()], 10);
    let d = reference("detect", 1);
    let m = reference("middle", 1);
    e.ingest(patch(&reference("camera", 1), Some(1), None, Some(vec![])))
        .unwrap();
    assert!(
        e.ingest(patch(
            &d,
            Some(3),
            Some(5),
            Some(vec![reference("camera", 1), m.clone()])
        ))
        .unwrap()
        .is_empty()
    );
    let results = e
        .ingest(patch(
            &m,
            Some(2),
            Some(3),
            Some(vec![reference("camera", 2)]),
        ))
        .unwrap();
    assert_eq!(results[0].reason(), Reason::Ambiguous);
}

#[test]
fn late_edges_cannot_form_a_cycle_and_rejected_patches_are_atomic() {
    let mut e = engine(vec![], 10);
    let a = reference("a", 1);
    let b = reference("b", 1);
    e.ingest(patch(&a, Some(1), None, Some(vec![b.clone()])))
        .unwrap();
    let before = e.stats();
    assert!(
        e.ingest(patch(&b, Some(2), None, Some(vec![a.clone()])))
            .is_err()
    );
    assert_eq!(e.stats(), before);
    assert!(e.record(&b).is_none());
    assert!(e.ingest(patch(&a, Some(99), Some(100), None)).is_err());
    assert_eq!(e.record(&a).unwrap().start_ns, Some(1));
    assert_eq!(e.record(&a).unwrap().end_ns, None);
}

#[test]
fn reused_sensor_and_cached_timestamp_survive_sensor_eviction() {
    let mut e = engine(vec![latency()], 1);
    let c = reference("camera", 1);
    let d = reference("detect", 1);
    e.ingest(patch(&c, Some(2), None, Some(vec![]))).unwrap();
    e.ingest(patch(&d, Some(3), None, Some(vec![c.clone()])))
        .unwrap();
    assert!(
        e.ingest(patch(&reference("camera", 2), Some(7), None, Some(vec![])))
            .unwrap()
            .is_empty()
    );
    let results = e.ingest(patch(&d, None, Some(8), None)).unwrap();
    assert_eq!(results[0].duration_ns, Some(6));
    let results = e
        .ingest(patch(
            &reference("detect", 2),
            Some(9),
            Some(10),
            Some(vec![c]),
        ))
        .unwrap();
    assert_eq!(results[0].reason(), Reason::HistoryEvicted);
}

#[test]
fn eviction_finalizes_waiters_and_anchors_and_rejects_old_replay() {
    let mut e = engine(vec![latency()], 1);
    let c = reference("camera", 1);
    let d = reference("detect", 1);
    e.ingest(patch(&c, None, Some(2), Some(vec![]))).unwrap();
    e.ingest(patch(&d, Some(3), Some(4), Some(vec![c.clone()])))
        .unwrap();
    let result = e
        .ingest(patch(&reference("camera", 2), Some(5), None, Some(vec![])))
        .unwrap();
    assert_eq!(result[0].reason(), Reason::HistoryEvicted);
    assert_eq!(e.stats().wait_links, 0);
    assert!(e.ingest(patch(&c, Some(1), None, None)).is_err());
    e.ingest(patch(&reference("detect", 2), Some(6), None, None))
        .unwrap();
    let result = e
        .ingest(patch(&reference("detect", 3), Some(7), None, None))
        .unwrap();
    assert_eq!(result[0].reason(), Reason::AnchorEvicted);
    assert_eq!(e.stats().instances, 1);
}

#[test]
fn out_of_order_retention_keeps_n_highest_sequences() {
    let mut e = engine(vec![], 2);
    for n in [10, 8, 9] {
        e.ingest(patch(&reference("s", n), Some(n), None, None))
            .unwrap();
    }
    assert!(e.record(&reference("s", 8)).is_none());
    for n in [9, 10] {
        assert!(e.record(&reference("s", n)).is_some());
    }
    assert!(
        e.ingest(patch(&reference("s", 7), Some(7), None, None))
            .is_err()
    );
}

#[test]
fn fixed_selectors_freeze_at_resolution_and_latest_uses_anchor_snapshot() {
    let def = definition(
        "freshness",
        "detect",
        probe("camera", Boundary::Start, Selector::LatestKnownAtAnchor),
        probe("detect", Boundary::End, Selector::ConsumedAncestor),
    );
    let mut e = engine(vec![def.clone()], 10);
    e.ingest(patch(&reference("camera", 1), Some(1), None, Some(vec![])))
        .unwrap();
    e.ingest(patch(&reference("detect", 1), Some(2), None, None))
        .unwrap();
    e.ingest(patch(&reference("camera", 2), Some(3), None, Some(vec![])))
        .unwrap();
    let result = e
        .ingest(patch(&reference("detect", 1), None, Some(8), None))
        .unwrap();
    assert_eq!(result[0].duration_ns, Some(7));
    let mut empty = engine(vec![def], 10);
    let result = empty
        .ingest(patch(&reference("detect", 1), Some(2), None, None))
        .unwrap();
    assert_eq!(result[0].reason(), Reason::NoMatch);
}

#[test]
fn same_invocation_waits_for_target_and_previous_waits_for_exact_sequence() {
    let def = definition(
        "matched",
        "detect",
        probe("camera", Boundary::Start, Selector::SameInvocation),
        probe("detect", Boundary::End, Selector::ConsumedAncestor),
    );
    let mut e = engine(vec![def], 10);
    e.ingest(patch(&reference("detect", 1), Some(5), Some(10), None))
        .unwrap();
    let mut unrelated = patch(&reference("camera", 2), Some(4), None, None);
    unrelated.invocation = "different".into();
    assert!(e.ingest(unrelated).unwrap().is_empty());
    assert_eq!(
        e.ingest(patch(&reference("camera", 1), Some(2), None, None))
            .unwrap()[0]
            .duration_ns,
        Some(8)
    );
    let def = definition(
        "period",
        "camera",
        probe("camera", Boundary::Start, Selector::Previous),
        probe("camera", Boundary::Start, Selector::ConsumedAncestor),
    );
    let mut e = engine(vec![def], 10);
    e.ingest(patch(&reference("camera", 3), Some(30), None, None))
        .unwrap();
    let results = e
        .ingest(patch(&reference("camera", 2), Some(20), None, None))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].duration_ns, Some(10));
}

#[test]
fn clocks_and_negative_duration_produce_explicit_incomplete_results() {
    for (clock, t, expected) in [
        ("remote", 2, Reason::ClockMismatch),
        ("host/boot/monotonic", 20, Reason::InvalidDuration),
    ] {
        let mut e = engine(vec![latency()], 10);
        let mut c = patch(&reference("camera", 1), Some(t), None, Some(vec![]));
        c.clock_domain = clock.into();
        e.ingest(c).unwrap();
        let result = e
            .ingest(patch(
                &reference("detect", 1),
                Some(5),
                Some(10),
                Some(vec![reference("camera", 1)]),
            ))
            .unwrap();
        assert_eq!(result[0].reason(), expected);
        assert_eq!(result[0].duration_ns, None);
    }
}

#[test]
fn unrelated_steps_do_not_evaluate_pending_instances() {
    let mut e = engine(vec![latency()], 10);
    e.ingest(patch(
        &reference("detect", 1),
        Some(1),
        None,
        Some(vec![reference("camera", 1)]),
    ))
    .unwrap();
    let before = e.stats().evaluations;
    for n in 1..=10 {
        e.ingest(patch(&reference("other", n), Some(n), None, Some(vec![])))
            .unwrap();
    }
    assert_eq!(e.stats().evaluations, before);
    assert_eq!(
        e.affected_definitions(&reference("camera", 0).step_key(), Boundary::Start),
        vec!["capture-to-detection"]
    );
    assert!(
        e.affected_definitions(&reference("camera", 0).step_key(), Boundary::End)
            .is_empty()
    );
}

#[test]
fn stream_churn_inputs_and_pending_state_are_bounded() {
    let mut limits = Limits {
        streams: 2,
        generations_per_stream: 2,
        inputs_per_generation: 1,
        ..Limits::default()
    };
    let config = Definitions {
        version: VERSION,
        measurements: vec![latency()],
    };
    let mut e = Engine::new(config.clone(), limits.clone()).unwrap();
    for n in 0..200 {
        e.ingest(patch(
            &reference("detect", n),
            Some(n),
            None,
            Some(vec![reference("missing", n)]),
        ))
        .unwrap();
        assert!(e.stats().instances <= 2);
        assert!(e.stats().wait_links <= 6);
    }
    e.ingest(patch(&reference("camera", 1), Some(1), None, None))
        .unwrap();
    let before = e.stats();
    assert!(
        e.ingest(patch(&reference("third", 1), Some(1), None, None))
            .is_err()
    );
    assert_eq!(e.stats(), before);
    assert!(
        e.ingest(patch(
            &reference("detect", 201),
            Some(201),
            None,
            Some(vec![reference("a", 1), reference("b", 1)])
        ))
        .is_err()
    );
    limits.generations_per_stream = 0;
    assert!(Engine::new(config, limits).is_err());
}

#[test]
fn restart_epochs_keep_identity_distinct_and_conflicts_fail() {
    let mut e = engine(vec![], 10);
    let first = reference("camera", 1);
    let mut next = first.clone();
    next.epoch = "boot-2".into();
    e.ingest(patch(&first, Some(1), None, None)).unwrap();
    e.ingest(patch(&next, Some(2), None, None)).unwrap();
    assert_eq!(e.stats().records, 2);
    let mut conflict = first.clone();
    conflict.sequence = 2;
    assert!(e.ingest(patch(&conflict, Some(3), None, None)).is_err());
    conflict = first;
    conflict.generation = "another-id".into();
    assert!(e.ingest(patch(&conflict, Some(3), None, None)).is_err());
}

#[test]
fn protobuf_presence_unknown_fields_versions_and_enum_validation() {
    let update = patch(&reference("camera", 1), Some(0), None, Some(vec![]));
    let mut wire = update.encode_to_vec();
    // Unknown future field 100 with a varint value. Existing fields survive.
    wire.extend_from_slice(&[0xa0, 0x06, 0x01]);
    let decoded = StepUpdate::decode(wire.as_slice()).unwrap();
    assert_eq!(decoded, update);
    let mut e = engine(vec![], 10);
    e.decode_update(&wire).unwrap();
    assert_eq!(
        e.record(update.generation.as_ref().unwrap())
            .unwrap()
            .start_ns,
        Some(0)
    );
    let mut invalid = update;
    invalid.version = 999;
    assert!(e.ingest(invalid).is_err());
    for field in 0..3 {
        let mut d = latency();
        match field {
            0 => d.start.as_mut().unwrap().selector = 999,
            1 => d.start.as_mut().unwrap().boundary = 999,
            _ => d.missing_data = 999,
        }
        assert!(
            Engine::new(
                Definitions {
                    version: VERSION,
                    measurements: vec![d]
                },
                Limits::default()
            )
            .is_err()
        );
    }
    assert!(!step_provenance::DESCRIPTOR.is_empty());
}

#[test]
fn never_received_generation_expires_when_its_stream_window_advances() {
    for via_intermediate in [false, true] {
        let mut e = engine(vec![latency()], 1);
        let missing = reference(if via_intermediate { "middle" } else { "camera" }, 1);
        e.ingest(patch(
            &reference("detect", 1),
            Some(5),
            Some(10),
            Some(vec![missing.clone()]),
        ))
        .unwrap();
        let mut newer = missing;
        newer.sequence = 10;
        newer.generation = "new".into();
        let result = e
            .ingest(patch(&newer, Some(20), None, Some(vec![])))
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].reason(), Reason::HistoryEvicted);
        assert_eq!(e.stats().wait_links, 0);
    }
}

#[test]
fn shared_ancestors_are_deduplicated_and_state_edges_can_span_generations() {
    let mut e = engine(vec![latency()], 10);
    let c = reference("camera", 1);
    e.ingest(patch(&c, Some(1), None, Some(vec![]))).unwrap();
    e.ingest(patch(
        &reference("state", 1),
        Some(2),
        Some(3),
        Some(vec![c.clone()]),
    ))
    .unwrap();
    e.ingest(patch(
        &reference("state", 2),
        Some(4),
        Some(5),
        Some(vec![reference("state", 1), c]),
    ))
    .unwrap();
    let result = e
        .ingest(patch(
            &reference("detect", 1),
            Some(6),
            Some(9),
            Some(vec![reference("state", 2)]),
        ))
        .unwrap();
    assert_eq!(result[0].duration_ns, Some(8));
}

#[test]
fn invalid_input_sets_metadata_and_wire_sizes_preserve_state() {
    let mut e = engine(vec![], 10);
    let r = reference("camera", 1);
    e.ingest(patch(&r, Some(5), None, Some(vec![]))).unwrap();
    let before = e.stats();
    let mut bad = patch(&r, None, Some(9), None);
    bad.invocation = "changed".into();
    assert!(e.ingest(bad).is_err());
    assert!(e.ingest(patch(&r, None, Some(4), None)).is_err());
    assert!(
        e.ingest(patch(&r, None, Some(9), Some(vec![reference("other", 1)])))
            .is_err()
    );
    assert!(
        e.ingest(patch(
            &reference("other", 1),
            Some(7),
            None,
            Some(vec![r.clone(), r.clone()])
        ))
        .is_err()
    );
    assert!(e.decode_update(&vec![0; (1 << 20) + 1]).is_err());
    assert!(e.decode_update(&[0xff]).is_err());
    assert_eq!(e.stats(), before);
    assert_eq!(e.record(&r).unwrap().end_ns, None);
}

#[test]
fn lineage_reports_roots_and_gaps_without_materializing_transitive_history() {
    let mut e = engine(vec![], 1);
    let c = reference("camera", 1);
    let l = reference("lidar", 1);
    let d = reference("detect", 1);
    e.ingest(patch(&c, Some(1), None, Some(vec![]))).unwrap();
    e.ingest(patch(&d, Some(3), None, Some(vec![c.clone(), l.clone()])))
        .unwrap();
    let lineage = e.lineage(&d).unwrap();
    assert_eq!(lineage.roots, vec![c.clone()]);
    assert_eq!(lineage.missing, vec![l.clone()]);
    assert!(!lineage.is_complete());
    e.ingest(patch(&l, Some(2), None, Some(vec![]))).unwrap();
    assert!(e.lineage(&d).unwrap().is_complete());
    e.ingest(patch(&reference("camera", 2), Some(4), None, Some(vec![])))
        .unwrap();
    assert_eq!(e.lineage(&d).unwrap().expired, vec![c]);
}

#[test]
fn timestamp_only_patch_does_not_revalidate_unchanged_deep_edges() {
    let mut e = Engine::new(
        Definitions {
            version: VERSION,
            measurements: vec![],
        },
        Limits {
            traversal_records: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let a = reference("a", 1);
    let b = reference("b", 1);
    let c = reference("c", 1);
    e.ingest(patch(&a, Some(1), None, Some(vec![b.clone()])))
        .unwrap();
    e.ingest(patch(&b, Some(2), None, Some(vec![c.clone()])))
        .unwrap();
    e.ingest(patch(&c, Some(3), None, Some(vec![]))).unwrap();
    e.ingest(patch(&a, None, Some(10), None)).unwrap();
    assert_eq!(e.record(&a).unwrap().end_ns, Some(10));
    assert!(e.lineage(&a).is_err());
}
