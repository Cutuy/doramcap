# Activity tracing

Embedded recording of `tracing` spans and events, with bounded capture buffers,
background encoding, and MCAP, WebSocket or OpenTelemetry outputs.

![Architecture](docs/architecture.svg)

[Design and bounds](docs/design.md) ·
[Buffer lifetimes](docs/buffer-lifetimes.svg) ·
[Proposed external-buffer ownership](docs/external-buffer-ownership.svg) ·
[Roadmap and validation gates](docs/101826-my-roadmpa.md) ·
[Validation evidence](docs/validation.md) ·
[Wire schema](proto/activity.proto)

## Quick start

```rust
use activity_tracing::{Config, Runtime, sinks::McapSink};

# fn main() -> activity_tracing::Result<()> {
let mut runtime = Runtime::start(
    Config::default(),
    vec![Box::new(McapSink::create("activity.mcap")?)],
)?;
let handle = runtime.handle();
let registration = handle.register_thread("camera")?;
tracing::subscriber::with_default(handle.subscriber(), || {
    let span = tracing::info_span!(
        target: "activity", "perception.infer",
        task.name = "camera_front", frame_id = 42u64,
    );
    let _entered = span.enter();
    tracing::info!(target: "activity", milestone = "input_ready");
});
drop(registration);
let report = runtime.shutdown()?;
assert!(report.sinks.iter().all(|sink| sink.error.is_none()));
# Ok(())
# }
```

Register every emitting thread, use the `activity` target, and drop registrations
before `shutdown()`. Outputs: `McapSink`, `WebSocketSink` (`websocket`) and `OtelSink` (`otel`).

## Run

```sh
cargo test -p activity-tracing --all-features
cargo run -p activity-tracing --example record -- /tmp/activity.mcap
cargo run -p activity-tracing --release --example throughput -- 100000
cargo run -p activity-tracing --release --example throughput -- 100000 1000
```

The [Activity lab](../../apps/activity-demo/) shows live capture and message causality.
