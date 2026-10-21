use fault_aggregation::*;
use fault_reporter::{self as reporter, *};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
fn ms(t: u64) -> Duration {
    Duration::from_millis(t)
}
#[derive(Default)]
struct Manual(AtomicU64);
impl Clock for Manual {
    fn now(&self) -> Duration {
        ms(self.0.load(Ordering::Relaxed))
    }
}
struct Harness {
    clock: Arc<Manual>,
    reporter: Reporter,
    publisher: Publisher,
    check: Check,
    agg: Aggregator,
    observability: Arc<FaultObservability>,
}
impl Harness {
    fn new(policy: Policy) -> Self {
        let clock = Arc::new(Manual::default());
        let (reporter, publisher) =
            Reporter::with_clock(7, reporter::Config::default(), clock.clone()).unwrap();
        let mut definition = Definition::new("writer", "disk", "io").unwrap();
        definition.policy = policy;
        let check = reporter.register(definition).unwrap();
        let observability = Arc::new(FaultObservability::new());
        let mut agg = Aggregator::new(
            99,
            fault_aggregation::Config::default(),
            observability.clone(),
        )
        .unwrap();
        agg.reconcile(&publisher.snapshot(), ms(0)).unwrap();
        Self {
            clock,
            reporter,
            publisher,
            check,
            agg,
            observability,
        }
    }
    fn report(&mut self, time: u64, outcome: Outcome) -> Observation {
        self.clock.0.store(time, Ordering::Relaxed);
        self.check.observe(outcome, Text::EMPTY, None).unwrap();
        self.publisher.snapshot().conditions[0].latest.unwrap()
    }
    fn deliver(&mut self, o: Observation, at: u64) {
        self.agg.ingest(o, o.observed_at, ms(at)).unwrap();
    }
    fn health(&self) -> Health {
        self.agg.state(self.check.key()).unwrap().health
    }
}
#[test]
fn immediate_failure_one_second_recovery_and_timer_never_clears() {
    let mut h = Harness::new(Policy::default());
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    assert_eq!(h.health(), Health::Active);
    let o = h.report(10, Outcome::Success);
    h.deliver(o, 10);
    assert_eq!(h.health(), Health::Recovering);
    h.agg.advance(ms(1000)).unwrap();
    assert_eq!(h.health(), Health::Recovering);
    let o = h.report(1009, Outcome::Success);
    h.deliver(o, 1009);
    assert_eq!(h.health(), Health::Recovering);
    let o = h.report(1010, Outcome::Success);
    h.deliver(o, 1010);
    assert_eq!(h.health(), Health::Healthy);
    let count = h
        .agg
        .state(h.check.key())
        .unwrap()
        .latest
        .unwrap()
        .successes;
    assert_eq!(
        h.agg.ingest(o, o.observed_at, ms(1010)).unwrap(),
        Ingest::Duplicate
    );
    assert_eq!(
        h.agg
            .state(h.check.key())
            .unwrap()
            .latest
            .unwrap()
            .successes,
        count
    );
}
#[test]
fn intervening_failure_resets_recovery_even_when_not_delivered() {
    let mut h = Harness::new(Policy::default());
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    let o = h.report(10, Outcome::Success);
    h.deliver(o, 10);
    h.report(999, Outcome::Failure);
    let o = h.report(1010, Outcome::Success);
    h.deliver(o, 1010);
    assert_eq!(h.health(), Health::Recovering);
    let o = h.report(2010, Outcome::Success);
    h.deliver(o, 2010);
    assert_eq!(h.health(), Health::Healthy);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().latest.unwrap().failures,
        2
    );
}
#[test]
fn success_before_failure_preserves_history_without_reactivation() {
    let mut h = Harness::new(Policy {
        recovery: Duration::ZERO,
        ..Default::default()
    });
    let failure = h.report(0, Outcome::Failure);
    let success = h.report(1, Outcome::Success);
    h.deliver(success, 1);
    assert_eq!(h.health(), Health::Healthy);
    assert_eq!(h.agg.ingest(failure, ms(1), ms(1)).unwrap(), Ingest::Late);
    assert_eq!(h.health(), Health::Healthy);
    assert_eq!(
        h.agg.ingest(failure, ms(1), ms(1)).unwrap(),
        Ingest::Duplicate
    );
    let updates: Vec<_> = std::iter::from_fn(|| h.agg.pop_update()).collect();
    assert_eq!(
        updates
            .iter()
            .filter(|u| u.kind == UpdateKind::Late)
            .count(),
        1
    );
}
#[test]
fn optional_deadline_marks_stale_without_clearing() {
    let mut h = Harness::new(Policy {
        deadline: Some(ms(500)),
        ..Default::default()
    });
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    let o = h.report(1, Outcome::Success);
    h.deliver(o, 1);
    h.agg.advance(ms(502)).unwrap();
    assert_eq!(h.health(), Health::Recovering);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    let o = h.report(1100, Outcome::Success);
    h.deliver(o, 1100);
    assert_eq!(h.health(), Health::Recovering);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Fresh
    );
    let mut no_deadline = Harness::new(Policy::default());
    let o = no_deadline.report(0, Outcome::Failure);
    no_deadline.deliver(o, 0);
    no_deadline.agg.advance(ms(1_000_000)).unwrap();
    assert_eq!(
        no_deadline
            .agg
            .state(no_deadline.check.key())
            .unwrap()
            .freshness,
        Freshness::Fresh
    );
}
#[test]
fn snapshots_and_delayed_batches_never_refresh_old_evidence() {
    let mut h = Harness::new(Policy {
        deadline: Some(ms(500)),
        ..Default::default()
    });
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    h.clock.0.store(1000, Ordering::Relaxed);
    h.agg.reconcile(&h.publisher.snapshot(), ms(1000)).unwrap();
    h.agg.advance(ms(1000)).unwrap();
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    let o = h.report(1001, Outcome::Success);
    h.agg.ingest(o, ms(2000), ms(2000)).unwrap();
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
}
#[test]
fn restart_and_loss_reconciliation_rebuilds_state() {
    let mut h = Harness::new(Policy::default());
    for t in 0..300 {
        h.report(t, Outcome::Failure);
    }
    let snapshot = h.publisher.snapshot();
    assert!(snapshot.losses.failures > 0);
    let mut fresh = Aggregator::new(
        100,
        fault_aggregation::Config::default(),
        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
    )
    .unwrap();
    fresh.reconcile(&snapshot, ms(500)).unwrap();
    assert_eq!(fresh.state(h.check.key()).unwrap().health, Health::Active);
    assert_eq!(
        fresh.snapshot().sources[0].losses.failures,
        h.reporter.losses().failures
    );
    assert_eq!(fresh.snapshot().epoch, 100);
}
#[test]
fn event_only_never_creates_active_condition() {
    let mut h = Harness::new(Policy {
        mode: Mode::EventOnly,
        ..Default::default()
    });
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    assert_eq!(h.health(), Health::Unknown);
    assert!(std::iter::from_fn(|| h.agg.pop_update()).any(|u| u.kind == UpdateKind::Event));
}
#[test]
fn source_disconnect_and_explicit_retirement() {
    let mut h = Harness::new(Policy::default());
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    assert!(h.agg.retire_source(7).is_err());
    h.agg.set_connected(7, false, ms(1)).unwrap();
    assert_eq!(h.health(), Health::Active);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    h.agg.set_connected(7, true, ms(2)).unwrap();
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    h.agg.set_connected(7, false, ms(3)).unwrap();
    h.agg.retire_source(7).unwrap();
    assert!(h.agg.snapshot().sources.is_empty());
}
#[test]
fn bounded_output_gaps_are_visible_and_snapshots_remain_complete() {
    let mut h = Harness::new(Policy::default());
    h.agg = Aggregator::new(
        1,
        fault_aggregation::Config {
            output_capacity: 2,
            ..Default::default()
        },
        h.observability.clone(),
    )
    .unwrap();
    h.agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    for t in 0..10 {
        let o = h.report(t, Outcome::Failure);
        h.deliver(o, t);
    }
    assert_eq!(h.observability.stats().dropped_updates, 9);
    assert_eq!(
        h.agg.snapshot().sources[0].conditions[0]
            .latest
            .unwrap()
            .failures,
        10
    );
    assert_eq!(std::iter::from_fn(|| h.agg.pop_update()).count(), 2);
}
#[test]
fn input_bounds_conflicts_clock_and_invalid_observations() {
    let mut h = Harness::new(Policy::default());
    h.agg = Aggregator::new(
        1,
        fault_aggregation::Config {
            max_sources: 1,
            max_conditions_per_source: 1,
            ..Default::default()
        },
        h.observability.clone(),
    )
    .unwrap();
    h.agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    let def = Definition::new("other", "", "code").unwrap();
    assert_eq!(
        h.agg.register(
            Key {
                source: 7,
                condition: 1
            },
            def,
            ms(0)
        ),
        Err(fault_aggregation::Error::Capacity)
    );
    assert_eq!(
        h.agg.register(
            Key {
                source: 8,
                condition: 0
            },
            def,
            ms(0)
        ),
        Err(fault_aggregation::Error::Capacity)
    );
    assert_eq!(
        h.agg.register(h.check.key(), def, ms(0)),
        Err(fault_aggregation::Error::DefinitionConflict)
    );
    let mut o = h.report(0, Outcome::Failure);
    o.failures = 2;
    assert_eq!(
        h.agg.ingest(o, ms(0), ms(0)),
        Err(fault_aggregation::Error::InvalidObservation)
    );
    assert_eq!(h.health(), Health::Unknown);
    h.agg.advance(ms(1)).unwrap();
    assert_eq!(
        h.agg.advance(ms(0)),
        Err(fault_aggregation::Error::ClockWentBackwards)
    );
}
#[test]
fn old_duplicates_are_bounded_and_do_not_generate_new_history() {
    let mut h = Harness::new(Policy::default());
    h.agg = Aggregator::new(
        1,
        fault_aggregation::Config {
            dedup_window: 2,
            ..Default::default()
        },
        h.observability.clone(),
    )
    .unwrap();
    h.agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    let old = h.report(0, Outcome::Failure);
    h.deliver(old, 0);
    for t in 1..4 {
        let o = h.report(t, Outcome::Failure);
        h.deliver(o, t);
    }
    assert_eq!(h.agg.ingest(old, ms(4), ms(4)).unwrap(), Ingest::TooOld);
    assert_eq!(h.observability.stats().too_old, 1);
}
#[test]
fn rate_limited_evidence_can_clear_but_idle_flush_alone_cannot() {
    let mut h = Harness::new(Policy {
        success_interval: Some(ms(2000)),
        ..Default::default()
    });
    h.report(0, Outcome::Failure);
    h.report(1, Outcome::Success);
    h.agg
        .ingest_batch(&h.publisher.batch(10).unwrap(), ms(1))
        .unwrap();
    h.report(500, Outcome::Success);
    h.clock.0.store(2001, Ordering::Relaxed);
    h.agg
        .ingest_batch(&h.publisher.batch(10).unwrap(), ms(2001))
        .unwrap();
    assert_eq!(h.health(), Health::Recovering);
    h.report(2002, Outcome::Success);
    h.clock.0.store(4001, Ordering::Relaxed);
    h.agg
        .ingest_batch(&h.publisher.batch(10).unwrap(), ms(4001))
        .unwrap();
    assert_eq!(h.health(), Health::Healthy);
}

