# Embedded MCAP persistence: implementation plan

Status: M1–M7 implemented and validated on x86 Linux and ARM64 WSL.
The next managed-memory revision follows the
[growable free-list plan](101926-growable-buffer-pool.md), then the
[external-memory plan](101826-external-buffer-leases.md). Fixed-pool and segmented
reservation requirements below describe the original implementation; the new
plan supersedes them for future work. Historical validation remains unchanged.

## Architecture

- [Buffer lifecycle and writer](architecture/storage-persistence-review.svg)
- [CPU monitor integration](architecture/cpu-monitor-storage-review.svg)
- [Integration plan](101826-cpu-monitor-persistence.md)
- [Separate external-buffer feature](101826-external-buffer-leases.md)

Each independently created writer owns one io_uring and runs on one owner
thread. SQPOLL stays disabled. The application drives submission and completion.
Any-thread producer APIs are deferred until measurements justify them.

| Layer | Ownership and interface |
|---|---|
| Application / telemetry adapter | Protobuf sizing and encoding; schema and channel policy |
| MCAP layer | Record framing, schema/channel ordering, reserve/commit, sealing |
| Storage writer | File offset allocator, managed buffer pool, one io_uring |
| Linux filesystem and driver | File writes and device transfers |

The persistence library has no Protobuf or Dora dependency. The MCAP layer
reserves framing and payload space, returning a writable payload slice to the
caller. The caller encodes directly into that slice and commits the lease.
Framing is completed in the same storage buffer. String and byte fields are
copied during serialization; subsequent payload staging is avoided.

Small records occupy adjacent reservations in a combining buffer. Large records
use whole buffers or a bounded sequence of buffers. Define the oversized-record
contract before implementing multi-buffer records. Combining flushes on capacity,
maximum age, explicit flush, or close. Compression is deferred.

## Resource and file contracts

- Preallocate buffer memory, lease metadata, request slots, and completion state.
  Keep hot-path strings in startup metadata or dictionaries.
- Register stable, aligned buffers with the kernel. Initially configure 4 KiB
  alignment by default, with a 512 B option. Runtime discovery and compatibility
  validation are a follow-up; report I/O errors explicitly.
  Registered memory and direct I/O require separate configuration and validation.
- Allocate non-overlapping file ranges when buffers are sealed. Track outstanding
  ranges and the contiguous completed prefix independently of completion order.
- Emit valid MCAP records for alignment padding, including required record headers.
  Preserve leading/trailing magic, schema/channel ordering, DataEnd and footer.
  Decide the initial chunk/index policy in M1; bound metadata by file rotation.
- Validate final aligned writes and exact logical file length. Keep the trailing
  magic at the logical end and include truncation in finalization when needed.
- Reclaim a buffer after every referencing I/O and application borrow retires.
  Short writes retain ownership through retries. Cancellation tracks the original
  request through retirement. Errors remain observable after memory reclamation.
- Expose acceptance, write completion, durability, and finalization separately.
  Bound pool exhaustion and queue pressure with explicit return values.
- Evaluate SINGLE_ISSUER and DEFER_TASKRUN with owner identity and completion
  progress rules. Keep a supported fallback configuration explicit.

## Validation gate

Complete milestones in order. Before starting the next milestone, record the
commit, commands, platform/kernel/filesystem, results and relevant measurements
under the completed milestone. Check its box only when every acceptance check
passes. A failure or unavailable required environment keeps the gate open.
Re-run earlier regression checks affected by each change.

Run applicable functional suites on x86 Linux and the ARM64 Pueue worker at each
implementation milestone. Record the worker job ID and terminal success result.
WSL functional evidence and target-storage performance evidence remain separate.
Do not expand scope to work around a failed gate.

## Milestones

- [x] M0: Save approved base architecture and plans.
  Evidence: both SVGs parse; local documentation links resolve; diagrams reviewed.

### M1 — bounded managed memory

- [x] Implement exclusive leases and non-overlapping offset allocation, with
  explicit capacity limits and abandoned-reservation handling.
- Gate: exhaustion, reuse, arithmetic overflow and boundary-sized reservations
  pass; no live lease aliases writable memory; no steady-state heap allocation
  in repeated reserve/release cycles. Record configured limits and allocation counts.

### M2 — one ring, exact byte persistence

- [x] One owner submits managed buffers to one ring using ordinary buffered file
  I/O initially. Implement progress, completion, flush/close and error reporting.
- Gate: exact byte readback across reordered completions; injected partial writes,
  submission errors, cancellation and close with outstanding requests; owners
  survive until retirement. Demonstrate bounded queues, zero steady-state writer
  allocations and SQPOLL disabled. Distinguish written and synced completion.

### M3 — minimal valid MCAP

- [x] Encode directly into a reservation; one uncompressed Message per managed
  buffer, bounded record size, one file. Emit schemas/channels and valid close
  records. Use an explicit index-free baseline; return an error for oversized data.
- Gate: independent MCAP reading and Protobuf decoding recover exact payloads,
  sequence IDs and timestamps; empty file, size boundaries, schema/channel order,
  abandoned encoding and failed close pass. Verify the payload destination is
  the submitted buffer and measure allocation/copy counts after initialization.

### M4 — small-message combining

- [x] Reserve adjacent slices and flush on capacity, age, explicit flush or close.
- Gate: mixed message sizes retain M3 decoding results; no post-encoding payload
  copies; timer flush works at low rates and pool exhaustion is explicit. Test
  an unfinished reservation during flush and shutdown. Measure writes/message,
  buffer occupancy and latency against M3 with the same workload.

### M5 — aligned direct I/O

