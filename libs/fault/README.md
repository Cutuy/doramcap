# Fault libraries

Bounded fault capture, state aggregation and Protobuf publication for host-managed event loops.

![Architecture](protocol/docs/fault-event-architecture.svg)

| Crate | Responsibility |
|---|---|
| [fault-reporter](reporter/) | Capture and prioritized draining |
| [fault-aggregation](aggregation/) | Health, freshness and injected diagnostics |
| [fault-schema](schema/) | Generated Protobuf contracts and enums |
| [fault-protocol](protocol/) | Checked record translation and byte codecs |

[State transitions and workflow](protocol/docs/102026-event-driven-faults.md). Cargo package names remain `fault-*`.