#[test]
fn recovery_evidence_survives_deadline_gap_and_aggregator_restart() {
    let mut h = Harness::new(Policy {
        deadline: Some(ms(600)),
        ..Default::default()
    });
    for (t, outcome) in [
        (0, Outcome::Failure),
        (1, Outcome::Success),
        (501, Outcome::Success),
        (1001, Outcome::Success),
        (3000, Outcome::Success),
    ] {
        let o = h.report(t, outcome);
        h.deliver(o, t);
    }
    assert_eq!(h.health(), Health::Healthy);
    let mut restarted = Aggregator::new(
        1000,
        fault_aggregation::Config::default(),
        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
    )
    .unwrap();
    restarted
        .reconcile(&h.publisher.snapshot(), ms(3000))
        .unwrap();
    assert_eq!(
        restarted.state(h.check.key()).unwrap().health,
        Health::Healthy
    );
    let o = h.report(3001, Outcome::Failure);
    h.deliver(o, 3001);
    let o = h.report(3002, Outcome::Success);
    h.deliver(o, 3002);
    assert_eq!(h.health(), Health::Recovering);
}

#[test]
fn transport_allowance_and_unobserved_deadline_are_explicit() {
    let mut h = Harness::new(Policy {
        deadline: Some(ms(100)),
        ..Default::default()
    });
    h.agg.advance(ms(101)).unwrap();
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    h.agg = Aggregator::new(
        1,
        fault_aggregation::Config {
            transport_allowance: ms(101),
            ..Default::default()
        },
        h.observability.clone(),
    )
    .unwrap();
    h.agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    let o = h.report(0, Outcome::Success);
    h.deliver(o, 0);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
}

