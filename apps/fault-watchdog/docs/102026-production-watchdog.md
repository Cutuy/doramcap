![Production watchdog proposal](production-watchdog.svg)

# Production watchdog

Created 2026-10-20. Implementation and validation checklist.

## Milestones

- [x] Production `apps/fault-watchdog`: Dora ingress/publication, deadline scheduling and injected `FaultObservability`.
- [x] App-owned `WatchdogConfig.proto`, resolved through `pbtxt-config`; validate before accepting reports.
- [x] Optional `mcap` adapter using `storage-persistence::McapWriter` and existing fault schema descriptors.
- [x] Optional `otel` adapter; map domain transitions/events to structured OTel logs with optional trace correlation.
- [x] Separate demo sources and dashboard consuming the same production publication stream.

## Validation

- [x] Test default, `mcap`, `otel`, and combined feature builds.
- [x] Reject runtime configuration for outputs absent from the build.
- [x] Validate deadlines/reconnect through Dora; test bounded retries and epoch/revision gaps at host/display boundaries.
- [x] Confirm slow/full/failing sinks cannot block aggregation or other outputs.
- [x] Verify MCAP schemas, readback, idle flush deadlines and bounded finalization.
- [x] Verify recorder faults remain visible without recursive persistence/export failures.
- [x] Measure control-to-live smoke latency; run ARM64 and Dora validation.
- [ ] Follow-up: benchmark persistence/export latency distributions under sustained load.

## Build and runtime boundary

The base app always aggregates and publishes domain output over Dora. `default = []` means no optional exporters; aggregation is an ordinary required dependency. `mcap` adds the optional writer dependency; `otel` adds exporter dependencies. Both can be enabled together. Features belong on the app and gate its adapter modules; core fault crates remain independent of exporters.

Protobuf config selects which compiled sinks run, queue/retry limits, recording paths/rotation/flush, export endpoints and shutdown deadlines. Enabling a Cargo feature only makes a capability available. Keep one stable config schema across builds and reject unavailable capabilities at startup. Do not silently ignore requested sinks.

Keep producer condition definitions authoritative for recovery/deadline semantics initially. App configuration owns operational limits and routing; policy overrides need an explicit precedence contract before adding them.

