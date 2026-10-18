# Phase 0: native cgroup CPU monitor

Date: October 18, 2026
Status: implementation plan

## Outcome

Start doramcap as an independent Rust/Cargo project. A system-monitor executable hosts a Dora node and a dedicated 10 Hz collection thread. The first monitor reads native cgroup v2 CPU accounting through eBPF/libbpf-rs, preserving every hierarchy level. Numeric batches cross a bounded, preallocated handoff to publication. MCAP encoding belongs downstream.

This plan supersedes the original roadmap's Phase 0 scope where they differ. Later roadmap phases remain exploratory.

## Agreed design

- Minimum qualification target: ARM64 Linux 6.8, including NVIDIA Jetson Linux 39.2.1 / JetPack 7.2.1 for Thor and Orin. Kernel configuration, BTF, permitted BPF hooks, and kfuncs must be checked; a version number alone is insufficient.
- Develop in this repository using Rust/Cargo; consume Dora and libbpf-rs as external dependencies. Use small C CO-RE programs through libbpf-cargo where needed.
- Reuse the kernel's native cgroup hierarchy and inclusive counters at every level. **Never derive parent cgroup totals by summing tasks or child cgroups.** Root accounting has distinct native semantics.
- Prefer snapshotting native per-CPU cgroup counters after the kernel flushes rstat. Any inaccessible metric is explicitly unavailable, not silently synthesized.
- Individual task/kernel-thread measurements are diagnostic views. Preserve identity and actual cgroup membership; do not claim asynchronous kernel work belongs to its initiating application.
- Collect every 100 ms using actual monotonic timestamps. Collection reads cumulative counters, not statistical samples of whichever task is running at that instant.
- Preallocate batches and queue capacity. Numeric IDs appear in the hot path; names reside in a bounded dictionary with revision/generation information. Metadata work is outside the periodic collection path.
- A single collection producer hands owned batches to a single publication consumer. The collector never waits for Dora, compression, or disk I/O. On exhaustion, drop the new batch and expose cumulative loss counters. Recycle consumed buffers.
- Publish snapshots and dictionary context through the Dora node. No requirement for MCAP bytes on the collection thread. A separate downstream recorder can write the same published payloads into MCAP.
- Distinguish kernel per-CPU scratch storage from actual per-core measurements. Preserve unknown/overflow states. Do not double-count task/kthread views with cgroup totals or overlapping ancestor totals.
- Normal shutdown stops collection, joins the thread, drains eligible batches, and finalizes downstream recording.

## Architecture

```text
native kernel accounting / eBPF
    -> CPU monitor backend
    -> dedicated 10 Hz collection thread
    -> reusable batch pool + bounded SPSC handoff
    -> system-monitor Dora node
    -> live consumer and/or separate MCAP recorder
```

A capability report and deterministic synthetic backend make the executable testable without BPF privilege. Synthetic data must be explicitly selected and labeled; it is not a fallback pretending to be kernel data.

## Milestones and validation

### M1 — Repository and contracts

- [ ] Create Cargo workspace, executable/library boundaries, README and reproducible dependency lockfile.
- [ ] Define numeric snapshots, capacities, availability/loss flags, clock metadata and dictionary identity semantics.
- [ ] Implement reusable bounded handoff and a monitor interface.

Validation:
- [ ] `cargo fmt --check`, workspace build, and targeted unit tests pass.
- [ ] Exercise queue exhaustion, buffer reuse and producer/consumer shutdown.
- [ ] An allocation-counting test verifies steady-state collection/handoff does not allocate after initialization.

### M2 — Platform capability and native cgroup backend

- [ ] Detect kernel/architecture, cgroup v2, BTF, CPU capacity, and BPF load/attach errors.
- [ ] Build CO-RE BPF with libbpf-rs/libbpf-cargo and collect native inclusive CPU counters for all discoverable cgroups.
- [ ] Read native per-core counters where qualified, retaining special root semantics and explicit availability.
- [ ] Bound result capacity and report overflow/errors instead of reallocating or silently losing data.
- [ ] Establish dictionary entries for cgroup IDs, parent IDs and paths outside the periodic hot path.