#[test]
fn all_delivery_permutations_converge_on_latest_evidence() {
    let mut h = Harness::new(Policy::default());
    let records = [
        h.report(0, Outcome::Failure),
        h.report(1, Outcome::Success),
        h.report(1001, Outcome::Success),
        h.report(1002, Outcome::Failure),
    ];
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let order = [a, b, c, d];
                    if (0..4).any(|i| order[..i].contains(&order[i])) {
                        continue;
                    }
                    let mut agg = Aggregator::new(
                        1,
                        fault_aggregation::Config::default(),
                        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
                    )
                    .unwrap();
                    agg.register(
                        h.check.key(),
                        h.publisher.snapshot().conditions[0].definition,
                        ms(0),
                    )
                    .unwrap();
                    for i in order {
                        agg.ingest(records[i], ms(1002), ms(1002)).unwrap();
                    }
                    assert_eq!(agg.state(h.check.key()).unwrap().health, Health::Active);
                    assert_eq!(agg.state(h.check.key()).unwrap().latest, Some(records[3]));
                }
            }
        }
    }
}

#[test]
fn new_source_instance_has_independent_sequences_and_health() {
    let mut h = Harness::new(Policy::default());
    let o = h.report(0, Outcome::Failure);
    h.deliver(o, 0);
    h.agg.set_connected(7, false, ms(1)).unwrap();
    let (reporter, mut publisher) = Reporter::new(8, reporter::Config::default()).unwrap();
    let check = reporter
        .register(Definition::new("writer", "disk", "io").unwrap())
        .unwrap();
    h.agg.reconcile(&publisher.snapshot(), ms(2)).unwrap();
    check.success().unwrap();
    h.agg
        .ingest_batch(&publisher.batch(1).unwrap(), ms(3))
        .unwrap();
    assert_eq!(h.agg.state(check.key()).unwrap().health, Health::Healthy);
    assert_eq!(h.agg.state(h.check.key()).unwrap().health, Health::Active);
    assert_eq!(
        h.agg.state(h.check.key()).unwrap().freshness,
        Freshness::Stale
    );
    assert_eq!(h.agg.snapshot().sources.len(), 2);
}

