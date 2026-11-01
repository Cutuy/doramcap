# Cgroup and task CPU accounting: interpretation

## Two observation paths, related native accounting

| Library | Measurement | Scope |
|---|---|---|
| `cgroup-bpf` | eBPF reads native per-CPU cgroup `subtree_bstat` runtime after rstat flush | Cgroup plus descendants; root rows use native global per-CPU accounting |
| `task-accounting` | eBPF observes `sched_stat_runtime` and accumulates charged runtime | PF_KTHREAD lifetime × CPU × cgroup × classification flags, starting at attachment |

Neither CPU measurement comes from polling `/proc` or cgroup filesystem stat
files. Filesystem discovery supplies cgroup names/paths only.

In upstream Linux v6.8, scheduler `update_curr_task()` sends the **same
`delta_exec`** to the runtime tracepoint and `cgroup_account_cputime()`.
Consequently, runtime deltas have a common accounting basis. The two views
overlap; adding them double-counts runtime. Exact reconciliation on a vendor
kernel still requires validation: snapshots are not atomic or aligned,
accounting updates can be delayed, and the task map can lose records when full.
Compare interval deltas over aligned windows. Raw cgroup user/system fields are
not the adjusted `cpu.stat` values.

## Membership and worker roles

Processes normally inherit cgroup membership. A login session can contain many
programs; services, containers, or explicitly launched scopes can provide
separate workload groups. There is no automatic one-cgroup-per-process rule.

| Task flag | Meaning | Typical CPU attribution |
|---|---|---|
| `PF_KTHREAD` | Kernel thread | Its existing cgroup; many reside in root |
| `PF_IO_WORKER` | I/O worker, typically an io_uring worker acting for userspace | Normally inherits the creating task's cgroup; runtime is already included in that cgroup |
| `PF_WQ_WORKER` | Kernel workqueue worker (`kworker`) | Its own cgroup, typically root |

Worker **role** and kernel-thread **classification** are different dimensions.
In particular, `PF_IO_WORKER` does not imply `PF_KTHREAD`. Actual membership
determines cgroup attribution; verify each worker's cgroup. Membership changes
and kernel/version-specific accounting can complicate comparisons. CPU
attribution is distinct from block-I/O or memory attribution.

The BPF task collector now filters for `PF_KTHREAD` before identity lookup/map
insertion. Typical `PF_IO_WORKER` tasks lacking that flag are excluded. The
publication path sums kernel counters per CPU into `virtual/kernel`, a separate
reporting namespace, and sends no individual task records.

The task collector records the task's existing default cgroup ID. It creates no
cgroups and moves no tasks. The cgroup component of its key preserves membership
at each observed charge; it does not identify which application caused
asynchronous kernel work. Currently its exported classification flags contain
only `KERNEL_THREAD` (bit 0); I/O/workqueue role flags are **not yet exported**.
These flags use library-defined bit values.

## Reporting rules

- Label aggregates of selected tasks as **reporting groups**.
- A kernel-thread line sums `PF_KTHREAD` runtime per CPU. Application syscall
  execution and separate IRQ/softirq/idle accounting remain outside its scope.
- An I/O-worker breakdown may already be inside a session/service total. Show
  it as an overlapping diagnostic breakdown.
- Cgroup totals include descendants, so parent/child lines also overlap.
- Report unavailable measurements and task-map losses explicitly; do not infer
  zero usage or force all series to sum to 100%.

## References

- [Linux v6.8 scheduler accounting](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/fair.c)
- [Linux task flags](https://github.com/torvalds/linux/blob/v6.8/include/linux/sched.h)
- [Cgroup v2 membership and inheritance](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [Kernel workqueues](https://www.kernel.org/doc/html/latest/core-api/workqueue.html)
- [Task collector coverage, loss semantics, and qualification gates](README.md)
