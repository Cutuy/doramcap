![Architecture](fault-reporting.svg)

# Fault reporting roadmap

Created 2026-10-20. Core crates implemented; app integration remains planned. Detailed implementation plans now live with their library crates.

## Milestones

- [x] Agree on core observation semantics, bounds and Crossbeam; defer transport/schema selection to app integration.
- [x] Implement `libs/fault/reporter`: caller API, separate queues, local state and Rust publication handoff. Transport adapters deferred.
- [x] Implement `libs/fault/aggregation`: deterministic state machine, freshness and snapshots.
- [ ] Add `apps/fault-watchdog`: Dora ingress, scheduling and bounded output routing.
- [ ] Add `apps/fault-demo`: synthetic producers and live fault display.
- [ ] Add demo MCAP recording through a recorder consumer; choose existing recorder extension versus separate app during integration.

## Validation

- [ ] Record library tests and overload benchmarks in the detailed plans.
- [ ] Verify Dora delivery, reconnect and aggregator restart behavior end to end.
- [ ] Verify frontend snapshot/update reconciliation and stale-source display.
- [ ] Verify MCAP schema, timestamps, event identity, finalization and injected write errors.
- [ ] Run repository checks and ARM64 CI; record job IDs/results. Run Dora end-to-end tests separately.

## Ownership

| Component | Owns |
|---|---|
| Reporter library | Nonblocking observations, bounded success/failure queues, sequencing and loss visibility |
| Aggregation crate | Fault lifecycle, deduplication, freshness, bounded state and output records |
| Watchdog app | Dora subscriptions, clocks/timers, process lifecycle and output queues |
| Demo frontend | Current state, history and loss/staleness indicators |
| Recorder consumer | Batching, MCAP schemas/channels, rotation and storage error reporting |

Implemented scope includes reporter, aggregation and Protobuf protocol crates. Apps and recording remain follow-up work.

## Decisions proposed

- Stateful checks report success or failure. Duplicate observations update evidence without repeating state transitions.
- Periodic checks may declare a deadline. Silence becomes stale/unknown; it never proves recovery.
- Event-only failures remain available for one-off diagnostics and do not automatically create latched conditions.
- Use Dora for initial transport, with transport-independent library APIs.
- Persist transitions and repetition/loss summaries. Frontends also receive snapshots.
- A recorder failure uses the reporting path; recurring errors from persisting that failure are suppressed and counted.
- External supervision owns Dora/process/host failure detection.

## Dependency evaluation gate

Before implementation, present maintained candidates and measured tradeoffs for user evaluation: existing `crossbeam-queue`/`crossbeam-channel` for bounded handoff, `prost` for the shared Protobuf contract, and the existing `dora-node-api` adapter. Check current maintenance and repository versions then. Evaluate standard OTel LogRecords versus a dedicated fault envelope before selecting the wire schema; OTel export is optional.

## Detailed plans

- [Reporter library](../../../libs/fault/reporter/docs/102026-fault-reporter.md)
- [Aggregation crate](../../../libs/fault/aggregation/docs/102026-fault-aggregation.md)

## Evidence

2026-10-20: repository layout and activity schema/exporter inspected. No implementation or runtime validation completed.

## Scope decision

2026-10-20: user approved Crossbeam and transport-independent Rust records, deferring Protobuf/Dora adapters and demo apps. Use separate queues, optional deadlines, immediate failure activation and one-second recovery hysteresis. The core supports snapshots and batches; host apps own scheduling and transport flush.

Core evidence: 26 local tests, allocation validation, strict Clippy, formatting and lifecycle example passed. ARM64 CI job 88 passed the submitted runtime sources. Detailed plans record test scope and measurement limits.

## Publication schemas

The [fault-protocol crate](../../../libs/fault/protocol/README.md) now defines reporter IPC and aggregation storage/publication envelopes with checked bidirectional translations. See its [wire contract](../../../libs/fault/protocol/docs/102026-fault-publications.md). Actual Dora and MCAP adapters remain follow-up work.

[Proposed event-driven architecture and state transitions](../../../libs/fault/protocol/docs/102026-event-driven-faults.md).
