# Monitor core

Shared CPU snapshot types, bounded reusable buffers, producer/consumer handoff,
and aggregation of kernel task runtime into virtual per-CPU reporting counters.

`Consumer::recv()` blocks until a snapshot arrives or the producer disconnects.
Selected receives combine snapshots with control channels while preserving reusable leases.

Run `cargo test --release --locked -p monitor-core`. The allocation integration test verifies that
steady-state numeric collection and handoff allocate no userspace memory.
The [system-monitor application](../../../apps/system-monitor/) owns scheduling and
publication; [task-accounting](../task-accounting/) defines task identities.
