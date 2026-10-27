# Workflow event protocol v1

[`workflow.proto`](../proto/workflow.proto) defines `doramcap.workflow.v1.Event`. `workflow_protocol` supplies the codec and descriptor.
Transport provides framing and must bound frame size before decode.

Events carry schema version, observer sequence, session/invocation, derived Rust
step name, step contract revision, executor, parent/dependencies, state, monotonic
elapsed/duration nanoseconds, and optional JSON payload/error. Sequence is scoped
to one observer instance; clones share it. Failed admission/delivery consumes a
sequence number. Consumers detect gaps and consult publisher loss counters.
IDs and times are uint64; JavaScript consumers must preserve IDs as strings/Long.
Elapsed time is relative to context creation, not a synchronized cross-host clock.
Payload capture is opt-in and consumers must treat it as potentially sensitive.
Unknown schema versions must not be silently interpreted as the current contract.

The schema and generated descriptor are byte-identical to the contract formerly
owned by `telemetry-protocol`. Rust consumers import `workflow_protocol` directly.
No compatibility re-export retains the previous crate dependency. Transport
framing, protobuf package names, field tags and event versions remain unchanged.
