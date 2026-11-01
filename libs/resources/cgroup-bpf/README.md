# Native cgroup backend

Enable `bpf` to compile the real CO-RE reader. Build dependencies: Rust, clang
with BPF target, libelf and zlib headers/libraries (libbpf is vendored by
libbpf-rs). Set `CLANG` when clang is outside PATH.

The sleepable cgroup iterator walks the kernel hierarchy in preorder, flushes
native rstat once, and reads each non-root cgroup's per-CPU `subtree_bstat`.
CO-RE selects `rstat_base_cpu` or legacy `rstat_cpu`; weak kfunc detection
selects `css_rstat_flush` or `cgroup_rstat_flush`. Separate field reads preserve
layout correctness. Hierarchy totals come directly from kernel counters. Root
rows carry `ROOT` and read global per-CPU `kernel_cpustat` through BPF. Their
runtime is user + nice + system + IRQ + softirq, matching Linux root cgroup CPU
accounting. Offline runqueues are omitted. Root rows are independent of subtree
rstat and must be excluded from descendant rollups. `RAW_USER_SYSTEM` means user/system counters are raw,
before cpu.stat adjustment.

Requirements: BTF, cgroup iterator, numeric BPF iterator, a supported rstat flush
kfunc, visible `__per_cpu_offset`, and BTF-visible `kernel_cpustat`/`runqueues`
kernel symbols. The native per-CPU pointers
lack tags needed by `bpf_per_cpu_ptr`, so checked reads use the offset table.
CAP_BPF/CAP_PERFMON cover tracing; CAP_SYSLOG is additionally needed when kernel
policy hides symbol addresses. A startup check rejects hidden addresses.
Linux 6.8 remains the intended minimum, with direct qualification still pending.

A record capacity bounds cgroup × possible-CPU rows (including root). The
backend counts excess rows in `CollectionStats::overflow`. Output Vec must
already have capacity at least equal to the configured capacity. Array maps are
preallocated once, reused without insert/delete churn; map reads use
caller-owned stack memory, avoiding libbpf-rs allocating lookup convenience
APIs. Each collection allocates a kernel iterator descriptor and seq-file state.
Concurrent kernel flushes/updates can race reads; observation timestamps must
bracket the traversal. Successful userspace collection is intended to allocate
no heap memory; error diagnostics and backend creation may allocate. Allocation
and performance qualification on NVIDIA BSP remains required.

API: `Backend::new(record_capacity)` then `collect_into(&mut Vec<CgroupRecord>)`.
Neither method silently falls back to synthetic or filesystem metrics. Creating
a disabled backend returns `FeatureDisabled`. Dictionary discovery belongs to
the host's metadata path; the reader only emits kernel numeric identities.

Qualification commands:

```sh
cargo test -p cgroup-bpf
cargo check -p cgroup-bpf --features bpf
cargo run -p cgroup-bpf --features bpf --example snapshot
```

Live validation now passes on x86_64 Linux 7.0 and ARM64 WSL2 Linux 6.18:
controlled nested cgroups, pinned cores, cpu.stat comparisons, bounded overflow,
and 40 warmed-up collection calls with zero observed userspace allocations.
Run `apps/system-monitor/tests/validate_native.py` for controlled integration and the
`native_allocations` Cargo example for the allocation probe. See
[validation evidence](docs/101826-native-kernel-validation.md).
Linux 6.8/NVIDIA qualification and long-duration overhead testing remain open.

See [CPU accounting nuances](../task-accounting/CPU-ACCOUNTING.md) for worker attribution and reporting without double-counting.
