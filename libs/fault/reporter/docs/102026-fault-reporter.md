![Architecture](fault-reporting.svg)

# Fault reporter library

Created 2026-10-20. Implementation: `libs/fault/reporter`. Transport-independent core; Dora adapter runs outside caller callbacks.

## Milestones

- [x] Finalize API, condition identities, event-only mode and deadline registration.
- [x] Select dependencies after user evaluation; define allocation and concurrency budgets.
- [x] Implement independent bounded failure and success queues.
- [x] Implement local condition state, sequencing, loss counters and recovery reconciliation.
- [x] Implement optional success rate limiting and fair draining.
- [x] Implement Rust batch/snapshot handoff, idle summary scheduling and producer closure.
- [ ] Integrate event-driven transport publication and transport shutdown deadlines in the future host app (deferred).

## Validation

- [x] Duplicate observations produce one transition and accurate repetition summaries.
- [x] Success floods cannot consume failure queue capacity.
- [x] Failure floods cannot prevent recovery/freshness delivery indefinitely.
- [x] Concurrent callers preserve unique sequences; aggregation tolerates urgent/repeat reordering.
- [x] Rate-limited successes preserve first recovery and periodic freshness.
- [x] Queue overflow and reconnect snapshots expose losses and restore current state.
- [ ] Measure call latency, allocations and memory at idle, burst and sustained overload.
- [x] Verify idle summary deadlines and closed-handle rejection.
- [ ] Verify transport failure and transport flush deadlines during app integration (deferred).

## Caller contract

Operations: register a condition, report success, report failure, emit a standalone failure event, and close a source. Low-level libraries return `Result`; the owner chooses recovery and reports the outcome. Reporting does not convert a failed operation into a successful result.

| Mode | Semantics |
|---|---|
| Stateful, on demand | Failure activates; success evidence spanning the recovery interval clears; silence leaves state unchanged |
| Stateful, periodic | Same transitions plus a configured observation deadline; silence marks evidence stale |
| Event only | Records an occurrence; no implied recovery or persistent active condition |

Idempotency means repeated observations do not repeat transitions. A reporting window is a separate optional freshness policy. Recommend stateful reporting by default and deadlines only for checks expected to run regularly.

Condition key: source instance + component + resource + stable fault code. Source instance changes on restart. Bound condition registration and context sizes; reject excess registrations explicitly.

## Separate queues

- Failure observations enter the failure queue; success observations enter the success queue. Configure independent capacities.
- Assign a monotonically increasing per-condition sequence at the same synchronization point as local state updates, before queue routing.
- Maintain bounded latest state independently of queue admission. Return an explicit admission/coalescing/drop outcome without blocking on transport.
- Count overflow separately for successes and failures. A transient failure lost before recovery remains visible as loss; a snapshot cannot recreate its full history.
- Prioritize failures with a finite burst quota, then service successes. Define a maximum recovery/freshness delay under the supported load envelope.
- Publish producer snapshots on reconnect and periodically, so dropped transitions can reconcile. Snapshot generation is bounded and consistent per condition, with per-condition sequences.

## Optional rate limiting

Disabled by default. Configure success refresh limiting per condition; account every call in local state before deciding publication.

- First observation, failure-to-success transitions and recovery confirmation bypass the success refresh limiter.
- Coalesce repeated healthy observations into count + latest sequence + latest observation time.
- Flush pending summaries on a timer even when callers become idle.
- For periodic checks, the library rejects success intervals at or above the deadline. The host budgets polling latency and transport allowance separately.
- Start with failures unthrottled at admission; bounded queues still impose explicit loss. Any later failure coalescing must preserve activation transitions and counts.
- Rate limiting controls publication volume; it does not guarantee constant total CPU cost for arbitrarily frequent calls.

## Publication contract

Batches carry source instance, sequence/watermark, observation timestamps, condition identities, observations and cumulative loss counters. Preserve optional trace correlation and bounded diagnostic context. The [reporter Protobuf contract](../../schema/proto/reporter.proto) supplies the wire representation.

