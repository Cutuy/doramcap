# Recording failure handling

![Planned failure and shutdown flow](architecture/recording-failure-handling.svg)

Source audit found five gaps below. Implementation and validation remain open.
Prioritize F1 and F2. Completion errors must reach applications independently of
optional observation; outstanding I/O retains its memory until retirement.

## F1 — Completion-error propagation

Finding: [`Writer::progress`](../src/writer.rs) returns success after recording a
failed CQE. Small commits into an existing combining buffer can also succeed
after the writer has latched an error.

- [ ] Return latched failures from public progress/poll and all admission/commit paths.
- [ ] Separate internal draining from public error reporting so every outstanding request can retire safely.
- [ ] Preserve the first failure with operation/file context; classify backpressure separately.
- [ ] Validate injected EIO, ENOSPC, zero-byte and invalid short completions: idle polling and subsequent small/large/slab/SHM admission expose failure; draining releases owners safely.

## F2 — Explicit shutdown

Finding: dropping [`McapCore`](../src/mcap/core.rs) loses an active combining batch;
`Writer::drop` suppresses drain errors. Early returns in the system monitor and
storage demo can skip explicit finalization.

- [ ] Finalize recording on normal stop and recoverable application-error exits; preserve both the original error and any finalization failure.
- [ ] Define an explicit failure shutdown path that drains submitted I/O and reports accepted-but-unwritten data.
- [ ] Detect unfinished writer destruction with an always-available failure signal; define behavior during unwinding without a second panic.
- [ ] Validate early input/publication errors, pending combining data, footer/truncate/sync failures, and persistent submission failure. Assert error results, retained ownership and independently readable output where finalization succeeds.

## F3 — Durable sink completion

Finding: the [telemetry MCAP sink](../../telemetry-mcap/src/lib.rs) and
[activity MCAP sink](../../activity-tracing/src/sinks/mod.rs) finish and flush
buffered output without syncing the underlying file.

- [ ] Flush and sync the file before reporting successful sink finalization; propagate each failure.
- [ ] Validate injected flush/sync failures and successful finalized-file readback for both sinks.

## F4 — Application failure status

Finding: storage-demo catches recording failures, publishes a UI error and keeps
running; its worker returns `()`, allowing successful process exit. Exercise
commands classify preceding flush failures as command rejection.

- [ ] Retain session failure through worker join and process exit; keep the failure visible in the UI.
- [ ] Distinguish recoverable command rejection from recording failure in exercise controls.
- [ ] Validate a failed recording produces a failed session and nonzero exit; backpressure remains retryable.

## F5 — Sticky durability failures

Finding: [`Writer::sync_file`](../src/writer.rs) returns sync errors without
latching writer failure, allowing later admission or file replacement.

- [ ] Latch sync/finalization failures and reject subsequent admission or rotation; define any explicit recovery separately.
- [ ] Validate failed sync remains visible through polling, admission and shutdown; successful finalization reports synced coverage.

## Qualification

- [ ] Run fault-injection and byte-level readback tests on x86 and ARM64 with observation disabled and enabled.
- [ ] Run affected-package default/all-feature tests, strict Clippy and recorder/demo shutdown tests.
- [ ] Record commands, worker job IDs and results here before checking milestones.
