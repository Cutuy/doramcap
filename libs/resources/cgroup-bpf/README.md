# Native cgroup backend

Reads per-cgroup, per-CPU CPU time directly from kernel rstat counters through a
sleepable BPF cgroup iterator. Root rows read global `kernel_cpustat`.

`Backend::new(record_capacity)` then `collect_into(&mut Vec<CgroupRecord>)`.
Neither silently falls back to synthetic or filesystem metrics; a build without
feature `bpf` returns `FeatureDisabled`. The reader emits only kernel numeric
identities.

Feature `bpf` needs clang with the BPF target plus libelf and zlib; libbpf is
vendored. Set `CLANG` when clang is outside PATH.

```sh
cargo test -p cgroup-bpf
cargo check -p cgroup-bpf --features bpf
cargo run -p cgroup-bpf --features bpf --example snapshot
```

[Traversal, requirements and bounds](docs/design.md) ·
[Validation evidence](docs/101826-native-kernel-validation.md) ·
[CPU accounting nuances](../task-accounting/CPU-ACCOUNTING.md)
