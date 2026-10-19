# Activity tracing

Embedded activity recording with `tracing`, Crossbeam, Protobuf, MCAP and optional
WebSocket / OpenTelemetry outputs.

- [Architecture](docs/architecture.svg)
- [Buffer lifetimes](docs/buffer-lifetimes.svg)
- [Roadmap and validation gates](docs/101826-my-roadmpa.md)
- [Validation evidence](docs/validation.md)
- [Wire schema](proto/activity.proto)

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

`Runtime` owns the encoder and one worker per sink. Applications initialize it
once and retain its handle. No separate recorder process is required.

## Component boundary

This crate owns the subscriber, bounded capture/encoded buffer pools, wire schema,
background encoder and reusable MCAP/WebSocket/OpenTelemetry sinks. Applications
provide `Config`, install the subscriber, register their emitting threads, and
own runtime shutdown. The crate builds and runs its tests independently of the
demo; it has no dependency on app code or frontend assets.

Custom transports implement `Sink` and consume `BatchView` on an output worker.
`ActivityContext` carries same-runtime message causality; `Handle::buffers()`
provides read-only monitoring snapshots. Worker simulations, scheduling, control
APIs and viewer retention policies belong to their host application.

## Capture contract

Register each emitting OS thread before capture, up to `max_threads`, including every async executor
worker. Registration guards remain on their registering threads. Set the
subscriber globally or use a `Dispatch` shared across threads. Instrument async
futures with `tracing::Instrument`; synchronous enter guards end before await.
Names beginning with the `activity::` target namespace, and the exact `activity`
target, are captured. Logical task identity is an explicit field such as
`task.name`; each record also carries Linux TID and the registered thread name.

The subscriber uses a preallocated span table, bounded thread-local context
stacks, and a short standard-library mutex for span bookkeeping. This is a
synchronous critical section subject to contention. Buffer acquisition and
publication never wait for output capacity. No output I/O runs in callbacks.
This initial bounded subscriber owns span tracking; it is installed as a
`Subscriber`. A composable `Layer` adapter would need its registry allocations
validated separately.

Initialization and thread registration allocate. The allocation test covers
warmed callsites, span creation/cloning/close, context entry/exit, numeric fields,
string fields, primitive Debug formatting, and pool exhaustion. Callers can
allocate while computing attributes or implementing `Debug`; those operations
remain caller-owned. Stack overflow suppresses contextual parenting until nested
enter guards unwind in LIFO order. Explicit parent IDs remain available.

Each record holds up to eight fields. String values are copied into packed block
storage and capped at 256 UTF-8 bytes per field. Exhausted string space truncates
at a UTF-8 boundary and increments field loss. Metadata and thread names are
limited to 256 bytes; unsupported callsite metadata is filtered. Repeated static
names appear once in each batch dictionary. No per-record `String` is created on
application threads.

`flush_events=true` provides immediate publication, including standalone events.
Set it to false to batch event-heavy workloads. Full blocks, span lifecycle
records, span exit, explicit `Registration::flush()`, and registration drop publish
partial blocks. With batching enabled, call `flush()` before an idle period.
An idle producer's unpublished partial block has no background flush deadline.

## Output and memory bounds

Crossbeam `ArrayQueue` moves capture-block ownership between free and ready pools.
The encoder reuses scratch buffers and writes into preallocated encoded buffers.
Crossbeam bounded channels carry leases; workers borrow immutable batch bytes.
The last lease returns its buffer to the pool. Each sink has its own capacity,
worker and drop counter. Queue-full behavior drops that sink's incoming batch.

Configuration requires enough encoded buffers for all output queues, one in-flight
batch per output, and the encoder. This reserves progress capacity while slow
outputs hold their bounded leases. Capture pool exhaustion drops records. Span
capacity exhaustion increments rejected spans. `ShutdownReport` contains final
capture losses, per-sink dropped/written batches, and sink errors. Batch sequence
gaps expose downstream losses; every batch also carries cumulative producer loss
counters. Sequence reflects publication order; timestamps retain event time.

The core's retained storage scales with `blocks × (records_per_block × record
size + string_bytes_per_block)`, plus `encoded_buffers × (4096 ×
records_per_block + string_bytes_per_block + 1024)` and bounded span/context
storage (`max_threads × max_depth` context entries). These are conservative encoder bounds. OS thread stacks, MCAP indexes,
OTel SDK allocations and transport buffers add memory outside these pools.
MCAP indexing can grow with recording length; file rotation is an application
policy in this version.

## Sinks

`McapSink` uses the established `mcap` crate directly with this component's schema.
It embeds the FileDescriptorSet and preserves original encoded payloads and
capture timestamps. Creation refuses existing files. Finalization flushes the
writer; filesystem durability across power loss requires additional policy.
The existing CPU-specific `telemetry-mcap::McapSink` accepts CPU envelopes, so
activity schema/channel ownership stays in this component.

With `websocket`, `WebSocketSink::accept(stream, timeout)` performs a server
handshake during setup. Each binary frame contains one self-contained batch.
Create one sink per preconnected viewer. The caller owns listening, authorization,
TLS termination and reconnect policy. Tungstenite copies the frame payload on its
worker. I/O timeouts bound network stalls. Dynamic viewer attachment is future work.

With `otel`, `OtelSink::http(endpoint, timeout, max_active_spans)` uses the official
OpenTelemetry SDK exporter interface and OTLP HTTP protobuf exporter. A custom
SDK `SpanExporter` can be passed to `OtelSink::new`. It must run correctly on a
synchronous worker and bound its own I/O. Trace IDs combine the random session ID
and root span ID; span IDs match the recording exactly. Links, fields, parentage,
span events and timestamps are preserved. Standalone events with no span are
counted as orphan records; a separate OTel logs adapter is future work.

Assembly bounds active spans, attributes (64), events (128) and links (64).
Evicted or unfinished spans export with `activity.incomplete=true`; sequence gaps
mark active spans. `OtelStats` exposes incomplete, orphan and gap counts. OTel
conversion/serialization and MCAP/network libraries may allocate on their workers.
No whole-process allocation-free guarantee is made.

Drop every registration guard, then call `shutdown()` to drain and finalize.
Sink errors are returned in the report. Custom sink implementations must finish
within an application-appropriate timeout. Disk writes may block on the OS;
shutdown waits for them. `Drop` stops workers as a fallback; explicit shutdown
provides error reporting and checks that producer registrations have ended.

## Run

```sh
cargo test -p activity-tracing --all-features
cargo run -p activity-tracing --example record -- /tmp/activity.mcap
cargo run -p activity-tracing --release --example throughput -- 100000
cargo run -p activity-tracing --release --example throughput -- 100000 1000
```

The benchmark reports wall time, encoded bytes and losses. Its optional second
argument adds a microsecond sleep after each 64 attempted events; wall time includes
that pacing. One warm-up event is included in encoded bytes. Compare loss rates
alongside throughput. Thor memory-bandwidth behavior requires a run on Thor.

## Pub/sub example

The [Activity lab](../../apps/activity-demo/) runs four worker threads with a live
TypeScript viewer and embedded MCAP recording. Capture a copied token with
`handle.context(&publish_span)`, carry it in the message, then create the receive
span inside `handle.with_parent(context, || { ... })`. Tokens preserve parent and
trace identity after the publisher closes. They are scoped to one runtime and a
synchronous callback on a registered thread.

`handle.buffers()` samples free/ready capture blocks, free encoded buffers,
registered threads and configured payload capacities. Concurrent queue lengths
are approximate, independently sampled values. Read snapshots from a monitoring
thread; no metrics updates are added to producer capture callbacks.
