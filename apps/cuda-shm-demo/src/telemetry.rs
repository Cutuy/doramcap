//! Persist original activity Protobuf batches for binary live relay and MCAP.
//! Framing: little-endian u32 length followed by the unchanged batch bytes.
use activity_tracing::{BatchView, Config, Result, Runtime, Sink, sinks::McapSink};
use std::{fs::File, io::Write, path::Path};

struct LiveSink(File);
impl Sink for LiveSink {
    fn name(&self) -> &str {
        "live-protobuf"
    }
    fn write(&mut self, batch: BatchView<'_>) -> Result<()> {
        let length = u32::try_from(batch.bytes.len())?;
        self.0.write_all(&length.to_le_bytes())?;
        self.0.write_all(batch.bytes)?;
        Ok(())
    }
}
pub fn start(dir: &Path, role: &str) -> Result<Runtime> {
    std::fs::create_dir_all(dir)?;
    let stem = format!("{role}-{}", std::process::id());
    let live = LiveSink(
        File::options()
            .create_new(true)
            .write(true)
            .open(dir.join(format!("{stem}.activity.pb")))?,
    );
    Runtime::start(
        Config {
            flush_events: false,
            ..Config::default()
        },
        vec![
            Box::new(live),
            Box::new(McapSink::create(dir.join(format!("{stem}.mcap")))?),
        ],
    )
}

/// Publish partial batches every 50 ms while the demo loop is active.
/// Full batches and registration drop also flush through activity-tracing.
pub fn flush_due(
    registration: Option<&activity_tracing::Registration>,
    last: &mut std::time::Instant,
) {
    if last.elapsed() >= std::time::Duration::from_millis(50) {
        if let Some(registration) = registration {
            registration.flush();
        }
        *last = std::time::Instant::now();
    }
}
