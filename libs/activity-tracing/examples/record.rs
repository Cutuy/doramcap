//! cargo run -p activity-tracing --example record -- /tmp/activity.mcap
use activity_tracing::{Config, Runtime, sinks::McapSink};
fn main() -> activity_tracing::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "activity.mcap".into());
    let mut runtime = Runtime::start(
        Config {
            sink_queue: 16,
            encoded_buffers: 32,
            ..Default::default()
        },
        vec![Box::new(McapSink::create(path)?)],
    )?;
    let handle = runtime.handle();
    let registration = handle.register_thread("camera")?;
    tracing::subscriber::with_default(handle.subscriber(), || {
        for frame in 0..3 {
            let span = tracing::info_span!(target:"activity","perception.infer",task.name="camera_front",frame);
            let _entered = span.enter();
            tracing::info!(target:"activity",milestone="input_ready");
        }
    });
    drop(registration);
    let report = runtime.shutdown()?;
    println!("{report:#?}");
    if report.capture.records != 0
        || report
            .sinks
            .iter()
            .any(|s| s.error.is_some() || s.dropped != 0)
    {
        return Err("recording failed".into());
    }
    Ok(())
}
