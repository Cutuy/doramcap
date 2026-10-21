use fault_reporter::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
#[derive(Default)]
struct Manual(AtomicU64);
impl Manual {
    fn set(&self, ms: u64) {
        self.0.store(ms, Ordering::Relaxed);
    }
}
impl Clock for Manual {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::Relaxed))
    }
}
fn setup(config: Config, policy: Policy) -> (Arc<Manual>, Reporter, Publisher, Check) {
    let clock = Arc::new(Manual::default());
    let (reporter, publisher) = Reporter::with_clock(1, config, clock.clone()).unwrap();
    let mut definition = Definition::new("writer", "disk", "io").unwrap();
    definition.policy = policy;
    let check = reporter.register(definition).unwrap();
    (clock, reporter, publisher, check)
}
#[test]
fn separate_queues_and_latest_state_survive_overflow() {
    let (_, reporter, mut publisher, check) = setup(
        Config {
            failure_capacity: 1,
            success_capacity: 1,
            repeat_capacity: 1,
            ..Default::default()
        },
        Policy::default(),
    );
    assert_eq!(check.success().unwrap(), Admission::Queued);
    assert_eq!(check.success().unwrap(), Admission::Queued);
    assert_eq!(check.success().unwrap(), Admission::Dropped);
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Queued);
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Queued);
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Dropped);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Failure);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Failure);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Success);
    let latest = publisher.snapshot().conditions[0].latest.unwrap();
    assert_eq!(
        (latest.sequence, latest.failures, latest.successes),
        (6, 3, 3)
    );
    assert_eq!(
        reporter.losses(),
        Losses {
            failures: 1,
            successes: 1,
            coalesced_successes: 0
        }
    );
}
#[test]
fn failures_have_bounded_priority() {
    let (_, _, mut publisher, check) = setup(
        Config {
            failure_burst: 2,
            ..Default::default()
        },
        Policy::default(),
    );
    check.success().unwrap();
    for _ in 0..8 {
        check.failure(Text::EMPTY).unwrap();
    }
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Failure);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Failure);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Success);
}
#[test]
fn rate_limit_preserves_first_recovery_and_idle_summary_timestamp() {
    let (clock, reporter, mut publisher, check) = setup(
        Config::default(),
        Policy {
            success_interval: Some(Duration::from_secs(1)),
            ..Default::default()
        },
    );
    check.success().unwrap();
    publisher.try_recv().unwrap();
    clock.set(100);
    assert_eq!(check.success().unwrap(), Admission::Coalesced);
    clock.set(200);
    check.failure(Text::EMPTY).unwrap();
    clock.set(300);
    assert_eq!(check.success().unwrap(), Admission::Queued);
    clock.set(500);
    assert_eq!(check.success().unwrap(), Admission::Coalesced);
    while publisher.try_recv().is_some() {}
    clock.set(1299);
    publisher.tick();
    assert!(publisher.try_recv().is_none());
    clock.set(1300);
    publisher.tick();
    let record = publisher.try_recv().unwrap();
    assert_eq!(record.observed_at, Duration::from_millis(500));
    assert_eq!(record.success_since, Some(Duration::from_millis(300)));
    assert_eq!(record.consecutive_successes, 2);
    assert_eq!(reporter.losses().coalesced_successes, 2);
    publisher.tick();
    assert!(publisher.try_recv().is_none());
}
#[test]
fn deadline_gap_resets_success_run() {
    let (clock, _, publisher, check) = setup(
        Config::default(),
        Policy {
            deadline: Some(Duration::from_secs(1)),
            success_interval: Some(Duration::from_millis(500)),
            ..Default::default()
        },
    );
    check.success().unwrap();
    clock.set(1001);
    check.success().unwrap();
    let latest = publisher.snapshot().conditions[0].latest.unwrap();
    assert_eq!(latest.consecutive_successes, 1);
    assert_eq!(latest.success_since, Some(Duration::from_millis(1001)));
}
#[test]
fn registration_bounds_invalid_config_and_event_mode() {
    assert!(Reporter::new(0, Config::default()).is_err());
    assert!(
        Reporter::new(
            1,
            Config {
                failure_capacity: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let (_, reporter, _, check) = setup(
        Config {
            max_conditions: 1,
            ..Default::default()
        },
        Policy {
            mode: Mode::EventOnly,
            ..Default::default()
        },
    );
    assert_eq!(check.success(), Err(Error::WrongMode));
    check.failure(Text::new("offline").unwrap()).unwrap();
    assert!(matches!(
        reporter.register(Definition::new("writer", "other", "io").unwrap()),
        Err(Error::Capacity)
    ));
    assert!(matches!(
        reporter.register(Definition {
            policy: Policy {
                mode: Mode::EventOnly,
                ..Default::default()
            },
            ..Definition::new("writer", "disk", "io").unwrap()
        }),
        Err(Error::Duplicate)
    ));
    assert_eq!(
        Policy {
            deadline: Some(Duration::from_secs(1)),
            success_interval: Some(Duration::from_secs(1)),
            ..Default::default()
        }
        .validate(),
        Err(Error::InvalidConfig)
    );
    assert_eq!(Text::new(&"é".repeat(97)), Err(Error::TextTooLong));
    assert_eq!(Text::new(&"é".repeat(96)).unwrap().as_str().len(), 192);
}
#[test]
fn close_rejects_handles_and_retains_pending_state() {
    let (clock, reporter, publisher, check) = setup(
        Config::default(),
        Policy {
            success_interval: Some(Duration::from_secs(1)),
            ..Default::default()
        },
    );
    check.success().unwrap();
    clock.set(10);
    check.success().unwrap();
    assert!(publisher.final_snapshot().is_err());
    reporter.close();
    assert_eq!(check.success(), Err(Error::Closed));
    assert_eq!(
        publisher.final_snapshot().unwrap().conditions[0]
            .latest
            .unwrap()
            .sequence,
        2
    );
    assert!(matches!(
        reporter.register(Definition::new("x", "", "x").unwrap()),
        Err(Error::Closed)
    ));
}
#[test]
fn backwards_clock_does_not_mutate_state() {
    let (clock, _, publisher, check) = setup(Config::default(), Policy::default());
    clock.set(10);
    check.success().unwrap();
    clock.set(9);
    assert_eq!(check.failure(Text::EMPTY), Err(Error::ClockWentBackwards));
    assert_eq!(
        publisher.snapshot().conditions[0].latest.unwrap().sequence,
        1
    );
}
#[test]
fn concurrent_callers_preserve_counts_and_unique_sequences() {
    let (_, _, mut publisher, check) = setup(
        Config {
            failure_capacity: 4000,
            repeat_capacity: 4000,
            success_capacity: 4000,
            ..Default::default()
        },
        Policy::default(),
    );
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let check = check.clone();
            scope.spawn(move || {
                for _ in 0..1000 {
                    if worker % 2 == 0 {
                        check.success().unwrap();
                    } else {
                        check.failure(Text::EMPTY).unwrap();
                    }
                }
            });
        }
    });
    let last = publisher.snapshot().conditions[0].latest.unwrap();
    assert_eq!(
        (last.sequence, last.successes, last.failures),
        (4000, 2000, 2000)
    );
    let mut seen = std::collections::BTreeSet::new();
    let mut count = 0;
    while let Some(o) = publisher.try_recv() {
        assert!(seen.insert(o.sequence));
        count += 1;
    }
    assert_eq!(count, 4000);
}
#[test]
fn batches_are_bounded_and_carry_loss_counters() {
    let (_, _, mut publisher, check) = setup(
        Config {
            failure_capacity: 2,
            repeat_capacity: 1,
            success_capacity: 2,
            ..Default::default()
        },
        Policy::default(),
    );
    for _ in 0..3 {
        check.failure(Text::EMPTY).unwrap();
    }
    assert!(publisher.batch(0).is_err());
    assert!(publisher.batch(7).is_err());
    let batch = publisher.batch(1).unwrap();
    assert_eq!(batch.observations.len(), 1);
    assert_eq!(batch.losses.failures, 1);
    assert_eq!(publisher.batch(4).unwrap().observations.len(), 1);
}

