# Opportunistic MCAP indexes — implementation

Created: 2026-11-16. Four approved PRs, in landing order.
[Architecture and decisions](111626-opportunistic-mcap-indexes.md).

Keep the three existing write paths and shared io_uring writer. Each PR includes
its callers, API documentation and tests. Introduce no runtime dependency,
environment input, format option or compatibility layer.

## PR 1 — Accept buffers prepared by callers

**Work items**

- [x] Add `PreparedBlock::from_slab`; remove the originating-writer requirement.
- [x] Validate supplied message records, declared channels and alignment before admission.
- [x] Preserve the supplied pointer, length, bytes and reader/I/O ownership.
- [x] Keep the existing message-record file layout and optional framing helper.

**Validation**

- [x] Independently authored buffers.
- [x] Malformed input and rejected ownership.
- [x] Compatible writers and backpressure.
- [x] Held readers, reuse and rotation.

## PR 2 — Write file endings

**Work items**

- [x] Retain original declaration boundaries for grouped schema/channel summaries.
- [x] Replace the fixed ending with DataEnd, summaries, Summary Offsets and Footer.
- [x] Stream aligned final writes and truncate only after trailing magic.
- [x] Preserve open-first rotation and report submission, truncate and sync failures.
- [x] Keep messages outside Chunks until the next PR.

**Validation**

- [x] Official summary and sequential readback.
- [x] Multi-write summaries and exact EOF.
- [x] Independently readable rotated files.
- [x] Finalization failures.
- [x] Rust and TypeScript compatibility fixtures.

## PR 3 — Add batch headers and record batch locations

**Work items**

- [x] Put one uncompressed Chunk header before each combining batch, large message and slab.
- [x] Preserve arbitrary payload lengths, backing memory and existing acceptance order.
- [x] Update caller framing helpers, slab admission and GPU allocation sizing together.
- [x] Validate supplied Chunk framing, declared channels, time bounds and padding.
- [x] Stage one Chunk Index before admission; patch actual offsets and roll back rejection.
- [x] Retain serialized Chunk Indexes until finish/rotation and include them in summaries.
- [x] Document per-file memory growth and reuse capacity after rotation.
- [x] Omit Message Indexes; keep remaining alignment space as padding.

**Validation**

- [x] Three-path payload and address preservation.
- [x] Alignment boundaries and arbitrary large payloads.
- [x] Caller-authored Chunks and malformed input.
- [x] Pending slots, retries, held readers and reuse.
- [x] Chunk ranges, summary pointers and rotation.
- [x] Crash prefixes and finalization failures.
- [x] Official Rust and TypeScript sequential readers.

## PR 4 — Store message locations in leftover padding

**Work items**

- [x] Determine write extents from required headers and messages before attempting indexes.
- [x] Serialize every present channel's complete Message Index, or discard the whole attempt.
- [x] Keep indexes immediately after their Chunk; leave zero bytes or valid private padding.
- [x] Bound scratch by existing slack and scan message headers without reading payload contents.
- [x] Validate caller-supplied indexes without rewriting slab memory.
- [x] Extend Chunk Index maps with accepted Message Index locations.
- [x] Preserve admission rollback, summary streaming and per-file capacity reuse.
- [x] Cover mixed indexed/unindexed files with official sequential readers.
- [x] Preserve existing cost samples and report qualification separately.

**Validation**

- [x] Exact fits, shortfalls and residual gaps.
- [x] Multiple channels and reversed timestamps.
- [x] Complete indexes or none, with unchanged write extents.
- [x] Slab byte identity and malformed indexes.
- [x] Mixed-file filtering and all-indexed readback.
- [x] Existing low-rate, burst and saturated cost samples.
- [x] Final-source local integration checks.
- [x] ARM64 integration checks.
- [x] CUDA slab readback.

Build unprivileged with locked dependencies and release profiles. Elevate only
capability-requiring test executables. Report local, ARM64 and CUDA/Dora evidence
separately; prior final-source evidence does not validate intermediate PRs.
