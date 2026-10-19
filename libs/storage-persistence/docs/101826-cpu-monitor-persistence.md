# CPU monitor: embedded persistence plan

Status: implementation validated on x86 Linux and ARM64 WSL. See the
[writer plan](101826-mcap-writer-design.md).

## Components and artifacts

[Approved architecture](architecture/cpu-monitor-storage-review.svg)

```text
eBPF counters → collector thread (10 Hz)
             → bounded snapshot lease handoff
             → writer/publication owner thread
                 → telemetry adapter → MCAP reservation
                 → Protobuf encoding into managed payload slice
                 → storage commit → io_uring → filesystem → storage
                 → Dora publication → cpu-viewer process
```

Collection remains on its dedicated thread. Storage and publication share an
owner thread initially. The collector passes preallocated snapshot leases;
queue overflow has explicit accounting. The owner resolves dictionary/schema
state and encodes messages into reserved storage buffers.

The telemetry adapter owns Protobuf knowledge. The MCAP layer owns channels,
schemas and framing. The storage writer owns offsets, buffers and I/O. Embedded
recording is configured by system-monitor; each writer instance is independent.

Dora's current publication API owns its destination buffers. Publishing the
serialized payload requires a copy into Dora storage. End the payload borrow
before allowing pool reclamation. Storage completion alone permits reclamation
only when all borrows have ended. A future external-buffer Dora API requires a
separate lifetime design.

Retain bounded collector progress during storage pressure. Define application
policy for recording loss, publication loss and fatal recording errors. Record
observable counters and preserve timestamps/sequences across both destinations.
Each rotated file includes schema/channel definitions and a dictionary checkpoint.

## Milestones and validation

This plan executes within base milestone M7. Complete M1–M4 below in order;
record commands, commit, platform and pass/fail evidence before advancing.
Earlier base writer gates must remain passing. Follow the base plan gate rules.

- [x] M0: Save process/component architecture and integration plan.
  Validation: diagram renders and documentation links resolve.
- [x] M1: Define configuration, overload behavior, flush cadence, shutdown order
  and dictionary checkpoint policy.
  Validation: walkthrough of slow storage, full pool, stalled publication,
  recording errors, rotation and termination; collector remains bounded.
- [x] M2: Wire the telemetry adapter to reserve/encode/commit and embed the writer.
  Validation: compare recorded payloads to published payloads by timestamp and
  sequence; independently decode recorded Protobuf and read the MCAP; verify
  storage has no Protobuf/Dora dependency; trace payload copies and allocations.
- [x] M3: Exercise end-to-end operation and failure handling.
  Validation: sustained 10 Hz collection with recording and viewer publication;
  delayed I/O and exhausted buffers; dictionary changes and rotation; graceful
  shutdown and write failures; verify counters expose losses and errors.
- [x] M4: Validate on x86 and ARM64.
  Validation: native capability-required collection tests plus separate Dora
  end-to-end tests on both hosts. Use the documented Pueue worker for ARM64;
  report job results, allocation counts and collection/publication latency.

Storage allocation goals apply to the new writer's steady state. Existing
snapshot conversion and dictionary allocations need separate measurement and
explicit follow-up work if they prevent the application-level budget.

## M7 implementation policy and qualification budgets

Recording is optional. Default recording pool: eight 256-KiB buffers, 4-KiB direct
I/O, 100-ms combining age. Buffered mode is selectable. Large encoded records use contiguous allocations grown on demand.
The owner drives progress during idle polls.
Storage errors are fatal and observable; the collector joins before exit. Explicit flushes may stall the owner while the collector continues and counts
discarded batches. An injected owner delay exercises the same bounded handoff.

Rotation uses a caller-configured soft byte target, creates files exclusively,
repeats schemas/channels and writes a full dictionary before the next snapshot.
Dora publication copies from the already encoded managed payload; packet
construction allocations remain measured separately from storage operations.

Qualification budgets, set before integration measurement: normal synthetic
10-Hz runs must retain every sample and produce independently readable files;
intentional 250-ms owner delays with one queue slot must expose dropped batches
and terminate cleanly. Storage hot-path tests must retain zero heap allocations.
Record elapsed time and sample/drop counts; timing benchmarks are observations,
with a 30-second test timeout for short functional runs. Hardware-specific
bandwidth/latency targets require target NVMe measurements.

Validation: base plan M7 evidence and ARM64 Pueue job 24. Synthetic/native Dora
flows each matched 24 snapshots and two dictionaries against embedded MCAPs.