Cargo features are compile-time optional dependency selection and should be additive; runtime options select enabled behaviors. See the [Cargo feature reference](https://doc.rust-lang.org/cargo/reference/features.html).

## Ownership

| Owner | Responsibility |
|---|---|
| Production services | Use `fault-reporter` in their high-level error handling |
| Watchdog app | Config, clocks, source lifecycle, bounded fan-out and shutdown |
| Fault libraries | Capture, domain state, schemas and checked wire translation |
| MCAP adapter | Register fault schema/channels; submit publications to the writer library |
| OTel adapter | Map domain events and isolate exporter backpressure |
| Demo app | Synthetic scenarios, live visualization and controls |

The MCAP adapter uses `storage-persistence` directly. Existing `telemetry-mcap` wraps telemetry-specific schemas and is not a generic fault adapter. Reuse writer batching/rotation/finalization instead of implementing another writer. The same `AggregationPublication` contract serves live consumers and recording. OTel is a projection of that domain contract.

Reporter draining and aggregation run in host event loops. Each output has an independent bounded queue and explicit loss/reconciliation behavior. MCAP blocking flush/finalization and OTel retries need isolated sink execution; an MCAP owner thread is acceptable where required by the writer API. This is separate from producer reporting and must not stall the aggregation loop. Schedule idle writer progress/flush explicitly.

Report sink failures locally into aggregation and publish them to remaining healthy outputs. Suppress the failing sink's own diagnostic feedback, retaining failure/drop counters and a local fallback. MCAP enabled does not imply durable delivery of every event: overflow, crash-loss and shutdown outcomes must be observable.

## Configuration ownership

Place operational config under `apps/fault-watchdog/proto/`; keep shared reporter and aggregation wire records under `libs/fault/schema/proto/`. Reuse `pbtxt-config` without mixing unrelated payload types in one resolution. The user approved the official OTel SDK/OTLP HTTP exporter before adoption.

## Evidence

Repository review confirmed `storage-persistence` is the MCAP writer, `telemetry-mcap` is a telemetry-specific wrapper, and `pbtxt-config` supplies typed Protobuf TextFormat composition. The app reuses Dora, Prost, Crossbeam, Tokio/Axum and the approved official OTel 0.33 logs exporter.

The SVG was rendered and visually inspected; its XML and local design link were validated.

## Implementation checklist

### Configuration and host
- [x] Generate stable app config descriptors; implement presence-aware pbtxt overlays/defaults.
- [x] Validate feature availability, memory/cardinality bounds and sink settings at startup.
- [x] Bind each configured input to one source incarnation; require registration snapshot before batches.
- [x] Reject malformed frames without stopping healthy inputs; count/report ingress faults.
- [x] Handle input close, transport timeout, source restart/retirement and final snapshots.
- [x] Advance freshness deadlines while ingress is idle; publish periodic reconciliation snapshots.

### Output isolation
- [x] Bounded queues and retained frame retry limits for live, MCAP and OTel.
- [x] Queue overflow requests a new snapshot; expose per-sink drops/errors.
- [x] Feedback becomes local registered fault conditions; suppress each sink's own feedback.
- [x] Bound shutdown wait and report unfinalized output explicitly.

### Adapters and demo
- [x] MCAP: create-new paths, embedded descriptor/channel, publication timestamps, rotation, idle flush, finish.
- [x] OTel: official log exporter, stable fault identity/revision attributes, severity and trace IDs, timeout/retries.
- [x] Demo: two producer instances, fail/recover/flap/pause/flood/event controls and periodic source snapshots.
- [x] Dashboard: snapshot/update reconciliation, epoch/gap handling, bounded history, separate health/freshness.
- [x] Dora composition and repeatable headless smoke validation with recording readback.

### Release checks
- [x] Config/restart/overflow/deadline/feedback/slow-sink unit tests.
- [x] Local feature matrix, Clippy, rustdoc and demo HTTP/control checks.
- [x] ARM64 workspace/native suite and explicit Dora demo/MCAP/OTLP validation.

## Implementation evidence — 2026-10-20

- Ten focused tests pass: host lifecycle/config, immediate publication, blocked-sink shutdown/overflow, self-feedback filtering, bounded retries, display reconciliation, MCAP readback/rotation/flush, and OTLP HTTP identity/trace correlation.
- Default, `mcap`, `otel`, and combined feature tests pass locally. Strict all-target Clippy, formatting and rustdoc pass.
- MCAP test reads recordings with the independent `mcap` reader; the HTTP test decodes official OTLP Protobuf messages.
- ARM64 workspace/native job 95 passed. Dora job 100 passed fail/recover/pause/flood/event controls, MCAP readback (788 publications), OTLP delivery (788 requests), and the documented template. Control-to-active smoke latency was 27.5 ms. Final-source job 101 passed ten focused tests, strict Clippy, and Dora/MCAP/OTLP validation: 792 publications/requests and 28.7 ms control-to-active smoke latency.
- Runtime details: [queue, retry, configuration and shutdown contract](102026-runtime.md).

Validation notes: job 94 hit an intermittent existing storage observability test;
job 95 passed it and the full suite. Dora validation caught runtime compatibility
and producer-close ordering; both were fixed before job 100. App startup uses a
one-worker Tokio runtime required by Dora. The final host closes all sink queues
before waiting on any finalizer; a dedicated test covers a blocked peer.

All submitted validation jobs have finished. Follow-up app builds used two compiler jobs. Sustained-load latency distributions remain a separate benchmark task; smoke timings are not latency guarantees.
