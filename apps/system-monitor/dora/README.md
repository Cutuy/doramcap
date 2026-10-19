# System monitor DORA wrapper

`system-monitor-dora` owns the DORA node on the monitor's publication thread.
It publishes snapshots and dictionary checkpoints, polls stop events, and closes
the node after the final publication. It encodes immutable Protobuf packets
directly into `send_output_raw` destinations after checking their encoded size.

This crate is enabled by the `system-monitor/dora` feature. Collection libraries
remain independent of DORA. Run `cargo test -p system-monitor-dora` for buffer
encoding checks and the recorder's end-to-end test for delivery validation.
