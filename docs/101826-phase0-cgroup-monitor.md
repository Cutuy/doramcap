# Phase 0: native cgroup CPU monitor

Date: October 18, 2026
Status: native cgroup pipeline validated on x86_64 Linux 7.0 and ARM64 WSL2 Linux 6.18; Linux 6.8/Jetson and task diagnostics remain open

## Outcome

Start doramcap as an independent Rust/Cargo project. A system-monitor executable hosts a Dora node and a dedicated 10 Hz collection thread. The first monitor reads native cgroup v2 CPU accounting through eBPF/libbpf-rs, preserving every hierarchy level. Numeric batches cross a bounded, preallocated handoff to publication. MCAP encoding belongs downstream.

This plan supersedes the original roadmap's Phase 0 scope where they differ. Later roadmap phases remain exploratory.

## Agreed design

- Minimum qualification target: ARM64 Linux 6.8, including NVIDIA Jetson Linux 39.2.1 / JetPack 7.2.1 for Thor and Orin. Kernel configuration, BTF, permitted BPF hooks, and kfuncs must be checked; a version number alone is insufficient.
- Develop in this repository using Rust/Cargo; consume Dora and libbpf-rs as external dependencies. Use small C CO-RE programs through libbpf-cargo where needed.
- Reuse the kernel's native cgroup hierarchy and inclusive counters at every level. **Never derive parent cgroup totals by summing tasks or child cgroups.** Root accounting has distinct native semantics.
- Prefer snapshotting native per-CPU cgroup counters after the kernel flushes rstat. Mark inaccessible metrics explicitly unavailable.
- Individual task/kernel-thread measurements are diagnostic views. Preserve identity and actual cgroup membership; do not claim asynchronous kernel work belongs to its initiating application.
- Collect every 100 ms using actual monotonic timestamps. Collection reads cumulative runtime counters.
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

A capability report and deterministic synthetic backend make the executable
testable without BPF privilege. Select and label synthetic data explicitly.

## Milestones and validation

### M1 — Repository and contracts

- [x] Create Cargo workspace, executable/library boundaries, README and reproducible dependency lockfile.
- [x] Define numeric snapshots, capacities, availability/loss flags, clock metadata and dictionary identity semantics.
- [x] Implement reusable bounded handoff and a monitor interface.

Validation:
- [x] `cargo fmt --check`, workspace build, and targeted unit tests pass.
- [x] Exercise queue exhaustion, buffer reuse and producer/consumer shutdown.
- [x] An allocation-counting test verifies steady-state collection/handoff does not allocate after initialization.

### M2 — Platform capability and native cgroup backend

- [x] Detect kernel/architecture, cgroup v2, BTF, CPU capacity, and BPF load/attach errors.
- [x] Build CO-RE BPF with libbpf-rs/libbpf-cargo and collect native inclusive CPU counters for all discoverable cgroups.
- [x] Read native per-core counters where qualified, retaining special root semantics and explicit availability.
- [x] Use fixed result capacity and explicit overflow/error counters.
- [x] Establish dictionary entries for cgroup IDs, parent IDs and paths outside the periodic hot path.

Validation:
- [x] Compile BPF and Rust integration; test capability failures without privileges.
- [x] On a qualified host, compare native cgroup totals with `cpu.stat` over aligned intervals, including parent/child groups.
- [x] Validate per-core values under pinned CPU load and cgroup creation/deletion.
- [x] Record any missing privileges or unsupported kernel features as unfulfilled qualification gates.

### M3 — Monitor host and Dora publication

- [x] Host the monitor in a dedicated periodic thread and the Dora node in the publication path.
- [x] Publish versioned batches with timestamps, cumulative counters, sequence/loss information and dictionary context.
- [x] Provide a standalone diagnostic/synthetic run mode and a Dora dataflow example.

Validation:
- [x] Run a finite synthetic session and verify cadence, sequence, identity and clean shutdown.
- [x] Deliberately slow publication; demonstrate bounded memory and visible sample loss while collection continues.
- [x] Run a real Dora dataflow when the Dora executable is available; otherwise state this qualification gap.

### M4 — Downstream MCAP smoke path

- [x] Provide a separate recorder executable/example consuming the published payload schema.
- [x] Register schemas/channels once, preserve original capture time, and finalize files explicitly.

Validation:
- [x] Write and read back MCAP; verify counts, payload values, timestamps and sequence numbers.
- [x] Confirm MCAP work does not run on the acquisition thread.

### M5 — Task/kernel-thread coverage and CPU reconciliation

- [ ] Qualify a common scheduler runtime basis for userspace tasks and kernel threads across supported scheduling classes.
- [ ] Capture task lifetime identity, cgroup membership and per-core runtime without custom cgroup rollups.
- [ ] Represent idle, IRQ, softirq and unexplained residuals with clearly documented accounting semantics.