Validation:
- [ ] Compile BPF and Rust integration; test capability failures without privileges.
- [ ] On a qualified host, compare native cgroup totals with `cpu.stat` over aligned intervals, including parent/child groups.
- [ ] Validate per-core values under pinned CPU load and cgroup creation/deletion.
- [ ] Record any missing privileges or unsupported kernel features as unfulfilled qualification gates.

### M3 — Monitor host and Dora publication

- [ ] Host the monitor in a dedicated periodic thread and the Dora node in the publication path.
- [ ] Publish versioned batches with timestamps, cumulative counters, sequence/loss information and dictionary context.
- [ ] Provide a standalone diagnostic/synthetic run mode and a Dora dataflow example.

Validation:
- [ ] Run a finite synthetic session and verify cadence, sequence, identity and clean shutdown.
- [ ] Deliberately slow publication; demonstrate bounded memory and visible sample loss while collection continues.
- [ ] Run a real Dora dataflow when the Dora executable is available; otherwise state this qualification gap.

### M4 — Downstream MCAP smoke path

- [ ] Provide a separate recorder executable/example consuming the published payload schema.
- [ ] Register schemas/channels once, preserve original capture time, and finalize files explicitly.

Validation:
- [ ] Write and read back MCAP; verify counts, payload values, timestamps and sequence numbers.
- [ ] Confirm MCAP work does not run on the acquisition thread.

### M5 — Task/kernel-thread coverage and CPU reconciliation

- [ ] Qualify a common scheduler runtime basis for userspace tasks and kernel threads across supported scheduling classes.
- [ ] Capture task lifetime identity, cgroup membership and per-core runtime without custom cgroup rollups.
- [ ] Represent idle, IRQ, softirq and unexplained residuals with clearly documented accounting semantics.

Validation:
- [ ] Test short-lived tasks, migration, long-running tasks spanning collection boundaries and PID reuse.
- [ ] Compare diagnostic task views with native cgroup accounting, documenting native root/interrupt differences.
- [ ] Measure collection cost and allocation behavior under load; do not claim zero allocation or exact reconciliation without evidence.

### M6 — ARM64 and handoff

- [ ] Validate compilation and a one-off run on ARM64 Linux; offered host: `cutuy@100.64.0.20` (WSL2).
- [ ] Keep WSL2 functional validation distinct from Jetson BSP/performance qualification.
- [ ] Update this plan with completed work, commands/results and remaining gates.

Validation:
- [ ] Native ARM64 or cross-compilation succeeds, with the validation mode recorded.
- [ ] The README describes prerequisites, bounded capacities, operational commands and limitations.

## Initial implementation choices

- Use an explicitly versioned fixed-width binary batch encoding for the internal/published data; choose an MCAP schema encoding that accurately describes it. Do not label arbitrary binary as Protobuf or Arrow IPC.
- Keep publisher-side allocation separate from acquisition-side allocation measurements.
- Use fixed capacity configurable at startup, with defaults suitable for a development machine. Record actual capacities in diagnostics.
- Do not install persistent services or alter remote kernel/cgroup configuration merely to make tests pass.

## Evidence and decisions

- `below` reviewed at `bbddf87047e6e548527ab6331af9ad2f610482d8`: useful native-counter reader, but per-snapshot map/vector/string allocations and non-preallocated BPF results do not satisfy our collection-path goals.
- Linux 6.8 is the agreed common minimum. Older Orin JetPack 6 / kernel 5.15 support is outside this initial profile.
- Full native per-core and task accounting are feasibility/qualification work, not capabilities assumed from the scaffold.

## Implementation log

Pending.
