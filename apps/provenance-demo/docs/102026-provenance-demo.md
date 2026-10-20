![Provenance demo architecture](102026-provenance-demo.svg)

# Dora provenance lab

Created: 2026-10-20. Status: proposal for architecture discussion. App implementation awaits that discussion.

## M1 — Agree on architecture

- [ ] Agree on separate camera, lidar, detection, control, collector, and web-gateway Dora nodes.
- [ ] Run camera at 20 Hz and lidar at 10 Hz; detection records the exact reused lidar generation it consumes.
- [ ] Instrument pipeline nodes with both step records and activity-tracing runtime spans.
- [ ] Show step and activity timelines side by side, plus latency plots; share chronological filtering, zoom, pan, follow-live, and selection.
- [ ] Keep activity/provenance types independent; attach generation fields to activity spans for optional selection links.
- [ ] Start on one Linux host. Carry explicit activity-session → host-monotonic clock mappings with uncertainty; surface unavailable alignment.
- [ ] **Validation gate:** record the architecture decisions from discussion before app implementation.

## M2 — Multi-process reporting and collection

- [ ] Define application payloads carrying `GenerationRef`; publish independent start/end protobuf patches over Dora.
- [ ] Use node-owned Dora publication; activity capture sinks enqueue into bounded per-node queues drained by the node event loop.
- [ ] Feed fixed definitions into the core evaluator; publish accepted step records and terminal results to the gateway.
- [ ] Forward activity batches independently; preserve producer/session identities and stream-specific gap counts.
- [ ] Add per-session clock descriptors or an additive activity protocol extension; verify relative activity timestamps map to the common monotonic timeline.
- [ ] Inject processing delays, report delays/loss, and node restarts through reproducible scenarios.
- [ ] **Validation gate:** verify exact cross-process ancestry, reused lidar frames, split timestamps, clock alignment, and incomplete results under loss.

## M3 — Gateway and frontend

- [ ] Build typed protobuf WebSocket envelopes for steps, activities, results, clock descriptors, and gap notices.
- [ ] Bound replay by bytes and event count; define reconnect and slow-client behavior. Preserve ordering for anchor-time selectors.
- [ ] Maintain separate browser stores for step records, activity sessions, and measurement results.
- [ ] Drive all views from one viewport model; apply the same chronological filter and zoom anchor to both timelines and latency points.
- [ ] Link a latency point to its selected probes, a step to its ancestors, and an activity span to its tagged generation.
- [ ] Retain 64-bit IDs/timestamps exactly; convert only viewport-relative time deltas to plotting numbers.
- [ ] **Validation gate:** browser tests verify synchronized zoom/filtering, cross-selection, reconnect deduplication, bounded retention, and visible gaps.

## M4 — Predefined measurements and acceptance

| Measurement | Anchor | Start probe | End probe |
|---|---|---|---|
| Capture → detection | Detection generation | Consumed camera start | Anchor end |
| Capture → control | Control generation | Consumed camera start | Anchor end |
| Lidar age → control | Control generation | Consumed lidar start | Anchor end |
| Detection duration | Detection generation | Anchor start | Anchor end |

- [ ] Load fixed protobuf definitions; display per-sample duration, selected references, and incomplete counts.
- [ ] Set expected delays in deterministic scenarios and compare measured values with source timestamps.
- [ ] Exclude incomplete samples from duration statistics; display their reasons separately.
- [ ] **Validation gate:** record multi-process Dora end-to-end results, frontend tests/build, browser checks, and verified ARM64 CI job results.

## Reuse candidates for evaluation

| Existing component | Candidate reuse |
|---|---|
| `apps/activity-demo` | React timeline interaction, protobuf activity decoding, bounded replay/gap handling |
| `apps/cpu-viewer` | Dora subscriber + Axum gateway; uPlot latency-series rendering |
| `libs/activity-tracing` | Per-process capture and custom sink interface; generation fields on spans |
| `libs/step-provenance` | Protobuf contracts, bounded graph, fixed pair-measurement engine |

Existing crates/packages to evaluate before app adoption: `dora-node-api`, `axum`,
`tokio`, `prost`, `tracing`, `protobufjs`, React, and uPlot. Prefer small extracted
viewport/decoder helpers after checking behavior in the existing apps.

## Discussion points

- Proposed layout: equal-width step and activity timelines, with a latency panel sharing the same time range.
- Proposed clocks: same-host monotonic measurement domain; explicit per-session mapping for activity-relative time. Current activity batches provide a Unix origin and relative time, so exact monotonic alignment needs additional clock metadata.
- Proposed correlation: generation identity fields on spans; each activity stream retains its own runtime session ID.
- Temporary measurements, min/max aggregations, and multi-host clock synchronization remain later work.

## Evidence

Inspected activity-demo's `LiveSink`, browser `TraceStore` and `zoomTimeline`, and
cpu-viewer's Dora/HTTP gateway and frontend dependencies. The activity decoder
currently resets on a new session; the demo needs a multi-session store. No app
code has been implemented; all milestone gates remain unchecked. Both architecture
SVGs were rendered with the installed librsvg/Cairo libraries and visually inspected;
local Markdown links resolve.
