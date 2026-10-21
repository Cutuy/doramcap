//! In-process capture→encoded publication measurements. Excludes IPC and storage.
use fault_aggregation::{Aggregator, Config as AggregationConfig};
use fault_protocol::*;
use fault_reporter::{Clock, Config, Definition, Policy, Reporter, Text};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

struct Epoch(Instant);
impl Clock for Epoch {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}
fn percentile(samples: &mut [u128], fraction: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * fraction / 100]
}
fn run(name: &str, count: usize, paced: bool, idle: bool) {
    let clock = Arc::new(Epoch(Instant::now()));
    let (reporter, mut publisher) = Reporter::with_clock(
        1,
        Config {
            failure_capacity: 64,
            success_capacity: 64,
            repeat_capacity: 64,
            ..Default::default()
        },
        clock.clone(),
    )
    .unwrap();
    let mut definition = Definition::new("writer", "disk", "io").unwrap();
    definition.policy = Policy {
        recovery: Duration::ZERO,
        ..Default::default()
    };
    let check = reporter.register(definition).unwrap();
    let observability = std::sync::Arc::new(fault_aggregation::FaultObservability::new());
    let mut agg = Aggregator::new(2, AggregationConfig::default(), observability.clone()).unwrap();
    agg.reconcile(&publisher.take_reconciliation().unwrap(), clock.now())
        .unwrap();
    while agg.pop_update().is_some() {}
    let (ack_tx, ack_rx) = mpsc::channel();
    let worker_clock = clock.clone();
    let worker = std::thread::spawn(move || {
        let limits = Limits::default();
        let mut capture = Vec::new();
        let mut transition = Vec::new();
        loop {
            let batch = publisher.batch(128).unwrap();
            if !batch.observations.is_empty() {
                let frame = ReporterFrame {
                    published_unix_ns: None,
                    payload: ReporterPayload::Batch(batch),
                };
                let bytes = encode_reporter(&frame, limits).unwrap();
                let encoded_at = worker_clock.now();
                let ReporterPayload::Batch(batch) =
                    decode_reporter(&bytes, limits).unwrap().payload
                else {
                    unreachable!()
                };
                for o in &batch.observations {
                    capture.push(encoded_at.saturating_sub(o.observed_at).as_nanos());
                }
                let begun = Instant::now();
                agg.ingest_batch(&batch, worker_clock.now()).unwrap();
                let mut updates = Vec::new();
                while let Some(update) = agg.pop_update() {
                    updates.push(update);
                }
                let frame = AggregationFrame {
                    published_unix_ns: None,
                    emitted_at: worker_clock.now(),
                    payload: AggregationPayload::Updates(Updates {
                        epoch: 2,
                        updates,
                        stats: observability.stats(),
                    }),
                };
                let _ = encode_aggregation(&frame, limits).unwrap();
                transition.push(begun.elapsed().as_nanos());
                if idle {
                    ack_tx.send(()).unwrap();
                }
            }
            if let Some(snapshot) = publisher.take_reconciliation() {
                agg.reconcile(&snapshot, worker_clock.now()).unwrap();
                while agg.pop_update().is_some() {}
            }
            if publisher.is_closed() && !publisher.has_records() {
                break;
            }
            publisher.wait(Duration::from_secs(1));
        }
        (capture, transition)
    });
    for i in 0..count {
        if i % 2 == 0 {
            check.failure(Text::EMPTY).unwrap();
        } else {
            check.success().unwrap();
        }
        if idle {
            ack_rx.recv().unwrap();
        }
        if paced {
            std::thread::sleep(Duration::from_micros(100));
        }
    }
    reporter.close();
    let (mut capture, mut transition) = worker.join().unwrap();
    println!(
        "{name}: admitted={} drops={} capture_to_encoded_ns p50={} p99={} aggregation_to_encoded_ns p50={} p99={}",
        capture.len(),
        reporter.losses().failures + reporter.losses().successes,
        percentile(&mut capture, 50),
        percentile(&mut capture, 99),
        percentile(&mut transition, 50),
        percentile(&mut transition, 99)
    );
}
fn main() {
    run("idle", 1000, true, true);
    run("burst", 10_000, false, false);
    run("sustained", 2000, true, false);
}
