# Validation — 2026-10-18

Environment: ARM64 WSL2 Linux 6.18, Rust 1.99.1, serial default Pueue queue on
`doramcap-ci@100.64.0.20`. Work was performed in pool slot 4 on
`codex/activity-architecture`. The local x86 build ran out of disk space;
compilation and executable validation used the ARM64 worker.

## Gates

| Gate | Evidence | Result |
|---|---|---|
| M1: bounded capture | Exhaustion/recycle, 4 concurrent producers, UTF-8 limits, registered-thread capacity | Passed |
| M2: instrumentation | Nested/cloned spans, cross-thread close, async polling on two threads, depth overflow recovery | Passed |
| M2: allocations | 1,000 successful span/event cycles, then a 10,000-cycle burst; allocation/reallocation counter on producer thread | **0 allocations** |
| M3: encoding/ownership | Protobuf decode and invalid-range rejection, last-reader recycling, isolated slow sink, complete failed-sink loss accounting | Passed |
| M4: MCAP | Original payload bytes, schema descriptor, timestamps, finalization, overwrite refusal | Passed |
| M4: runnable example | Pueue **27**: 9 MCAP batches, zero loss/errors; all-target Clippy passed after example capacity adjustment | Passed |
| M4: live | Local WebSocket handshake, binary frame and Protobuf decode | Passed |
| M4: OTel | Exported trace/span IDs, parentage, links, events, incomplete/evicted spans, actual OTLP HTTP request to local collector | Passed |
| M5: library | 20 all-feature tests and strict all-target Clippy; Pueue **25** | Passed |
| M5: workspace | Locked default/all-feature tests, binaries/examples build, privileged native accounting and allocation validation; Pueue **26** | Passed |
| Formatting/artifacts | Workspace rustfmt check, diff whitespace check, XML parse and rendered SVG inspection | Passed |

Final workspace snapshot:
`/home/doramcap-ci/jobs/517d81611f494d87986f912841154e2c`.
Its `SOURCE.json` records source hashes and the dirty-worktree state.

```sh
ssh doramcap-ci@100.64.0.20 pueue log --full 25
ssh doramcap-ci@100.64.0.20 pueue log --full 26
cargo test --locked -p activity-tracing --all-features
cargo clippy --locked -p activity-tracing --all-features --all-targets -- -D warnings
```

## Performance observations

The benchmark sends two-field standalone events into a byte-counting sink.
It includes one warm-up event in output bytes. It measures producer-loop wall
time and reports loss; it does not measure MCAP disk throughput or Thor bandwidth.

| Run | Events attempted | Producer loss | Sink loss | Encoded bytes | Loop wall time |
|---|---:|---:|---:|---:|---:|
| Burst, Pueue 18 | 100,000 | 82,976 records | 31 batches | 1,181,891 | 13.84 ms |
| 1,000 µs pacing per 64 events, Pueue 23 | 100,000 | 0 | 0 | 7,943,715 | 1.881 s |

The lossless paced run delivered 1,564 batches and approximately **79.44 encoded
bytes per event**, including its warm-up event. Its wall time includes deliberate
sleep. The burst result demonstrates overload/drop behavior; its loop time cannot
be interpreted as successful end-to-end throughput. These measurements preceded
the final registered-thread capacity guard; final functional and allocation gates
include that guard.

```sh
cargo run --release -p activity-tracing --example throughput -- 100000
cargo run --release -p activity-tracing --example throughput -- 100000 1000
```

## Remaining performance and product boundaries

- Span bookkeeping uses a standard mutex; contention and tail latency require
  workload measurements. The successful allocation gate provides no wait-free guarantee.
- Caller-defined attribute construction and Debug formatting may allocate.
- MCAP indexing, OTel assembly/serialization and WebSocket payload copies run on
  output workers and may allocate. Recording rotation and whole-process memory
  bounds require additional policy.
- The live adapter handles preconnected viewers. Dynamic attachment/reconnection,
  authentication and TLS termination are application responsibilities.
- Standalone events have no OTel span and increment orphan counters. An OTel logs
  adapter and cross-process context propagation remain future extensions.
- Test on Thor before setting bandwidth, CPU-cost or production capacity budgets.
