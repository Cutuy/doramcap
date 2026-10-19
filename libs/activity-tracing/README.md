# Activity tracing

Embedded recording of `tracing` spans and events, with bounded capture buffers,
background encoding, and MCAP, WebSocket or OpenTelemetry outputs.

![Architecture](docs/architecture.svg)

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


## Ownership

| Component | Responsibilities |
|---|---|
| `Runtime` | Subscriber, buffer pools, Protobuf encoding, one worker per sink, shutdown |
| Application | Configuration, subscriber installation, thread registration, scheduling, viewer policy, file rotation |
| `Sink` | Consume `BatchView` on an output worker; finalize output |
| `ActivityContext` | Carry message causality within one runtime |
| `Handle::buffers()` | Read buffer and registration snapshots |

Applications initialize the embedded runtime once and retain its handle. The
crate builds and tests independently of the demo.

## Capture

- Register every emitting OS thread, including async executor workers, within
  `max_threads`. Keep each registration guard on its registering thread.
- Install the subscriber globally or share a `Dispatch` across threads.
- Use `tracing::Instrument` for async futures. End synchronous enter guards before
  awaiting.
- Use the exact target `activity` or the `activity::` namespace.
- Supply logical task identity through a field such as `task.name`. Records also
  carry Linux TID and registered thread name.

### Bounds and allocation

The `Subscriber` owns a preallocated span table and bounded thread-local context
stacks. Span bookkeeping uses a short standard-library mutex and can contend.
Buffer acquisition and publication proceed without waiting for output capacity;
output I/O runs on sink workers. A future composable `Layer` adapter requires
separate validation of registry allocations.

Initialization and thread registration allocate. Allocation tests cover warmed
callsites, span creation/cloning/close, context entry/exit, numeric and string
fields, primitive `Debug` formatting, and pool exhaustion. Attribute computation
and caller-defined `Debug` implementations can allocate. Encoder/exporter workers,
transport libraries and OS resources have separate allocation costs.

| Limit | Behavior |
|---|---|
| Eight fields per record | Fields use bounded record storage |
| 256 UTF-8 bytes per string field | Strings copy into packed block storage; exhausted space truncates at a UTF-8 boundary and increments field loss |
| 256 bytes for metadata and thread names | Unsupported callsite metadata is filtered |
| Bounded context stack | Overflow suppresses contextual parenting until guards unwind in LIFO order; explicit parent IDs remain available |
| Bounded span table | Exhaustion increments rejected spans |

Repeated static names appear once per batch dictionary. Packed string storage
avoids per-record `String` allocation on application threads.

### Publication

`flush_events=true` publishes events immediately, including standalone events.
Set it to false to batch event-heavy workloads.

Full blocks, span lifecycle records, span exit, `Registration::flush()` and
registration drop publish partial blocks. With batching enabled, call `flush()`
before an idle period: an idle producer retains its partial block until a
publication trigger occurs.

## Buffers and loss reporting

Crossbeam `ArrayQueue` transfers capture blocks between free and ready pools.
The encoder reuses scratch storage and writes into preallocated encoded buffers.
Bounded channels carry leases to sink workers, which borrow immutable batch bytes.
The last lease returns its buffer to the pool.

Each sink has its own queue capacity, worker and drop counter. Configuration
reserves encoded buffers for every output queue, one in-flight batch per output,
and the encoder so encoding can progress while outputs hold leases.

| Condition | Result |
|---|---|
| Capture pool exhausted | Records are dropped |
| Sink queue full | That sink's incoming batch is dropped |
| Batch sequence gap | Exposes downstream loss |
| Shutdown | `ShutdownReport` returns capture losses, per-sink dropped/written batches and sink errors |

Every batch carries cumulative producer loss counters. Sequence numbers reflect
publication order; timestamps retain event time.

Conservative core storage bounds are:

- Capture: `blocks × (records_per_block × record size + string_bytes_per_block)`.
- Encoded buffers: `encoded_buffers × (4096 × records_per_block + string_bytes_per_block + 1024)`.
- Span/context storage: bounded span table plus `max_threads × max_depth` context entries.

OS thread stacks, MCAP indexes, OTel SDK allocations and transport buffers add
memory outside these pools. MCAP indexes grow with recording length; applications
control file rotation.

## Outputs

### MCAP

`McapSink` uses the `mcap` crate with the activity schema. It embeds the
FileDescriptorSet and preserves encoded payloads and capture timestamps.
Creation refuses existing files. Finalization flushes the writer; power-loss
durability requires an application policy.

Activity schema and channel ownership live here. The CPU-specific
`telemetry-mcap::McapSink` accepts CPU envelopes.

### WebSocket (`websocket` feature)

`WebSocketSink::accept(stream, timeout)` performs the server handshake during
setup. Each binary frame contains one self-contained batch. Create one sink per
preconnected viewer.

Applications own listening, authorization, TLS termination and reconnect policy.
Tungstenite copies frame payloads on the sink worker; I/O timeouts bound network
stalls. Dynamic viewer attachment is future work.

### OpenTelemetry (`otel` feature)

`OtelSink::http(endpoint, timeout, max_active_spans)` uses the official SDK exporter
interface and OTLP HTTP Protobuf exporter. `OtelSink::new` accepts a custom SDK
`SpanExporter` that supports synchronous workers and bounds its I/O.

- Trace IDs combine the random session ID and root span ID; span IDs match the recording.
- Export preserves links, fields, parentage, span events and timestamps.
- Assembly bounds active spans, attributes (64), events (128) and links (64).
- Evicted or unfinished spans export with `activity.incomplete=true`; sequence
  gaps mark active spans.
- Standalone events without a span count as orphan records. An OTel logs adapter
  is future work.
- `OtelStats` exposes incomplete, orphan and gap counts.

## Shutdown

Drop every registration guard, then call `shutdown()` to drain and finalize.
Explicit shutdown checks that producer registrations have ended and returns sink
errors in its report. `Drop` stops workers as a fallback.

Custom sinks must finish within an application-appropriate timeout. Shutdown
waits for disk writes, which can block in the OS.

## Run

```sh
cargo test -p activity-tracing --all-features
cargo run -p activity-tracing --example record -- /tmp/activity.mcap
cargo run -p activity-tracing --release --example throughput -- 100000
cargo run -p activity-tracing --release --example throughput -- 100000 1000
```

The benchmark reports wall time, encoded bytes and losses. Its optional second
argument sets the sleep duration in microseconds after each 64 attempted events.
Wall time includes this pacing; encoded bytes include one warm-up event. Compare
loss rates alongside throughput. Thor memory-bandwidth validation requires a
run on Thor.

## Message causality and monitoring

The [Activity lab](../../apps/activity-demo/) runs four worker threads with a live
TypeScript viewer and embedded MCAP recording.

1. Capture a copied token with `handle.context(&publish_span)`.
2. Carry the token in the message.
3. Create the receive span inside `handle.with_parent(context, || { ... })`.

Tokens preserve parent and trace identity after the publisher closes. They apply
to one runtime and a synchronous callback on a registered thread.

Read `handle.buffers()` from a monitoring thread to sample free/ready capture
blocks, free encoded buffers, registered threads and configured payload
capacities. Queue lengths are approximate, independently sampled values.
Monitoring reads these snapshots separately from producer capture callbacks.