Validation:
- [ ] Test short-lived tasks, migration, long-running tasks spanning collection boundaries and PID reuse.
- [ ] Compare diagnostic task views with native cgroup accounting, documenting native root/interrupt differences.
- [ ] Measure collection cost and allocation behavior under load; do not claim zero allocation or exact reconciliation without evidence.

### M6 — ARM64 and handoff

- [x] Validate compilation and a one-off run on ARM64 Linux; offered host: `cutuy@100.64.0.20` (WSL2).
- [x] Keep WSL2 functional validation distinct from Jetson BSP/performance qualification.
- [x] Update this plan with completed work, commands/results and remaining gates.

Validation:
- [x] Native ARM64 or cross-compilation succeeds, with the validation mode recorded.
- [x] The README describes prerequisites, bounded capacities, operational commands and limitations.

## Initial implementation choices

- Use fixed-width numeric structs internally and versioned JSON on the publication thread. MCAP registers a JSON schema and preserves the published payload; JSON/string allocations stay outside collection.
- Keep publisher-side allocation separate from acquisition-side allocation measurements.
- Use fixed capacity configurable at startup, with defaults suitable for a development machine. Record actual capacities in diagnostics.
- Do not install persistent services or alter remote kernel/cgroup configuration merely to make tests pass.

## Evidence and decisions

- `below` reviewed at `bbddf87047e6e548527ab6331af9ad2f610482d8`: its per-snapshot map/vector/string allocations and BPF result allocation exceed our collection-path allocation budget.
- Linux 6.8 is the agreed common minimum. Older Orin JetPack 6 / kernel 5.15 support is outside this initial profile.
- Full native per-core and task accounting require feasibility and qualification checks.

## Implementation log

- Plan written before implementation; work split into core/host, native BPF, task diagnostics and publication/MCAP components.
- `cargo test --workspace --all-features`: 11 tests pass, including 10,000 zero-allocation synthetic collection/handoff iterations, concurrent buffer transfer, ABI checks, task replay and MCAP readback.
- Real native load smoke test: blocked by `EPERM` at libbpf probe. Native per-core comparisons, verifier acceptance and native allocation measurements remain unfulfilled.
- ARM64 SSH attempt: `Host key verification failed`; no bypass or remote changes performed.
- Experimental task backend is implemented. M5 remains incomplete: no live scheduler-class qualification, task-name dictionary, retirement, idle/IRQ or residual measurements.
- New repository worktree workflow appeared during implementation. Continuing in pool slot 2 on `feature/phase0-cgroup-monitor`; lease retained until publication. Original checkout was preserved.
- Backpressure smoke: 8 collected, 5 published, 3 reported drops with a deliberately slow publisher. SIGTERM smoke: 4 collected and published, clean exit.
- Cgroup dictionary uses bounded session-local entries and revisioned upserts; paths refresh at most every 10 seconds. Task names and safe retirement remain deferred.
- Real Dora 1.1.0 dataflow and independent MCAP parser passed: 10 collected/published, zero drops, 10 finalized messages with sequences 0–9 and matching original capture timestamps. `examples/validate_dora.py` reproduces the check. Recorder drains buffered inputs past normal input-closure events.
- Final formatting and whitespace checks pass; default-feature tests pass (12), all-feature tests pass (11).

### Capability declarations

- [x] Keep commented `#[cap(CAP_BPF, CAP_PERFMON)]` declarations only on native leaf helpers that load BPF objects or attach tracing programs. Constructors, monitor entry points and publication carry no blanket declarations.
- [ ] Qualify exact capability requirements on the target kernel; declarations describe the intended Linux 6.8 tracing profile. Minimum capabilities still need qualification. No macro, aggregation framework or automatic privilege grant is introduced.

Validation: compile both BPF backends after extracting load/attach helpers. Existing-descriptor map access and iterator creation are not automatically annotated merely because they use BPF APIs; kernel-policy qualification remains open.

### Privileged native smoke test

Sudo-authenticated execution of `target/debug/examples/snapshot` on host
`7.0.0-38-generic` passed the earlier permission barrier, then failed during BPF
external-function resolution: `cgroup_rstat_flush` was not found in kernel or
module BTF. Host vmlinux BTF strings contain `css_rstat_flush` instead. The
smoke test stopped at a host kernel compatibility blocker, before verifier
acceptance or live accounting. The Linux 6.8 target still needs direct
validation; a 7.0 adapter requires checking the changed accounting structures
and kfunc ABI. No credentials were persisted in repository files.

### Native and ARM64 validation completed

The previous EPERM and missing-function blockers are resolved for local Linux
7.0 and ARM64 WSL2 Linux 6.18. Both pass controlled native parent/child per-core
accounting against cpu.stat, 65-sample 10 Hz collection, overflow, warmed-up
allocation probes, all-feature tests, and ten native Dora/MCAP sessions each.
See [the validation plan and evidence](101826-native-kernel-validation.md).
NVIDIA BSP, Linux 6.8 and experimental task-backend qualification remain open.
Root CPU accounting remains explicitly unavailable.
