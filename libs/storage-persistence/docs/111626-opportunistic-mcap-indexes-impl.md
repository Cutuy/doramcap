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

- [ ] Retain original declaration boundaries for grouped schema/channel summaries.
- [ ] Replace the fixed ending with DataEnd, summaries, Summary Offsets and Footer.
- [ ] Stream aligned final writes and truncate only after trailing magic.
- [ ] Preserve open-first rotation and report submission, truncate and sync failures.
- [ ] Keep messages outside Chunks until the next PR.

**Validation**

- [ ] Official summary and sequential readback.
- [ ] Multi-write summaries and exact EOF.
- [ ] Independently readable rotated files.
- [ ] Finalization failures.
- [ ] Rust and TypeScript compatibility fixtures.

## PR 3 — Add batch headers and record batch locations

**Work items**

- [ ] Put one uncompressed Chunk header before each combining batch, large message and slab.
- [ ] Preserve arbitrary payload lengths, backing memory and existing acceptance order.
- [ ] Update caller framing helpers, slab admission and GPU allocation sizing together.
- [ ] Validate supplied Chunk framing, declared channels, time bounds and padding.
- [ ] Stage one Chunk Index before admission; patch actual offsets and roll back rejection.
- [ ] Retain serialized Chunk Indexes until finish/rotation and include them in summaries.
- [ ] Document per-file memory growth and reuse capacity after rotation.
- [ ] Omit Message Indexes; keep remaining alignment space as padding.

**Validation**

- [ ] Three-path payload and address preservation.
- [ ] Alignment boundaries and arbitrary large payloads.
- [ ] Caller-authored Chunks and malformed input.
- [ ] Pending slots, retries, held readers and reuse.
- [ ] Chunk ranges, summary pointers and rotation.
- [ ] Crash prefixes and finalization failures.
- [ ] Official Rust and TypeScript sequential readers.

## PR 4 — Store message locations in leftover padding

**Work items**

- [ ] Determine write extents from required headers and messages before attempting indexes.
- [ ] Serialize every present channel's complete Message Index, or discard the whole attempt.
- [ ] Keep indexes immediately after their Chunk; leave zero bytes or valid private padding.
- [ ] Bound scratch by existing slack and scan message headers without reading payload contents.
- [ ] Validate caller-supplied indexes without rewriting slab memory.
- [ ] Extend Chunk Index maps with accepted Message Index locations.
- [ ] Preserve admission rollback, summary streaming and per-file capacity reuse.
- [ ] Cover mixed indexed/unindexed files with official sequential readers.
- [ ] Record three-path cost samples and final qualification separately from implementation.

**Validation**

- [ ] Exact fits, shortfalls and residual gaps.
- [ ] Multiple channels and reversed timestamps.
- [ ] Complete indexes or none, with unchanged write extents.
- [ ] Slab byte identity and malformed indexes.
- [ ] Mixed-file filtering and all-indexed readback.
- [ ] Low-rate, burst and saturated costs.
- [ ] Final-source local integration checks.
- [ ] ARM64 integration checks.
- [ ] CUDA slab and Dora qualification.

Build unprivileged with locked dependencies and release profiles. Elevate only
capability-requiring test executables. Report local, ARM64 and CUDA/Dora evidence
separately; prior final-source evidence does not validate intermediate PRs.
