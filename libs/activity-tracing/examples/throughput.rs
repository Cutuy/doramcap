//! Reports wall time (including optional pacing), encoded bytes, and losses.
use activity_tracing::{BatchView, Config, Result, Runtime, Sink};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
struct Count(Arc<AtomicU64>);
impl Sink for Count {
    fn name(&self) -> &str {
        "count"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        self.0.fetch_add(b.bytes.len() as u64, Ordering::Relaxed);
        Ok(())
    }
}
fn emit(value: u64) {
    tracing::info!(target:"activity",value,text="frame ready");
}
fn main() -> Result<()> {
    let count = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(100_000u64);
    if count == 0 {
        return Err("event count must be nonzero".into());
    }
    let pace_us = std::env::args()
        .nth(2)
        .map(|s| s.parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    let bytes = Arc::new(AtomicU64::new(0));
    let mut runtime = Runtime::start(
        Config {
            flush_events: false,
            ..Default::default()
        },
        vec![Box::new(Count(bytes.clone()))],
    )?;
    let h = runtime.handle();
    let reg = h.register_thread("benchmark")?;
    let dispatch = tracing::Dispatch::new(h.subscriber());
    let _default = tracing::dispatcher::set_default(&dispatch);
    emit(0);
    reg.flush();
    let start = Instant::now();
    for n in 0..count {
        emit(n);
        if pace_us > 0 && n % 64 == 63 {
            std::thread::sleep(std::time::Duration::from_micros(pace_us));
        }
    }
    reg.flush();
    let elapsed = start.elapsed();
    drop(reg);
    let report = runtime.shutdown()?;
    println!(
        "events={count} pace_us_per_64={pace_us} wall_ns={} wall_ns_per_event={:.1} encoded_bytes={} losses={:?} outputs={:?}",
        elapsed.as_nanos(),
        elapsed.as_nanos() as f64 / count as f64,
        bytes.load(Ordering::Relaxed),
        report.capture,
        report.sinks
    );
    Ok(())
}
