use fault_aggregation::{Update, UpdateKind, View};
use fault_demo::Display;
use fault_protocol::{AggregationFrame, AggregationPayload, Updates};
use std::time::Duration;
fn frame(payload: AggregationPayload) -> AggregationFrame {
    AggregationFrame {
        published_unix_ns: None,
        emitted_at: Duration::ZERO,
        payload,
    }
}
#[test]
fn gaps_snapshots_duplicates_and_epoch_reset() {
    let mut d = Display::default();
    d.apply(frame(AggregationPayload::Snapshot(View {
        epoch: 1,
        revision: 5,
        stats: Default::default(),
        sources: vec![],
    })));
    assert_eq!(d.json()["complete"], true);
    let u = Update {
        epoch: 1,
        revision: 7,
        kind: UpdateKind::Source,
        key: None,
        state: None,
        observation: None,
        source: 1,
        connected: true,
        losses: Default::default(),
    };
    let f = frame(AggregationPayload::Updates(Updates {
        epoch: 1,
        updates: vec![u],
        stats: Default::default(),
    }));
    d.apply(f.clone());
    d.apply(f);
    assert_eq!(d.json()["complete"], false);
    assert_eq!(d.json()["history"].as_array().unwrap().len(), 1);
    d.apply(frame(AggregationPayload::Snapshot(View {
        epoch: 1,
        revision: 7,
        stats: Default::default(),
        sources: vec![],
    })));
    assert_eq!(d.json()["complete"], true);
    d.apply(frame(AggregationPayload::Snapshot(View {
        epoch: 2,
        revision: 1,
        stats: Default::default(),
        sources: vec![],
    })));
    assert_eq!(d.json()["revision"], "1");
    assert!(d.json()["history"].as_array().unwrap().is_empty());
}

#[derive(Default)]
struct Clock(std::sync::atomic::AtomicU64);
impl fault_reporter::Clock for Clock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(std::sync::atomic::Ordering::Relaxed))
    }
}
#[test]
fn timeline_retains_hysteresis_and_never_advances_evidence_on_silence() {
    let clock = std::sync::Arc::new(Clock::default());
    let (reporter, publisher) =
        fault_reporter::Reporter::with_clock(1, Default::default(), clock.clone()).unwrap();
    let check = reporter
        .register(fault_reporter::Definition::new("demo", "a", "operation").unwrap())
        .unwrap();
    let mut aggregator =
        fault_aggregation::Aggregator::new(2, Default::default(), Default::default()).unwrap();
    let mut display = Display::default();
    for (ms, fail) in [
        (0, false),
        (100, true),
        (300, false),
        (800, false),
        (1300, false),
    ] {
        clock.0.store(ms, std::sync::atomic::Ordering::Relaxed);
        if fail {
            check
                .failure(fault_reporter::Text::new("failure").unwrap())
                .unwrap();
        } else {
            check.success().unwrap();
        }
        let now = Duration::from_millis(ms);
        aggregator.reconcile(&publisher.snapshot(), now).unwrap();
        let mut updates = Vec::new();
        while let Some(u) = aggregator.pop_update() {
            updates.push(u);
        }
        display.apply(AggregationFrame {
            published_unix_ns: None,
            emitted_at: now,
            payload: AggregationPayload::Updates(Updates {
                epoch: 2,
                updates,
                stats: Default::default(),
            }),
        });
        if ms == 800 {
            let first = display.json()["conditions"][0]["recovery_evidence_ms"].clone();
            assert_eq!(first, 500.0);
            display.apply(AggregationFrame {
                published_unix_ns: None,
                emitted_at: Duration::from_millis(1000),
                payload: AggregationPayload::Snapshot(aggregator.snapshot()),
            });
            assert_eq!(
                display.json()["conditions"][0]["recovery_evidence_ms"],
                first
            );
        }
    }
    let data = display.json();
    let condition = &data["conditions"][0];
    let states: Vec<_> = condition["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["health"].as_str().unwrap())
        .collect();
    assert!(
        states
            .windows(3)
            .any(|s| s == ["HEALTH_ACTIVE", "HEALTH_RECOVERING", "HEALTH_HEALTHY"])
    );
    assert_eq!(condition["recovery_ms"], 1000.0);
    let reports = condition["reports"].as_array().unwrap();
    assert_eq!(
        reports
            .iter()
            .map(|p| p["successes"].as_str().unwrap().parse::<u64>().unwrap())
            .sum::<u64>(),
        4
    );
    assert_eq!(
        reports
            .iter()
            .map(|p| p["failures"].as_str().unwrap().parse::<u64>().unwrap())
            .sum::<u64>(),
        1
    );
    display.command("a", "pause");
    assert_eq!(display.json()["commands"][0]["mode"], "pause");
    // Snapshots with duplicate evidence do not manufacture new report marks.
    let count = reports.len();
    display.apply(AggregationFrame {
        published_unix_ns: None,
        emitted_at: Duration::from_secs(2),
        payload: AggregationPayload::Snapshot(aggregator.snapshot()),
    });
    assert_eq!(
        display.json()["conditions"][0]["reports"]
            .as_array()
            .unwrap()
            .len(),
        count
    );
}
