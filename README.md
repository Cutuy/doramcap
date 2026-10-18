# doramcap

Phase 0 prototype: native cgroup CPU accounting → dedicated 10 Hz collector →
bounded reusable buffers → Dora node → separate MCAP recorder.

The [dated plan](docs/101826-phase0-cgroup-monitor.md) tracks design, milestones,
and validation. Kernel cgroup counters are collected at every hierarchy level;
no parent totals are synthesized from children or tasks.

## Build and try

Rust is pinned in `rust-toolchain.toml`; dependencies are locked in `Cargo.lock`.
The default build supports explicit synthetic data and standalone JSONL output.

```sh
cargo build --locked --workspace
cargo run -p system-monitor -- --check
cargo run -p system-monitor -- --backend synthetic --samples 10 --output /tmp/monitor.jsonl
cargo test --locked --workspace
```

Native BPF builds need Clang with the BPF target, libelf/zlib development files,
and Linux headers. `CLANG` can select a compiler. Build and run:

```sh
cargo build --locked --workspace --all-features
cargo test --locked --workspace --all-features
./target/debug/system-monitor --check
./target/debug/system-monitor --backend bpf --samples 10
```

Loading requires BPF/tracing privileges, readable kernel BTF, cgroup v2 and the
required iterator/kfunc support. Intended minimum qualification target is ARM64
Linux 6.8. A successful capability report does **not** prove attach support.
Native mode fails explicitly; it never substitutes synthetic measurements.

With Dora CLI 1.1.0 installed and the all-feature build complete:

```sh
dora run examples/phase0-dataflow.yml
```

Run `python3 examples/validate_dora.py --help` for the reproducible end-to-end validation. This check passed with Dora 1.1.0, including all ten MCAP messages and original timestamps.

The example publishes ten synthetic snapshots and writes `phase0.mcap`. The
recorder refuses to overwrite an existing recording. To collect native data,
change the monitor arguments in the example to `--backend bpf --samples 100 --dora`.
The system-monitor process contains the Dora node; MCAP serialization and file
I/O occur in the separate recorder process.

## Timing and memory contract

Collection runs every 100 ms and records start/end monotonic timestamps plus
Unix capture time. Counters are cumulative; a snapshot is not an atomic kernel
transaction. Missed deadlines skip catch-up bursts. `--records` (default 4096)
bounds cgroup × possible-CPU rows; `--queue-slots` (default 8) bounds batches.
When the pool is exhausted, collection continues into a reusable discard batch,
increments loss, and publishes again when a slot is available. Final session
statistics on stderr include losses after the last published snapshot.

Numeric collection/handoff uses preallocated storage. The synthetic allocator
test covers 10,000 iterations with zero allocations. This is not yet a measured
guarantee for native BPF collection. Kernel iterator creation itself allocates
kernel descriptor/seq state. Metadata discovery, JSON serialization and Dora
publication may allocate on the publication thread.

Published JSON includes a schema version, session identity, sequence, timestamps,
losses, numeric cgroup rows and a bounded cgroup-path dictionary. MCAP stores the
same JSON with a JSON schema. Dictionary entries are upserts keyed by cgroup ID within the session; full checkpoints appear every 100 published batches. Paths refresh at most every 10 seconds and can remain unknown under permission, depth or capacity limits. Entries are retained for the session; no retirement protocol is implemented. Consumers must honor availability flags; root,
IRQ and idle values are not fabricated. Do not add overlapping ancestor totals
or add task diagnostics to cgroup totals.

## Qualification status

Native Rust and CO-RE programs compile. Unprivileged loading failed with
`EPERM`; a subsequent sudo-authenticated smoke test reached BPF resolution but
failed because host Linux 7.0 lacks `cgroup_rstat_flush` in BTF. Its BTF contains
`css_rstat_flush` instead; this newer kernel needs separate compatibility work. Verifier acceptance, native allocator measurements,
per-core accuracy, Jetson performance and ARM64 execution remain open. SSH to
the offered ARM64 host stopped at host-key verification; no trust settings were
changed.

`--task-capacity N` enables experimental task runtime diagnostics with feature
`task-bpf`. Counters retain task lifetime, cgroup and CPU identity; finite map
capacity eventually fills under task churn. Task names, retirement, IRQ/idle
reconciliation and vendor scheduler qualification remain future work.

See [native backend details](crates/cgroup-bpf/README.md) and
[task accounting semantics](crates/task-accounting/README.md) before interpreting
measurements. This is a development prototype, not a qualified production monitor.
