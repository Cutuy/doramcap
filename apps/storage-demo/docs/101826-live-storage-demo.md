# Live storage demo

## Design

`system-monitor` emits framed Protobuf at 10 Hz into a bounded demo input queue.
A single owner thread decodes CPU packets, adds irregular small heartbeat packets,
and encodes directly into managed MCAP reservations. An optional large-message
burst exercises segmented reservations. The demo defaults to synthetic CPU input;
`--backend bpf` selects native collection with the required process capabilities.

The writer exposes bounded, allocation-free diagnostics: queued write ranges,
write CQEs, submission totals, pool availability and combining occupancy/age.
A separate HTTP runtime streams snapshots to a dependency-free web UI. Diagnostics
serialization and file inspection allocate in the demo application.

The UI correlates input messages, combining occupancy, SQE/CQE history, parsed
MCAP records and actual file-tail bytes. CQE completion means the write completed;
seal/rotate drains, truncates and fsyncs. Live snapshots can miss short-lived queue
occupancy; retained events preserve completion evidence. Disk inspection is bounded
per update and only advances through complete records after writes retire.

Controls: heartbeat burst, large message, explicit flush and seal/rotate. The app
binds loopback, uses exclusively created session files, and limits control/input
queues. Each rotation retains the io_uring and managed pool.

## Milestones and validation

- [x] D1: bounded writer diagnostics. Test range/CQE correspondence and unchanged
  allocation behavior with the existing storage suite.
- [x] D2: demo owner, CPU pipe, heartbeats, streaming endpoint and file parser.
  Validate bounded queues, input failure reporting, MCAP readback and rotation.
- [x] D3 implementation: web views, controls, freeze, sealed-file downloads.
- [x] D3 API validation: live SSE, control commands, independent MCAP readback,
  direct-I/O tail and graceful shutdown with an active stream.
- [ ] D3 visual validation: browser interaction and screenshot review. The current
  computer-use session exposes no browser; this remains an explicit review item.
- [x] D3 ARM64 final-source validation on `.20` (Pueue job 33).

## Limits

The pipe adds transport copies before the demo owner re-encodes into reservations.
The storage writer remains independent of Protobuf. Browser diagnostics are for
inspection; benchmark measurements use the existing storage benchmark. O_DIRECT
is configurable; alignment starts at 4096 B. Heartbeats use small valid telemetry
SnapshotFrame packets on their own topic. Large probes use a large source string
in the same schema. Downloads are limited to sealed session files.

## Validation evidence

Local storage tests passed with diagnostic instrumentation, including zero-allocation
write/encoding tests and queued-range/CQE correspondence across event-ring wrap.
Demo and storage Clippy passed with warnings denied. The HTTP suite passed for
both synthetic input and native BPF input via the existing sudo monitor launcher.
The native test decoded all three topics through the independent MCAP reader.
JavaScript syntax passed `node --check`.

ARM64 Pueue jobs 29 and 31 passed the full workspace/native/demo/Clippy suite.
Final-source job 33 includes graceful monitor-launcher cleanup and the native
input test options. Job 33 passed the full suite, demo validation and Clippy.
Snapshot: `/home/doramcap-ci/jobs/68c6352ef51944de930ea3f9a8de0a04`.
The final ARM64 storage benchmark measured zero hot-path allocations in both
buffered and direct modes. A local missing-monitor check confirmed the visible
error state and clean server shutdown.

The live local native demo uses `--seconds 600` to bound capture growth on this
space-constrained host. The web page stays available after finalization. A test
rotation verified DataEnd → Footer → final magic directly from the recorded file.

## Restart control

- [x] Keep the HTTP server and command supervisor alive after capture expiry.
- [x] Start a new writer/monitor session from the page; reset counters and charts.
- [x] Retire the current writer before deleting this demo directory's capture files.
- [x] Validate restart after expiry and during recording, cleanup scope, and SSE shutdown.

Restart validation passed locally with synthetic and native BPF input and on
ARM64 in Pueue job 35. Tests cover capture expiry, active restart, counters,
scoped deletion, unrelated-file retention and shutdown with an SSE client.
The four prior interactive-demo MCAP files were deleted (474 MiB).

## Stop control

- [x] Stop recording through the existing owner command queue; seal and sync the file.
- [x] Keep HTTP inspection and session restart available.
- [x] Validate sealed tail, stable file size after stop, and restart after stop.

Stop validation passed with native BPF input: MCAP footer and magic present,
file size stable after stop, HTTP inspection alive, and subsequent restart works.
Clippy and JavaScript syntax checks passed.

## Probe timing and load controls

- [x] Retain allocation-free submission-call to CQE-observation durations.
- [x] Show rolling p99/max/last latency and separate sample/arrival deviations.
- [x] Add bounded paced bursts, cancellation, and stop-during-burst behavior.
- [x] Validate timing arithmetic, window eviction, real CQEs and restart/stop.

CPU deviations use consecutive collector monotonic end timestamps and owner
arrival timestamps. Expected elapsed time is sequence delta × 100 ms; gaps remain
explicit. Write timing includes owner polling delay. The latest 256 observations
form each rolling window; event-log overruns are counted. Burst size is 1–256
probes, each about 96 KiB, with requested spacing 0–1000 ms and at most one
probe scheduled per owner loop.

Local native-input validation passed for paced probes, cancellation, stop with
a pending burst, timing windows, and idle startup without capture files. Storage
tests including zero allocations and CQE timing checks passed; Clippy and JS
syntax checks passed. Old interactive captures were removed before final
validation (583.6 MiB). The interactive server starts paused.

ARM64 Pueue job 36 passed the full workspace/native/demo suite and Clippy.
Snapshot: `/home/doramcap-ci/jobs/d40050a3bd2f4b29a7163d2c5755f0e8`.

## Manual injection visibility

- [x] Retain eight writer-confirmed receipts with actual encoded count/bytes,
  sequence range and encode/commit duration.
- [x] Highlight manual messages and display receipts independently of the fast
  live activity feed. Receipts describe encoding into buffers.
- [x] Validate both injection receipts against their actual payloads.

Native BPF HTTP validation and independent MCAP readback passed, including
heartbeat/probe receipt count, sequence-range and byte checks. Clippy and
JavaScript syntax checks passed.

## Library boundary audit

The writer event ring, submission/CQE counters, per-write timestamps, MCAP
diagnostics snapshot and `finish_with_diagnostics` are opt-in under the library
`diagnostics` feature. The demo enables it explicitly. Default library builds
omit that instrumentation. Probe generation, rolling metrics, JSON/SSE, file
inspection and HTTP/UI controls remain in this app. CI tests the default writer
in a separate invocation before building the workspace with the demo.
