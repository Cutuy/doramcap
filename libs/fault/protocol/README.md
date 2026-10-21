# Fault protocol

Versioned Protobuf publications and bidirectional translation for the reporter and aggregation crates. Serialization runs outside reporter callbacks.

| Boundary | Schema | Payloads |
|---|---|---|
| Reporter → watchdog IPC | [reporter.proto](../schema/proto/reporter.proto) | Observation batch or definition/latest-state snapshot |
| Watchdog → storage / viewer | [aggregation.proto](../schema/proto/aggregation.proto) | Revisioned update batch or full aggregation snapshot |
| Shared records | [common.proto](../schema/proto/common.proto) | Identity, policy, observations, counters and trace context |

## Translation

- `Wire::to_proto` / `Wire::from_proto`: checked conversion for each internal record.
- `encode_reporter` / `decode_reporter`: `ReporterFrame` ↔ Protobuf bytes.
- `encode_aggregation` / `decode_aggregation`: `AggregationFrame` ↔ Protobuf bytes.
- `pb`: generated types re-exported from `fault-schema`; both cores use these enums.

Decode reporter snapshots into `Aggregator::reconcile` and batches into `Aggregator::ingest_batch`. Encode `pop_update()` records as `Updates`; encode `snapshot()` as an aggregation snapshot. The host supplies publication timestamps and drains ready updates in revision order. Urgent records publish without a batch-fill delay.

See the [runnable example](examples/publications.rs) and [wire contract](docs/102026-fault-publications.md).

## MCAP

Register `FILE_DESCRIPTOR_SET` with schema encoding `protobuf`. Use `AGGREGATION_SCHEMA_NAME` (`doramcap.fault.v1.AggregationPublication`) and message encoding `protobuf` for aggregation history and snapshots. Persist the encoded publication bytes unchanged. `REPORTER_SCHEMA_NAME` is available for optional raw-input recording.

Snapshots support display/replay reconciliation. They are not complete aggregator runtime checkpoints: deduplication windows and expiry bookkeeping remain internal.

```sh
cargo test -p fault-protocol --locked
cargo run -p fault-protocol --example publications --locked -- /tmp/fault-fixtures
```

This crate creates no Dora subscriptions, sockets or MCAP files. Those remain host-app responsibilities.

[Event-driven architecture and state transitions](docs/102026-event-driven-faults.md).