Caller threads perform no disk/network I/O. Adapter workers drain ready records immediately, report transport health locally, and retain only bounded pending data. Retrying uses stable identities; consumers deduplicate. Shutdown has a deadline and returns unsent/lost counts; queue admission does not promise persistence.

## Evidence

2026-10-20: specification drafted; no runtime claims validated.

## Implemented scope and evidence

2026-10-20: Crossbeam approved by the user; `crossbeam-queue = "0.3.12"` resolves to existing lockfile version 0.3.14. Shared records are Rust types owned by `fault-reporter`. Transport/schema adapters, demo apps and MCAP integration are deferred by agreement.

Defaults: deadlines disabled, success limiting disabled, recovery interval one second. First failure activates immediately; recovery requires a later success observation spanning the interval. Timers never clear faults. Per-condition evidence includes counts, first/last failure times and longest success span since the latest failure.

Local validation: `cargo test -p fault-reporter -p fault-aggregation --locked` passed 26 tests, including concurrent producers, all permutations of a four-observation sequence, dropped failures, idle coalescing, stale evidence, restart reconciliation and output bounds. Scoped allocation test observed zero allocations across 30,000 report calls including queue overflow. Strict all-target/all-feature Clippy passed.

Transport deadlines/retries remain host responsibilities. `tick`, `batch`, `snapshot`, `close` and `final_snapshot` provide the core handoff; no hidden worker or unbounded transport retry exists. Producer callbacks use short per-condition mutexes and can wait for concurrent callers/snapshots.

Release smoke benchmark (100,000 calls per case, local x86_64; scheduling and clock-read overhead included):

| Case | p50 | p99 | Queue losses |
|---|---|---|---|
| Drain each call | 89 ns | 112 ns | 0 |
| Drain each 128 calls | 66 ns | 94 ns | 0 |
| No drain | 56 ns | 70 ns | 49,744 per queue |
| Limited successes | 51 ns | 53 ns | 0; 99,999 coalesced |

`Observation` is 368 bytes on this host. The original two-queue payload storage was 188,416 bytes, plus Crossbeam slot metadata, registered condition storage, locks and allocation overhead. Snapshot/batch vectors allocate on the publisher path. These measurements are a smoke baseline, not a latency guarantee. Reproduce with `cargo run -p fault-reporter --release --example fault_reporter_throughput --locked`.

ARM64 CI job **88** succeeded on 2026-10-20. Snapshot: `/home/doramcap-ci/jobs/5a6e4601ccb2492dbb129969463b355a`. It exercised the implemented core with 25 new tests, workspace default/all-feature tests and native validation. The subsequent independent-source restart regression passed locally (26 total), as did the runnable lifecycle example, final formatting and strict Clippy. Runtime source was unchanged after submission.

2026-10-20 follow-up: [fault-protocol](../../protocol/README.md) now provides the requested Protobuf definitions and checked bidirectional translation. Dora transport and storage host adapters remain deferred.

## Event-driven follow-up

- [x] Reserve urgent capacity separately from repeat traffic in each outcome lane.
- [x] Signal registration, records, summary scheduling, overflow and close.
- [x] Share the recovery predicate with aggregation; bypass limiting on confirmation.
- [x] Expose reconciliation snapshots and deadline-aware `wait`.

Evidence: [event-driven implementation and measurements](../../protocol/docs/102026-event-driven-faults.md). Current defaults reserve 1,024 total record slots (376,832 payload bytes on x86_64), plus queue metadata and condition storage. Failure repeats remain unthrottled; success summaries are optional.

## API documentation

- [x] Reduce README to purpose, architecture SVG, minimal example and reference links.
- [x] Put behavior, parameters, returns/errors, ownership and scheduling contracts in Rust doc comments.

2026-10-20 evidence: strict rustdoc (`RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps -p fault-reporter -p fault-aggregation`), both runnable crate examples, formatting and local documentation links passed. This follow-up changes documentation only.