- [x] Add configured 4 KiB / 512 B alignment, registered buffers, valid MCAP
  padding, exact logical tail length and finalization ordering.
- Gate: real direct-I/O readback on a supporting filesystem; address, length and
  offset checks; records around alignment boundaries; padding-header minimums;
  trailing magic at EOF; write, truncation and sync failures. Preserve M3/M4
  decoding. Zero-copy claims require measured evidence of the chosen path.
  Record which alignment modes the test filesystem actually supports.

### M6 — bounded large records and rotation

- [x] Add a bounded multi-buffer record contract and rotation; repeat schema,
  channel and application dictionary state in each file. Define maximum record
  size and behavior when a record exceeds the rotation target.
- Gate: payload hashes across buffer boundaries; configured maximum and overflow;
  failures mid-record; independent reading of every rotated file; bounded memory
  through repeated rotations. Crash tests distinguish finalized files from
  recoverable complete-record prefixes and report incomplete records.

### M7 — embedded CPU-monitor qualification

- [x] Complete the linked integration plan, including its x86 and ARM64 gates.
- Gate: 10 Hz collection plus publication/recording, slow storage and pool
  exhaustion, dictionary changes, rotation and shutdown. Record losses, CPU,
  allocations, latency and throughput for low-rate and large-record workloads.
  Set workload-specific performance budgets before running qualification.

## Later work

- [ ] Runtime filesystem/device alignment discovery and startup validation.
- [ ] Chunk indexes and compression, each with separate format/memory validation.
- [ ] SINGLE_ISSUER / DEFER_TASKRUN tuning with progress and regression evidence.
- [ ] Any-thread frontend, driven by measured owner-thread contention.
- [ ] External-buffer leases: independent feature and gates in the linked plan.

## References

- [Original idea](../../../docs/todo/101826-io-uring-dma-buffer-freelist.md)
- [MCAP specification](https://mcap.dev/spec)
- [io_uring setup](https://man7.org/linux/man-pages/man2/io_uring_setup.2.html)

## Validation evidence — 2026-10-18

Base commit: `710231d5899a3cd33d748d2db5d825ccff91ceef`; implementation is an
uncommitted worktree change. Local host: x86_64 Linux 7.0.0-38-generic, ext4.
Worker: ARM64 WSL2 Linux 6.18.54.1. Each listed Pueue job ended in Success;
source hashes are retained in that job directory's SOURCE.json.

| Gate | ARM64 job | Acceptance evidence |
|---|---|---|
| M1 | 4 | Exclusive/aligned leases, exhaustion, reuse, offset abandonment/overflow; 20,000 allocation-free cycles |
| M2 | 6 | Real ring readback, error/partial completion handling, submission-failure retention, cancellation/drop; 1,000 allocation-free writes |
| M3 | 8 | Independent MCAP reader and Protobuf decode; empty/boundary/abandoned records; zero measured encode/commit allocations |
| M4 | 9 | Adjacent reservations, timer/capacity/explicit/close flushes; 34 small messages in three data writes |
| M5 | 10 | Registered direct writes at 512 B/4 KiB, valid padding, exact EOF, foreign-pool rejection, truncate/sync failures |
| M6 | 11 | Segmented records, maximum rejection, incomplete encoding, 16 rotations per mode, abrupt-exit prefix handling |
| M7 | 24 | Default/all-feature workspace tests, native accounting, overload/rotation tests, synthetic/native Dora equality, allocation benchmark and changed-component Clippy |

Local storage tests and Clippy passed through M6. Local M7 tests and live Dora
flows passed: 24 snapshots / 26 total messages matched byte-for-byte between
embedded and subscriber recordings, for both synthetic and native BPF sources.
Normal 10-Hz integration recorded all 24 samples in 2.324 s; injected 250-ms
owner delay recorded eight and reported 16 dropped batches in 2.377 s.

Benchmark: 64 records of 1 MiB, bounded 32 × 64-KiB pool, debug assertions enabled,
startup/readback excluded and final sync included. Both modes measured zero heap
allocations across encoding, writes and finalization. Figures are observations
on these hosts; target-storage bandwidth qualification remains hardware-specific.

| Host / mode | MiB/s | Process CPU ms | p99 record µs |
|---|---:|---:|---:|
| x86 / buffered | 130.3 | 38.9 | 1560.3 |
| x86 / direct | 312.9 | 112.7 | 45330.6 |
| ARM64 WSL / buffered | 725.2 | 68.3 | 1239.0 |
| ARM64 WSL / direct | 410.6 | 148.4 | 15365.4 |

Commands: `cargo test -p storage-persistence`,
`cargo test -p telemetry-mcap -p system-monitor`,
`cargo run -p storage-persistence --example storage_benchmark`, and
`python3 apps/system-monitor/tests/validate_recording_dora.py --dora PATH --bin-dir target/debug [--backend bpf]`.
Run native validation under the AGENTS.md sudo procedure. Worker submission uses
`scripts/ci/submit.py`; the serial worker runner covers the full suite.

Worker repairs: installed its documented python3-protobuf prerequisite after
job 3 failed; fixed archive timestamps after job 7 exposed stale Cargo artifacts.
Connection timeouts now bound stalled SSH submissions. Dora CLI 1.1.0 is verified
against the upstream release SHA256 before installation.

Final ARM64 job **24** repeated the complete suite successfully, including
`cargo clippy --no-deps --all-targets --all-features -- -D warnings` for the four
changed components. The table above retains the job-14 benchmark observation;
job 24 measured 860.5 MiB/s buffered and 415.6 MiB/s direct, with zero allocations.
Snapshot: `/home/doramcap-ci/jobs/6e37abc850dc4188b97f64ad47aa5ec3`.
