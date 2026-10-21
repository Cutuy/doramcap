//! In-process integration; applications replace the handoff with their transport.
use fault_aggregation::{Aggregator, Config as AggregationConfig};
use fault_reporter::{Clock, Config, Definition, Reporter, Text};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
#[derive(Default)]
struct DemoClock(AtomicU64);
impl Clock for DemoClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::Relaxed))
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clock = Arc::new(DemoClock::default());
    let (reporter, mut publisher) = Reporter::with_clock(1, Config::default(), clock.clone())?;
    let check = reporter.register(Definition::new("recorder", "disk", "write_failed")?)?;
    let mut aggregator = Aggregator::new(
        1,
        AggregationConfig::default(),
        std::sync::Arc::new(fault_aggregation::FaultObservability::new()),
    )?;
    aggregator.reconcile(&publisher.snapshot(), clock.now())?;
    check.failure(Text::new("disk unavailable")?)?;
    aggregator.ingest_batch(&publisher.batch(64)?, clock.now())?;
    clock.0.store(10, Ordering::Relaxed);
    check.success()?;
    aggregator.ingest_batch(&publisher.batch(64)?, clock.now())?;
    clock.0.store(1010, Ordering::Relaxed);
    check.success()?;
    aggregator.ingest_batch(&publisher.batch(64)?, clock.now())?;
    while let Some(update) = aggregator.pop_update() {
        if let Some(state) = update.state {
            println!(
                "{}: {:?} {:?}",
                update.revision, state.health, state.freshness
            );
        }
    }
    reporter.close();
    aggregator.reconcile(&publisher.final_snapshot()?, clock.now())?;
    Ok(())
}
