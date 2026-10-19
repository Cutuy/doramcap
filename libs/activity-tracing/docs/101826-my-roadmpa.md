# Activity tracing roadmap

[Architecture](architecture.svg)

Implement an embedded recording library with standard `tracing` instrumentation,
Crossbeam queues, reusable capture blocks and encoded buffers. Register producer
threads before capture. Bound records, strings, active spans, queues and memory.
MCAP and live consumers share encoded batches; OpenTelemetry conversion runs in
its own worker. Initialization and output libraries may allocate. Measure the
complete steady-state producer path and publish its limitations.

## M1 — Bounded capture and ownership

- [x] Preallocate capture blocks; pack dynamic string bytes with offsets.
- [x] Explicit thread registration, monotonic/wall-clock correlation and identities.
- [x] Shared ready/free Crossbeam queues; visible overflow and shutdown behavior.

Gate: exhaustion, recycle, UTF-8 bounds, concurrent producers, and loss accounting
pass. Producer allocation tests cover successful and overflowing capture.

## M2 — Standard tracing integration

- [x] Bounded subscriber span/context storage; begin, update, event, link and end.
- [x] Correct parent/context handling, span cloning and cross-thread lifecycle.
- [x] Explicit capacity rejection, bounded fields, and partial-block flush.

Gate: nested, cloned and cross-thread spans preserve identities and close once;
registered producer callbacks allocate zero after setup. Document synchronization
and thread-registration requirements.

## M3 — Wire batches and independent fan-out

- [x] Versioned Protobuf schema, embedded descriptors and self-contained batches.
- [x] Reusable encoding buffers; bounded output queues pass shared buffer leases.
- [x] Recycle after last lease; slow/failing sinks report losses independently.

Gate: decode round trips, final-reader recycling, queue saturation, sink failure,
and shutdown drain tests pass. No per-event owned strings on the producer path.

## M4 — Embedded output adapters

- [x] MCAP writer with original batch bytes, schemas, timestamps and finalization.
- [x] Live WebSocket output with bounded writes and binary Protobuf frames.
- [x] Official OpenTelemetry SDK adapter with bounded span assembly and OTLP setup.
- [x] Runnable embedded example and public lifecycle documentation.

Gate: read a produced MCAP, receive/decode a live frame, and inspect exported
OpenTelemetry spans with matching trace/span identities. Shutdown finalizes outputs
and surfaces errors. No separate recorder process required.

## M5 — Integration and performance evidence

- [x] Format, lint, default/all-feature tests and workspace regressions.
- [x] Allocation validation and reproducible throughput/bytes-per-event benchmark.
- [x] ARM64 Pueue submission and explicit result inspection.
- [x] Record measured results, remaining limits and commands in this directory.

Gate: required checks pass; environmental blockers are recorded explicitly.
Allocation freedom applies only to tested registered producer paths. SDK, writer,
compression and network allocations remain separate audit items. Thor bandwidth
claims require measurements on Thor hardware.

## Execution notes

Worktree: `codex/activity-architecture` (pool slot 4). M1–M4 validation passed
on ARM64 Pueue job 21. The producer allocation gate includes 1,000 lossless
span/event cycles followed by a 10,000-cycle burst. See [validation evidence](validation.md)
for final job results and performance limits. M5 passed targeted job 25 and full
workspace/native job 26. All implementation milestones are complete; production
capacity tuning on Thor remains a documented follow-up.
