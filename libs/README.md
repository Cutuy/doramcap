# Libraries

Reusable crates live directly here or within a two-level domain group. Each
crate owns its manifest, API documentation, tests, and examples. Group directories
provide an index; Cargo package names stay independent of directory names.

| Group or crate | Scope |
|---|---|
| [orchestrator](orchestrator/) | Observable local async Rust steps and external-script adapter |
| [workload-lifecycle](workload-lifecycle/) | Foreign launch adoption and execution boundaries (design scaffold) |
| [session](session/) | Session identity and experiment evidence contracts (concept scaffold) |
| [resources](resources/) | Resource measurement, accounting, placement, and scheduling policy |
| [telemetry](telemetry/) | Publication contracts, codecs, and telemetry recording |
| [memory](memory/) | Memory allocation, mapping, sharing, and access lifetimes across devices |
| [fault](fault/) | Fault capture, health aggregation, and publication contracts |
| [activity-tracing](activity-tracing/) | Bounded span/event capture and output sinks |
| [step-provenance](step-provenance/) | Output ancestry and event-driven latency evaluation |
| [storage-persistence](storage-persistence/) | Bounded io_uring persistence and MCAP writing |
| [pbtxt-config](pbtxt-config/) | Typed Protobuf TextFormat configuration composition |

See [repository conventions](../docs/repository-layout.md). Run a crate's tests
from the workspace root, for example `cargo test -p telemetry-protocol`.
