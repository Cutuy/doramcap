# Resources

Measurement and control of host resources. Current crates cover Linux CPU and
cgroup accounting, snapshot handoff, CPU placement, and scheduling policy.

| Crate | Responsibility |
|---|---|
| [monitor-core](monitor-core/) | CPU snapshot types, bounded buffers, and collection handoff |
| [cgroup-bpf](cgroup-bpf/) | Native per-CPU cgroup counters through BPF |
| [task-accounting](task-accounting/) | Experimental kernel-thread runtime diagnostics |
| [resource-policy](resource-policy/) | CPU affinity, real-time scheduling, and cgroup bootstrap |

The [system-monitor application](../../apps/system-monitor/) owns collection
scheduling. [Telemetry](../telemetry/) owns publication and recording contracts.
