# Core contracts

## Records

| Field | Contract |
|---|---|
| Identity | Logical producer + restart epoch + step + opaque generation ID |
| Sequence | Unique ordering number per producer/epoch/step; repeated on every reference |
| Clock | Required domain identifying a shared timestamp origin and nanosecond scale |
| Invocation | Immutable correlation key; empty means unavailable |
| Inputs absent | Dependencies have not arrived yet |
| Inputs empty | Declared source/root generation |
| Start/end | Independently optional; zero is a valid timestamp |

IDs and sequences must remain unique throughout an epoch. The collector detects
conflicts within retained history. Applications choose epochs and global IDs.
Metadata repeats on every patch. Inputs become immutable when first supplied;
input order is insignificant. Duplicate patches are idempotent. Conflicts, cycles,
invalid local durations, oversized frames, and expired updates return errors
without mutating state. One input set applies to the generation's output.

## Probe selection

An anchor instance starts on the generation's first accepted patch. Each selected
generation is frozen; a later timestamp fills its pending boundary.

| Selector | Resolution |
|---|---|
| Consumed ancestor | Follow each path until the nearest matching step; include anchor; require one distinct reference |
| Same invocation | Match target step against the anchor's nonempty invocation key in retained records |
| Previous | Exact anchor sequence minus one, same producer/epoch/step; target must equal anchor step |
| Latest known at anchor | Most recently first-observed retained target generation at anchor arrival |

Unresolved ancestry waits for all branches needed to establish a unique match.
Shared ancestors are deduplicated. Multiple matches at resolution produce
`AMBIGUOUS`. Same-invocation selection freezes once unique; producers must use
invocation keys that uniquely identify the intended target output. Latest-known
uses collector arrival order; it measures availability at that observation point.
Replaying the same ordered input stream reproduces those snapshots.

Missing exact references wait until arrival or retention expiry. Missing
same-invocation matches wait until resolution or anchor eviction. No root match,
an empty invocation key, sequence zero for previous, and an empty latest-known
snapshot produce `NO_MATCH` immediately.

## Results and timing

- One terminal result per definition/anchor; duplicate updates emit nothing.
- Complete pairs contain unsigned `end − start` and both selected references.
- Clock domains must match exactly; domain synchronization is the reporter's responsibility.
- Cross-domain pairs produce `CLOCK_MISMATCH`; reversed pairs produce `INVALID_DURATION`.
- Incomplete results carry a reason and any available selected probes.
- Cached probe timestamps survive record eviction; unresolved required history expires.
- Completed instances retain only bounded deduplication state until anchor eviction.
- No result queue is retained internally. Callers control result storage and delivery.

## Bounds and dispatch

| Resource | Bound / policy |
|---|---|
| Records | N highest sequences per producer/epoch/step |
| Stream metadata | Fixed stream limit including epoch tombstones; reject new streams at capacity |
| Instances | At most retained records × configured definitions |
| Input edges | Configured maximum inputs per generation |
| Traversals | Configured visited-reference limit; fail explicitly on exhaustion |
| Frames / strings / definitions | Validated configured limits |
| Wait indexes | Derived from bounded instances and bounded traversal work |

Stream tombstones prevent old records from re-entering the graph. A collector
restart resets history, definitions, and deduplication; persisted replay must
preserve arrival order. Producer epoch churn eventually reaches the stream cap.

Definition indexes map step/boundary to definitions. Runtime indexes map exact
boundaries, unresolved records, target steps, and missing-reference streams to
waiting instances. Only affected instances are reevaluated. Eviction cleans their
indexes and finalizes affected pending instances. Silent pipelines remain pending
until new input advances retention; deadlines are a future option.

## Protocol and reuse

Package: `doramcap.provenance.v1`; envelopes require version 1. Unknown protobuf
fields are tolerated; unknown/unspecified selectors, boundaries, and policies
are rejected. Preserve field numbers when extending the schema. The compiled
file descriptor is exported as `DESCRIPTOR`.

The implementation reuses the workspace's `prost`, `prost-build`, and vendored
protoc versions. `pbtxt-config` is a future configuration-loader option. The
existing activity context has private same-runtime identity and session checks;
the provenance wire identity is independently usable across processes. No
activity-tracing dependency or context serialization change is introduced.
