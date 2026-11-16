# Opportunistic MCAP indexes — implementation

Created: 2026-11-16. Approved implementation milestones.
[Architecture and decisions](111626-opportunistic-mcap-indexes.md).

Scope: extend the existing per-message write-combine, large-message DMA and slab
paths. Introducing a new write path is forbidden. Revise the existing #3 slab
contract for caller-framed chunks and keep the current shared io_uring writer.

## M0 — Agree on contracts

**Work items**

- [x] Review R1: official sequential-reader contract for mixed files.
- [x] Review R2a: writer-owned #1/#2 buffers and caller-owned, fully framed #3 slabs.
- [x] R3: extend private framing; official `mcap` remains a test decoder and the I/O writer remains unchanged.
- [x] R4: retain serialized Chunk Index records until finish/rotation; document memory growth.
- [x] Record approved choices in the architecture document; reconcile affected API docs and consumers.

**Validation**

- [x] Reader capability fixtures.
- [x] Three-path ownership review.

## M1 — Chunk framing and padding fit

**Work items**

- [x] Keep chunk/index encoding private to `mcap`; use the mechanism approved in R3.
- [x] For writer-owned #1/#2, visit committed message headers for channels, time bounds and relative offsets; skip payload contents and preserve omitted CRCs. For #3, consume supplied framing for validation/summary metadata without rebuilding the bytes.
- [x] For #1/#2, establish the aligned physical extent from chunk framing and messages before trying optional indexes. For #3, validate the caller-supplied aligned extent without resizing it.
- [x] In #1/#2 framing and caller-side helpers, attempt complete Message Index serialization within available trailing bytes; use bounded scratch or an equivalent reversible attempt. Slab admission only validates the supplied choice.
- [x] If the complete set and valid residual padding fit, emit it immediately after the chunk. Otherwise discard the whole attempt and retain padding.
- [x] Keep chunk lengths separate from following indexes/padding. Return metadata for summary serialization without assigning file offsets early.
- [x] Preserve #1/#2 capacity decisions independently of optional indexes. Document mandatory chunk-header headroom for the caller-owned slab contract.

**Validation**

- [x] Official Rust record decoding.
- [x] Exact fit and one-byte shortfall.
- [x] Zero slack and residual gaps of one through eight bytes.
- [x] Multiple channels and complete-or-absent coverage.
- [x] Out-of-order timestamps and empty/oversized input.
- [x] Unchanged extent with indexes present or absent.

## M2 — Integrate the three paths

R2a distinguishes writer-allocated #1/#2 from caller-owned #3. The slab path
submits the caller's complete wire bytes unchanged.

**Work items**

- [x] Managed combining: reserve chunk-header space before encoding, finalize a complete chunk at `flush_combining()`, then accept one immutable aligned range.
- [x] Large managed DMA: accept arbitrary payload lengths. End the chunk immediately after the message payload; put any Message Index directly next, then padding. Split aligned SQEs without inserting bytes into the chunk or indexes.
- [x] Caller-owned slab: revise the existing slab admission contract to accept a caller-supplied Chunk header, messages, optional complete message indexes and padding. Submit exactly the supplied pointer and length.
- [x] Make current framing helpers optional for caller-authored slab bytes; do not require origin from a writer-owned builder. Validate framing and channel compatibility across rotation; return incompatible slabs intact.
- [x] Preserve current managed commit order and prepared-slab bypass behavior; do not allocate an open combining batch's file range solely to predict index space.
- [x] Finalize #1/#2 indexes before SQ publication; the caller finalizes #3 before publication. Keep every submitted slab byte immutable and retain its I/O lease through splitting, retries and cancellation.

**Validation**

- [x] Payload bytes and backing addresses.
- [x] Fit/omit cases for each path.
- [x] Arbitrary-length #2 payload and immediately adjacent index.
- [x] Caller-authored #3 chunk without the writer framing helper.
- [x] Mixed appends with one pending slot.
- [x] Rejected admission and retry.
- [x] Held readers and delayed completions.
- [x] GPU readiness and caller-owned slab handoff on a CUDA host.
- [x] Slab allocator, byte identity and recycle ownership.

## M3 — Summary and finalization

**Work items**

- [x] Capture Chunk Index metadata from actual accepted ranges, including chunks without message indexes. Record valid time bounds and an empty index map/zero length when absent.
- [x] Reserve summary-bookkeeping capacity before acceptance so allocation failure cannot leave accepted data unrepresented; rollback refused admissions without duplicate entries.
- [x] Apply R4 retention, expose its growth in API documentation, and reuse original schema/channel declarations.
- [x] Replace the current fixed 50-byte terminal sequence with DataEnd, grouped summary records, summary offsets, Footer and magic.
- [x] Stream final records through aligned managed writes without private padding inside summary groups. Trim only bytes after the trailing magic, then sync through existing barriers.
- [x] Reset per-file bookkeeping only after successful finalization; independently finalize each rotation. Preserve existing open-first rotation failure behavior.
- [x] Exercise summary submission, truncate and sync errors through McapWriter; retain existing writer fault checks.

**Validation**

- [x] Summary offsets and lengths.
- [x] Every accepted chunk represented.
- [x] Footer pointers and exact EOF.
- [x] Independent rotated files.
- [x] Format-level submission/truncate/sync failures and writer write failures.

## M4 — Independent readers and integration

**Work items**

- [x] Extend the existing Rust/TypeScript fixture corpus with all-indexed, no-message-index and mixed chunks across the three existing paths.
- [x] Replace assertions requiring unchunked/index-free output with exact payload, index-presence, chunk-range and complete-finalization assertions; retain legacy unchunked readback fixtures.
- [x] Use official `McapStreamReader` for complete mixed-file readback and scan-based time/topic filtering. Exercise all-indexed files with the official indexed reader separately.
- [x] Keep existing slab consumers on the updated optional helper; verify unchanged caller backing and GPU payload-offset calculation. Compile the GPU producer and exercise Rust verifier, recorder, monitor, tracing and fault consumers with new output.
- [x] Compare current and changed paths for padding bytes, allocations, registration reuse, throughput and latency at low rate, bursts and saturation. Separate required chunk-header cost from optional-index cost; [samples and driver](benchmarks/111626-index-cost.md).
- [x] Update writer contracts, API comments, README, compatibility fixtures and diagrams to describe implemented behavior after qualification.

**Validation**

- [x] Fragmented official TypeScript streaming.
- [x] Rust sequential and summary decoding.
- [x] Time/topic filtering across omitted indexes.
- [x] Packaged recording readback.
- [x] Focused release-profile storage checks.
- [x] Final-source full local CI.
- [x] ARM64 serial worker completion.
- [x] CUDA host slab qualification.

Build unprivileged with locked dependencies and release profiles. Run capability
checks under the repository's test-executable-only elevation rules. Full local CI
and ARM64 work belong to implementation integration; report CUDA/Dora coverage
separately when their paths are exercised. Keep validation results in the PR's
single-line summary; these checklists name gates only.