#[test]
fn urgent_failure_and_recovery_bypass_repeat_backlog_and_limiter() {
    let (clock, _, mut publisher, check) = setup(
        Config {
            failure_capacity: 2,
            success_capacity: 2,
            repeat_capacity: 1,
            failure_burst: 1,
            ..Default::default()
        },
        Policy {
            success_interval: Some(Duration::from_secs(10)),
            ..Default::default()
        },
    );
    publisher.take_reconciliation().unwrap();
    check.failure(Text::EMPTY).unwrap();
    publisher.try_recv().unwrap();
    check.failure(Text::EMPTY).unwrap(); // fill repeats
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Dropped);
    check.success().unwrap(); // recovery starts
    clock.set(500);
    assert_eq!(check.success().unwrap(), Admission::Coalesced);
    clock.set(1000);
    let generation = publisher.notification().generation();
    assert_eq!(check.success().unwrap(), Admission::Queued); // recovery confirmed
    assert!(publisher.notification().generation() > generation);
    assert_eq!(publisher.try_recv().unwrap().sequence, 4);
    assert_eq!(publisher.try_recv().unwrap().outcome, Outcome::Failure);
    assert_eq!(publisher.try_recv().unwrap().sequence, 6);
    assert!(publisher.try_recv().is_none());
    assert!(publisher.reconciliation_pending());
    let snapshot = publisher.take_reconciliation().unwrap();
    assert_eq!(snapshot.losses.failures, 1);
    assert_eq!(snapshot.conditions[0].latest.unwrap().sequence, 6);
    assert!(!publisher.reconciliation_pending());
    // Repeat floods cannot consume urgent failure capacity.
    check.failure(Text::EMPTY).unwrap();
    check.failure(Text::EMPTY).unwrap();
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Dropped);
    assert_eq!(publisher.try_recv().unwrap().sequence, 7);
}

