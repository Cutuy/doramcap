# Opportunistic MCAP indexes — implementation

Created: 2026-11-16. Proposed milestones; production work starts after decision review.
[Architecture and decisions](111626-opportunistic-mcap-indexes.md).

## M0 — Agree on contracts

**Work items**

- [x] Review R1: official sequential-reader contract for mixed files.
- [x] Review R2a: managed/slab header placement and slab sealing.
- [ ] Review R2b: framing for both immutable external APIs.
- [ ] Review R3: evaluate official `mcap` 0.26.0 against the current lease-aware writer before adopting a dependency or extending custom framing.
- [ ] Review R4: summary retention and per-file memory growth.
- [ ] Record approved choices in the architecture document; reconcile affected API docs and consumers.

**Validation**

- [x] Reader capability fixtures.
- [ ] Three-path ownership and external-input review.

## M1 — Chunk framing and padding fit

**Work items**

- [ ] Keep chunk/index encoding private to `mcap`; use the mechanism approved in R3.
- [ ] Visit complete message headers for channels, time bounds and relative offsets; skip payload contents. Preserve the current omitted-CRC policy.
- [ ] Establish the aligned physical extent from chunk framing and messages before trying optional indexes.
- [ ] Attempt complete Message Index serialization within the available trailing bytes; use bounded scratch or an equivalent reversible attempt.
- [ ] If the complete set and valid residual padding fit, emit it immediately after the chunk. Otherwise discard the whole attempt and retain padding.
- [ ] Keep chunk lengths separate from following indexes/padding. Return metadata for summary serialization without assigning file offsets early.
- [ ] Preserve message capacity decisions independently of optional indexes; framing headroom remains explicit in API capacity calculations.

**Validation**

- [ ] Official Rust record decoding.
- [ ] Exact fit and one-byte shortfall.
- [ ] Zero slack and residual gaps of one through eight bytes.
- [ ] Multiple channels and complete-or-absent coverage.
- [ ] Out-of-order timestamps and empty/oversized input.
- [ ] Unchanged extent with indexes present or absent.

## M2 — Integrate the three paths

R2a approves the managed/slab layout. External-input layouts await R2b. Payload
ownership is preserved.

**Work items**

- [ ] Managed combining: reserve chunk-header space before encoding, finalize a complete chunk at `flush_combining()`, then accept one immutable aligned range.
- [ ] Large managed DMA: frame one whole message in its contiguous allocation; reuse only trailing alignment slack for message indexes. Keep split SQEs within the same chunk.
- [ ] Prepared slabs: reserve framing before producer writes, finalize optional indexes while `PreparedRecordsBuilder` still owns writable bytes, then seal. `append_prepared_records` submits unchanged bytes.
- [ ] External payloads: evaluate putting chunk plus message headers at the end of the existing prefix page. With no trailing slack, omit message indexes.
- [ ] External records: implement the approved R2b wrapper or producer contract, preserving published bytes and full chunk coverage. Account separately for any required header-page traffic.
- [ ] Bind prepared-format identity across rotation and reject incompatible blocks intact.
- [ ] Preserve current managed commit order and prepared/external bypass behavior; do not allocate an open combining batch's file range solely to predict index space.
- [ ] Make the index-presence choice before SQ publication. Retain every source through split writes, retries and cancellation.

**Validation**

- [ ] Payload bytes and backing addresses.
- [ ] Fit/omit cases for each path.
- [ ] Mixed appends with one pending slot.
- [ ] Rejected admission and retry.
- [ ] Held readers and delayed completions.
- [ ] GPU readiness and immutable slab handoff.

## M3 — Summary and finalization

**Work items**

- [ ] Capture Chunk Index metadata from actual accepted ranges, including chunks without message indexes. Record valid time bounds and an empty index map/zero length when absent.
- [ ] Reserve summary-bookkeeping capacity before acceptance so allocation failure cannot leave accepted data unrepresented; rollback refused admissions without duplicate entries.
- [ ] Apply R4 retention, expose its growth in API documentation, and reuse original schema/channel declarations.
- [ ] Replace the current fixed 50-byte terminal sequence with DataEnd, grouped summary records, summary offsets, Footer and magic.
- [ ] Stream final records through aligned managed writes without private padding inside summary groups. Trim only bytes after the trailing magic, then sync through existing barriers.
- [ ] Reset per-file bookkeeping only after successful finalization; independently finalize each rotation. Preserve existing open-first rotation failure behavior.
- [ ] Define and test failures after data acceptance, after summary writes and before sync; surface incomplete output explicitly.

**Validation**

- [ ] Summary offsets and lengths.
- [ ] Every accepted chunk represented.
- [ ] Footer pointers and exact EOF.
- [ ] Independent rotated files.
- [ ] Submission, write and finalization failures.

## M4 — Independent readers and integration

**Work items**

- [ ] Extend the existing Rust/TypeScript fixture corpus with all-indexed, no-message-index and mixed chunks across the three paths and both external APIs.
- [ ] Replace assertions requiring unchunked/index-free output with exact payload, index-presence, chunk-range and complete-finalization assertions; retain legacy unchunked readback fixtures.
- [ ] Use official `McapStreamReader` for complete mixed-file readback and scan-based time/topic filtering. Exercise all-indexed files with the official indexed reader separately.
- [ ] Verify the production Rust linear-reader path and existing recorder, monitor, tracing and fault consumers with new output.
- [ ] Compare current and changed paths for padding bytes, allocations, registration reuse, throughput and latency at low rate, bursts and saturation. Separate required chunk-header cost from optional-index cost.
- [ ] Update writer contracts, API comments, README, compatibility fixtures and diagrams to describe implemented behavior after qualification.

**Validation**

- [ ] Fragmented official TypeScript streaming.
- [ ] Rust sequential and summary decoding.
- [ ] Time/topic filtering across omitted indexes.
- [ ] Packaged recording readback.
- [ ] Focused release-profile storage checks.
- [ ] Final-source full local CI.
- [ ] ARM64 serial worker completion.
- [ ] CUDA host slab qualification.

Build unprivileged with locked dependencies and release profiles. Run capability
checks under the repository's test-executable-only elevation rules. Full local CI
and ARM64 work belong to implementation integration; report CUDA/Dora coverage
separately when their paths are exercised. Keep validation results in the PR's
single-line summary; these checklists name gates only.
