# Experimental task runtime diagnostics

Optional per-task runtime counters for `PF_KTHREAD` kernel threads, separate from
native cgroup collection. It never builds cgroup totals or ancestor rollups.

With feature `bpf`, `Backend::new_experimental(capacity)` attaches a CO-RE raw
tracepoint to `sched_stat_runtime`. The probe admits only `PF_KTHREAD` tasks; a
map-read invariant rejects other records. Default builds contain only contracts
and a deterministic replay accumulator.

Each counter key is `(start_boottime_ns, cgroup_id, tid, tgid, cpu, flags)`.
Counters hold charged runtime since attachment; do not add them to native cgroup
totals or treat deltas as exact wall-clock slices.

[CPU accounting nuances](CPU-ACCOUNTING.md)