#[test]
fn event_only_overflow_requests_reconciliation_and_close_wakes() {
    let (_, reporter, mut publisher, check) = setup(
        Config {
            failure_capacity: 1,
            ..Default::default()
        },
        Policy {
            mode: Mode::EventOnly,
            ..Default::default()
        },
    );
    publisher.take_reconciliation().unwrap();
    check.failure(Text::EMPTY).unwrap();
    assert_eq!(check.failure(Text::EMPTY).unwrap(), Admission::Dropped);
    assert!(publisher.take_reconciliation().is_some());
    assert_eq!(publisher.try_recv().unwrap().sequence, 1);
    let signal = publisher.notification();
    let seen = signal.generation();
    reporter.close();
    assert_ne!(signal.generation(), seen);
    assert!(publisher.is_closed());
}

#[test]
fn reports_racing_with_wait_are_never_lost() {
    let (_, _, mut publisher, check) = setup(Config::default(), Policy::default());
    publisher.take_reconciliation().unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            for _ in 0..1000 {
                ready_rx.recv().unwrap();
                check.failure(Text::EMPTY).unwrap();
                done_rx.recv().unwrap();
            }
        });
        for sequence in 1..=1000 {
            ready_tx.send(()).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(1);
            loop {
                assert!(
                    std::time::Instant::now() < deadline,
                    "publisher slept through notification {sequence}"
                );
                if let Some(o) = publisher.try_recv() {
                    assert_eq!(o.sequence, sequence);
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "missed report {sequence}"
                );
                publisher.wait(Duration::from_secs(2));
            }
            done_tx.send(()).unwrap();
        }
    });
}

#[test]
fn summary_timer_flushes_idle_producer_without_refreshing_evidence() {
    let (reporter, mut publisher) = Reporter::new(1, Config::default()).unwrap();
    let mut d = Definition::new("writer", "", "io").unwrap();
    d.policy.success_interval = Some(Duration::from_millis(20));
    let check = reporter.register(d).unwrap();
    publisher.take_reconciliation();
    check.success().unwrap();
    publisher.try_recv().unwrap();
    check.success().unwrap();
    let observed = publisher.snapshot().conditions[0]
        .latest
        .unwrap()
        .observed_at;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        publisher.wait(Duration::from_secs(1));
        if let Some(o) = publisher.try_recv() {
            assert_eq!(o.observed_at, observed);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
    }
    assert!(publisher.next_summary_at().is_none());
}

#[test]
fn unspecified_generated_enums_are_rejected_by_core() {
    let (_, reporter, _, check) = setup(Config::default(), Policy::default());
    assert_eq!(
        check.observe(Outcome::Unspecified, Text::EMPTY, None),
        Err(Error::InvalidConfig)
    );
    let mut d = Definition::new("x", "", "x").unwrap();
    d.severity = Severity::Unspecified;
    assert!(matches!(reporter.register(d), Err(Error::InvalidConfig)));
    assert_eq!(
        Policy {
            mode: Mode::Unspecified,
            ..Default::default()
        }
        .validate(),
        Err(Error::InvalidConfig)
    );
}
