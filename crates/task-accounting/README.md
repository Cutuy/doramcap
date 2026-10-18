# Experimental task runtime diagnostics

This optional crate is separate from native cgroup collection. It never builds
cgroup totals or ancestor rollups. `Backend::new_experimental(capacity)` (feature
`bpf`) explicitly attaches a CO-RE raw tracepoint program to
`sched_stat_runtime`. Default builds contain only contracts and a deterministic
replay accumulator; they do not acquire or fabricate task measurements.

## Native source and coverage

Upstream Linux **v6.8** provides a promising common native runtime basis:

- [fair.c](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/fair.c):
  `update_curr_task()` emits `sched_stat_runtime(p, delta_exec)`, then passes that
  same delta to group and cgroup accounting. Fair tasks call it directly.
- [rt.c](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/rt.c) and
  [deadline.c](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/deadline.c):
  `update_curr_rt()` / `update_curr_dl()` use `update_curr_common()`, which calls
  `update_curr_task()`. This tracepoint is **not fair-only in upstream v6.8**.
- [stop_task.c](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/stop_task.c):
  `put_prev_task_stop()` also calls `update_curr_common()`. Idle has distinct
  semantics and is not a task-runtime stream here.

These are source findings, not confirmation of NVIDIA's exact BSP. Attach/load,
all scheduling classes, kernel threads, CPU attribution, cgroup movement, and
interrupt accounting must still be exercised on each supported target.

The raw tracepoint supplies the actual task pointer and charged duration. The
probe reads TID, TGID, `start_boottime`, `PF_KTHREAD`, and the task's real default
cgroup ID. It obtains the attributed CPU from `task->thread_info.cpu` (the native
SMP `task_cpu()` representation), **not the CPU executing the probe**: runtime
updates can execute remotely. This CO-RE layout currently requires SMP kernels
with embedded `thread_info.cpu`; incompatible BTF fails load explicitly.

## Counter and timing contract

Each key is `(start_boottime_ns, cgroup_id, tid, tgid, cpu, flags)`. Counters
contain charged runtime observed after attachment. PID reuse gets another
lifetime identity; CPU and cgroup movement create separate records. Persist a
boot/session identifier in the containing recording because identities are not
globally unique across reboots. No task strings occur in this path.

Collection reads cumulative counters; it never estimates uncharged execution.
A charge may cover time preceding a collection boundary (including attachment),
and NO_HZ tasks may delay updates. Repeated snapshots can therefore remain
unchanged while a task runs. Do not label a delta as an exact 100 ms wall-clock
execution slice. Collect start/end monotonic timestamps in the containing batch.

Snapshots are **not atomic** across records. Concurrently inserted keys may first
appear in the next traversal. No deletion occurs while attached, which avoids
iteration restarts and silently discarded final counters for short-lived tasks.
The deliberately fixed capacity counts task-lifetime × cgroup × core combinations.
Churn eventually fills the map: new keys increment cumulative dropped-event and
dropped-runtime counters; existing keys continue updating. Restarting attachment
starts a new session and resets counters. A bounded retirement/acknowledgement
protocol is required before enabling this for long unattended runs.

The kernel hash map uses its default preallocated entries; userspace reserves its
output vector at startup and uses raw map syscalls with stack key/value storage.
Normal collection does not deliberately allocate strings or per-record buffers.
This implementation still needs allocator and overhead profiling on a real BPF
run. Loading, attaching, error construction, and publication are outside that
claim. The optional `RuntimeAccumulator` is a replay/testing reference, not the
production acquisition path or a high-rate userspace event processor.

## IRQ, idle, and reconciliation

Native `rq_clock_task` accounting depends on kernel configuration. Linux v6.8
[core.c](https://github.com/torvalds/linux/blob/v6.8/kernel/sched/core.c)
`update_rq_clock_task()` subtracts IRQ accounting and paravirtual steal time when
their respective facilities are enabled; it also documents attribution lag at
interrupt boundaries. `ksoftirqd` task work must not be counted twice as both
thread execution and a generic softirq duration.

This crate does **not** measure idle, separate IRQ/softirq totals, or unexplained
residuals. Those remain unavailable, not zero. Do not equate the sum of these task
counters with root `cpu.stat`, or add task diagnostics to native cgroup totals.
Do not force a residual to zero. No workload-origin attribution of asynchronous
kernel work is claimed.

## Remaining qualification gates

- Build and load on NVIDIA ARM64 6.8 BTF/config; verify task CPU identity on ARM64.
- Exercise fair, FIFO/RR, DEADLINE, stop/kernel threads and explicit idle exclusion.
- Pinned CPU load, task migration, cgroup migration, rapid exit/PID reuse.
- Long-running NO_HZ tasks and windows straddling attach/detach and accounting updates.
- Capacity exhaustion: retained counters, dropped events, and dropped runtime.
- Compare aligned long-window task and native cgroup counters without custom rollups.
- Measure allocation/latency under load; add per-CPU IRQ/idle diagnostics with
  explicit native semantics and a safe retirement protocol before production use.

M5 remains incomplete until those gates pass. The monitor must expose this as an
explicit experimental option if integrated; it must not imply qualification from
successful compilation alone.
