# Native cgroup backend

Enable `bpf` to compile the real CO-RE reader. Build dependencies: Rust, clang
with BPF target, libelf and zlib headers/libraries (libbpf is vendored by
libbpf-rs). Set `CLANG` when clang is outside PATH.

The sleepable cgroup iterator walks the kernel hierarchy in preorder, flushes
native rstat once, and reads each non-root cgroup's per-CPU `subtree_bstat`.
There is no custom hierarchy rollup. The root is emitted with
`ROOT_UNAVAILABLE`; its native global accounting must be acquired separately.
`RAW_USER_SYSTEM` indicates that user/system counters are native raw values,
not the adjusted values printed by `cpu.stat`. Runtime is scheduler runtime.
The host needs BTF, cgroup iterator, `cgroup_rstat_flush` kfunc and appropriate
BPF/tracing privileges. Linux 6.8 is the intended qualification target, not a
promise that every vendor kernel exposes these capabilities.

A record capacity bounds cgroup × possible-CPU rows (including root). The
backend counts excess rows in `CollectionStats::overflow`. Output Vec must
already have capacity at least equal to the configured capacity. Array maps
are preallocated once, reused without insert/delete churn; map reads use
caller-owned stack memory, avoiding libbpf-rs allocating lookup convenience APIs.
Each collection creates a kernel iterator descriptor and seq-file state. This
is **not** a claim of zero kernel allocations or atomic snapshots: concurrent
kernel flushes/updates can race reads, and observation timestamps must bracket
the traversal. Successful userspace collection is intended to allocate no heap
memory; error diagnostics and backend creation may allocate. Allocation and
performance qualification on NVIDIA BSP remains required.

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

Initial development validation: default ABI/feature tests and native Rust/BPF
compilation passed. The one-shot native run failed at libbpf's trivial program
probe with `EPERM` (also unable to raise RLIMIT_MEMLOCK); the actual cgroup
program has **not** passed the kernel verifier or produced measurements here.
Pinned-load comparisons, allocation profiling and target ARM64 qualification
remain open.
