![Architecture](../../fault-watchdog/docs/production-watchdog.svg)

# Fault reporting roadmap

Created 2026-10-20. Detailed app milestones and evidence live in the
[production watchdog plan](../../fault-watchdog/docs/102026-production-watchdog.md).

## Milestones

- [x] Reporter: separate bounded outcome queues, optional deadlines, immediate failure and one-second recovery.
- [x] Aggregation: deterministic health/freshness, deduplication, revisioned updates and snapshots.
- [x] Shared Protobuf IPC/storage contracts and checked translation.
- [x] Production watchdog: configured Dora host, bounded fan-out and optional MCAP/OTel adapters.
- [x] Demo producers and dashboard: controls, independent freshness and snapshot reconciliation.
- [x] Complete real-Dora integration and ARM64 validation of the app composition.

## Ownership

| Component | Responsibility |
|---|---|
| Reporter | Capture outcomes; host event loop publishes |
| Aggregation | State transitions and domain output |
| Watchdog | Config, Dora lifecycle, queues and optional sinks |
| MCAP adapter | Existing storage writer, schema, rotation and finalization |
| Demo | Synthetic operations, controls and browser display |

## Detailed plans

- [Reporter](../../../libs/fault/reporter/docs/102026-fault-reporter.md)
- [Aggregation](../../../libs/fault/aggregation/docs/102026-fault-aggregation.md)
- [Wire contract](../../../libs/fault/protocol/docs/102026-fault-publications.md)
- [State transitions](../../../libs/fault/protocol/docs/102026-event-driven-faults.md)
- [Production app](../../fault-watchdog/docs/102026-production-watchdog.md)

## Evidence

Core library validation and ARM64 jobs 88–93 are recorded in the library plans.
The app's ten focused tests, individual/combined feature tests, strict Clippy and
rustdoc pass locally. The production plan tracks final ARM64/Dora results.

Dependencies were approved before implementation: existing Crossbeam, Dora, Prost,
Tokio/Axum and official OTel 0.33 SDK/OTLP HTTP export. MCAP uses `storage-persistence`.

ARM64 workspace/native job 95 and Dora/MCAP/OTLP job 100 passed. Final-source job 101 also passed ten focused tests and Dora/MCAP/OTLP validation, including the shutdown-isolation fix.
