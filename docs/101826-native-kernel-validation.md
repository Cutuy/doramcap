# Native Linux and ARM64 validation

## Design and scope

Finish Phase 0 native validation before advancing the roadmap. First qualify
native inclusive cgroup/per-core accounting on the local Linux 7.0 host; then
run on the user's ARM64 host at cutuy@100.64.0.20. Preserve Linux 6.8 compatibility
as an explicit target. Adapt to BTF/kernel accounting changes rather than
substituting synthetic counters or summing child cgroups.

Use the shared ignored sudo credential only for authorized test executables.
Keep test cgroups and workloads isolated, bounded, and cleaned up. Do not alter
persistent services, boot configuration, or the user's workloads.

## Milestones

- [x] Inspect live BTF and kernel source; implement compatible native reader.
  - Validation: BPF compilation, privileged verifier load, attachment, nonempty live snapshots.
- [x] Validate local accounting with controlled pinned CPU work in nested cgroups.
  - Validation: native parent/child counters, per-core attribution, cpu.stat deltas,
    overflow flags, actual 10 Hz capture, repeated snapshots, clean shutdown.
- [x] Run existing regression checks and native allocation/overhead measurements.
  - Validation: report exact checks and limitations; no synthetic results counted as native.
- [x] Connect to ARM64 host, inspect kernel/BTF/toolchain, build and run there.
  - Validation: architecture, native load and controlled accounting checks; distinguish
    WSL2 functional evidence from NVIDIA BSP/performance qualification.
- [x] Record evidence and remaining gaps in Phase 0 plan and README.

## Results

Validated October 18, 2026, using real kernel data on both hosts.

| Check | Local x86_64 | ARM64 WSL2 `.20` |
|---|---|---|
| Kernel | 7.0.0-38-generic | 6.18.54.1-microsoft-standard-WSL2 |
| Verifier/load/attach | PASS | PASS |
| Controlled native collection | 65 snapshots, 3,120 rows/snapshot | 65 snapshots, 504 rows/snapshot |
| Parent and two pinned children vs cpu.stat | <1 microsecond difference | <1 microsecond difference |
| Per-core workload attribution | All measured child runtime on assigned core | All measured child runtime on assigned core |
| Median collection / observed maximum | 1.027 / 8.577 ms | 0.428 / 1.539 ms |
| Median interval / observed maximum | 100.000 / 100.098 ms | 100.004 / 100.287 ms |
| Result-capacity overflow | Explicitly exercised | Explicitly exercised |
| Native userspace allocation probe | 0 allocations / 40 warmed-up calls | 0 allocations / 40 warmed-up calls |
| Real native Dora → MCAP | 10 sessions, 100/100 snapshots | 10 sessions, 100/100 snapshots |
| All-feature Rust regression suite | 11 passing tests | 11 passing tests |

These short runs establish functional behavior, not worst-case latency or a
long-duration overhead budget. The allocation probe measures successful native
userspace collection after warm-up; kernel iterator allocations still occur.
Root counters remain explicitly unavailable. Task/kernel-thread attribution,
IRQ/idle reconciliation, Linux 6.8, and NVIDIA BSP qualification remain open.

## Implementation changes demonstrated necessary by live testing

- Select `css_rstat_flush(&cgroup->self)` or legacy `cgroup_rstat_flush(cgroup)`
  using weak kfunc existence, and select split or legacy rstat storage via CO-RE.
- Use bounded numeric BPF iterators; the original loop exceeded verifier jump complexity.
- Read fields individually because CO-RE does not reorder members in a whole-struct copy.
- Resolve native per-CPU offsets through the kernel symbol table: rstat pointers
  lack the BTF per-CPU tag required by `bpf_per_cpu_ptr` on these hosts.
- Keep optional layouts in separate non-inlined functions, preventing compiler
  hoisting of relocations for absent fields.
- Require a visible `__per_cpu_offset` address before load. A non-root local test
  with only CAP_BPF/CAP_PERFMON failed; adding CAP_SYSLOG passed. Declare CAP_SYSLOG
  only at the symbol-reading leaf. Kernel symbol policy may differ by host.
- Finalize Dora by dropping its node, allowing data transport teardown before
  output-closure notification. Explicit early `close_outputs()` lost the final
  large native payload. Ten repeated runs on each architecture verify the fix.

## Reproduction

Build as an ordinary user; elevate only the validation programs:

```sh
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features
cargo build -p cgroup-bpf --features bpf --examples
sudo python3 examples/validate_native.py --monitor "$PWD/target/debug/system-monitor"
sudo ./target/debug/examples/native_allocations
sudo python3 examples/validate_dora.py --backend bpf --runs 10 --dora /path/to/dora-1.1.0
```

ARM64 source, tools, and binaries live under `/tmp/doramcap-validation` on `.20`.
The pinned Rust toolchain and downloaded Ubuntu compiler packages were extracted
there, without system-wide installation. Source transfers excluded credentials.
The ARM64 host's own credential file is `/home/cutuy/.sudo-password`, mode 0600,
outside the source tree; the local main-checkout credential was not copied there.

Source references: Linux [v7.0 rstat implementation](https://github.com/torvalds/linux/blob/v7.0/kernel/cgroup/rstat.c)
and [v6.8 rstat implementation](https://github.com/torvalds/linux/blob/v6.8/kernel/cgroup/rstat.c).

