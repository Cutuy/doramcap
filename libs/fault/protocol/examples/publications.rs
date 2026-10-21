//! Writes sample Protobuf payloads and descriptors for consumers in any language.
use fault_aggregation::{Aggregator, Config as AggregationConfig};
use fault_protocol::*;
use fault_reporter::{Config, Definition, Reporter, Text};
use std::time::Duration;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or("usage: publications OUTPUT_DIRECTORY")?;
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory)?;
    let (reporter, mut publisher) = Reporter::new(1, Config::default())?;
    let check = reporter.register(Definition::new("recorder", "disk", "write_failed")?)?;
    check.failure(Text::new("disk unavailable")?)?;
    let registration = publisher.snapshot();
    let frame = ReporterFrame {
        published_unix_ns: Some(1_000_000_000),
        payload: ReporterPayload::Snapshot(registration),
    };
    let bytes = encode_reporter(&frame, Limits::default())?;
    std::fs::write(directory.join("reporter.pb"), &bytes)?;
    let ReporterPayload::Snapshot(decoded) = decode_reporter(&bytes, Limits::default())?.payload
    else {
        unreachable!()
    };
    let observability = std::sync::Arc::new(fault_aggregation::FaultObservability::new());
    let mut aggregator = Aggregator::new(2, AggregationConfig::default(), observability.clone())?;
    aggregator.reconcile(&decoded, Duration::ZERO)?;
    let updates = Updates {
        epoch: 2,
        updates: std::iter::from_fn(|| aggregator.pop_update()).collect(),
        stats: observability.stats(),
    };
    let frame = AggregationFrame {
        published_unix_ns: Some(1_000_000_000),
        emitted_at: Duration::ZERO,
        payload: AggregationPayload::Updates(updates),
    };
    std::fs::write(
        directory.join("aggregation.pb"),
        encode_aggregation(&frame, Limits::default())?,
    )?;
    std::fs::write(directory.join("fault_descriptor.bin"), FILE_DESCRIPTOR_SET)?;
    // Normal IPC sends decoded batches to Aggregator::ingest_batch.
    let batch = publisher.batch(32)?;
    let frame = ReporterFrame {
        published_unix_ns: None,
        payload: ReporterPayload::Batch(batch),
    };
    std::fs::write(
        directory.join("reporter-batch.pb"),
        encode_reporter(&frame, Limits::default())?,
    )?;
    println!("wrote reporter.pb, reporter-batch.pb, aggregation.pb and fault_descriptor.bin");
    Ok(())
}
