# Collection behavior

Collection runs every 100 ms, recording monotonic start/end times and Unix capture
time. Counters are cumulative and can advance during collection. Missed deadlines
skip catch-up bursts. `--records` bounds cgroup × possible-CPU rows and
`--queue-slots` bounds buffered batches. Exhausting the batch pool increments loss
and continues collection into a reusable discard batch.

Numeric collection and handoff use preallocated storage. Metadata discovery,
Protobuf packet construction, and publication happen outside the collection
thread. The DORA wrapper encodes directly into DORA-managed buffers. Standalone
output uses a reusable serialization buffer.

DORA publishes snapshots at 10 Hz and full dictionary checkpoints at startup and
approximately every 2 seconds. Session identity and revision join the topics.
Publication aggregates PF_KTHREAD records into per-CPU virtual kernel counters;
individual tasks remain internal. A bounded kernel task map can fill over time.
Availability flags and losses accompany the counters. Cgroup hierarchy values
can overlap; keep ancestor totals and kernel diagnostics separate when plotting.

Native cgroup collection has been qualified on x86_64 Linux 7.0 and ARM64 WSL2
Linux 6.18. See the [native validation report](../../../libs/resources/cgroup-bpf/docs/101826-native-kernel-validation.md),
[cgroup backend](../../../libs/resources/cgroup-bpf/), and [task accounting semantics](../../../libs/resources/task-accounting/CPU-ACCOUNTING.md).
Linux 6.8, vendor kernels and long-duration overhead qualification remain open.

For message definitions, see the [wire contract](../../../libs/telemetry/protocol/docs/wire-protocol.md).