#[test]
fn output_signals_deadlines_and_overflow_reconciliation() {
    let mut h = Harness::new(Policy {
        deadline: Some(ms(100)),
        ..Default::default()
    });
    let signal = h.agg.notification();
    while h.agg.pop_update().is_some() {}
    let before = signal.generation();
    let failure = h.report(0, Outcome::Failure);
    h.deliver(failure, 0);
    assert!(signal.generation() > before);
    assert!(h.agg.has_updates());
    assert_eq!(
        h.agg.next_deadline(),
        Some(ms(100) + Duration::from_nanos(1))
    );
    while h.agg.pop_update().is_some() {}
    let before = signal.generation();
    h.agg.advance(h.agg.next_deadline().unwrap()).unwrap();
    assert!(signal.generation() > before);
    assert_eq!(h.health(), Health::Active);
    assert_eq!(h.agg.pop_update().unwrap().kind, UpdateKind::Freshness);
    assert_eq!(h.agg.next_deadline(), None);

    let mut agg = Aggregator::new(
        5,
        fault_aggregation::Config {
            output_capacity: 1,
            ..Default::default()
        },
        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
    )
    .unwrap();
    agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    assert!(agg.take_reconciliation().is_some()); // registration + observation overflow
    assert!(agg.take_reconciliation().is_none());
    assert_eq!(agg.snapshot().stats.dropped_updates, 1);
}

#[test]
fn injected_diagnostics_survive_aggregator_and_keep_snapshots_stable() {
    let mut h = Harness::new(Policy::default());
    let observability = Arc::new(FaultObservability::new());
    let mut agg = Aggregator::new(
        123,
        fault_aggregation::Config {
            output_capacity: 1,
            ..Default::default()
        },
        observability.clone(),
    )
    .unwrap();
    agg.reconcile(&h.publisher.snapshot(), ms(0)).unwrap();
    let captured = agg.snapshot();
    let failure = h.report(1, Outcome::Failure);
    agg.ingest(failure, ms(1), ms(1)).unwrap();
    let mut invalid = failure;
    invalid.sequence = 0;
    assert_eq!(
        agg.ingest(invalid, ms(1), ms(1)),
        Err(fault_aggregation::Error::InvalidObservation)
    );
    let expected = Stats {
        dropped_updates: 1,
        rejected_inputs: 1,
        too_old: 0,
    };
    assert_eq!(observability.stats(), expected);
    assert_eq!(agg.snapshot().stats, expected);
    assert_eq!(captured.stats, Stats::default());
    let isolated = Arc::new(FaultObservability::new());
    let _other =
        Aggregator::new(124, fault_aggregation::Config::default(), isolated.clone()).unwrap();
    assert_eq!(isolated.stats(), Stats::default());
    drop(agg);
    assert_eq!(observability.stats(), expected);
}
