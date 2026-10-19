# Three storage input paths

![Three input paths and the shared MCAP writer lifecycle](architecture/three-path-writer-lifecycle.svg)

Discussion · 2026-10-19. Diagrams show direct-mode submission. Managed reservations
and intra-process prepared blocks are implemented. Large external payloads and
shared-mapping adapters remain planned.

## 1 · Managed reservations

`reserve(message, size)` supplies writable payload space. Small records share a
managed combining allocation; large records acquire one contiguous allocation.
The caller encodes directly into that slice. Storage supplies framing, alignment,
file ranges and allocation reuse.

## 2 · Large external payloads

`append_external_payload(message, lease)` would accept existing payload bytes.
Above 256 KiB, storage builds managed framing spans and submits the original
external range with no payload copy. Inputs at or below the threshold use the
ordinary copy-and-combine policy.

For the aligned single-payload example, MCAP padding precedes the message header
inside the managed prefix; the header immediately precedes the payload in the
file. Every transfer must satisfy address, length and file-offset alignment.
Unsupported layouts fail explicitly. A logical append preflights all its spans
and reserves their file range before accepting ownership.

## 3 · Prepared external blocks

`append_prepared_records(block)` accepts sealed, complete MCAP records plus
valid alignment padding. The producer chooses block size and batching. A 4-KiB
aligned block can take this path; no 256-KiB threshold or storage combining applies.
Storage submits the same bytes and retains the block's read lease through I/O.

`prepare_records(WritableSlab)` supplies a builder for headers, payloads and
padding. Sealing produces `PreparedBlock`, bound to the originating writer's
schema/channel identity across rotation. Other writers reject it intact.
Storage retains responsibility for file headers, metadata, rotation and finalization.
Each submitted piece occupies one contiguous file range. Large managed and prepared
writes leave combining batches buffered; file placement and completion order across
pieces are unspecified. Queue pressure returns the prepared block intact for retry.

## Shared mechanism

The shared design uses a mapping-validity guard for persistent registration and
a separate read/reuse lease for each submission. Registration retains mapping
identity; read leases keep submitted bytes immutable until every referencing
CQE retires. Tracing readers can share the sealed-block lease. The last reader
and I/O release returns the block to its producer pool, while registration persists.

The underlying writer accepts managed or external byte ranges and owns file-offset
allocation, admission, registration, SQEs, retries and CQE retirement. Prepared
blocks bypass MCAP construction and combining through a thin format admission
step. Backend support must establish DMA eligibility for the actual memory type.

Related: [large external-memory plan](101826-external-buffer-leases.md),
[tracing slab ownership](../../activity-tracing/docs/external-buffer-ownership.svg).

## Intra-process slab ownership

`SlabPool::acquire()` supplies unique write access. `WritableSlab::seal(len)`
produces shared immutable bytes; MCAP callers use the framing builder's `seal()`.
`PreparedBlock::slab()` exposes the same whole-slab read lease held by storage.
All readers and I/O must finish before recycling. Payload views are deferred.

Call `prepare_slabs(&pool)` before capture to warm registration. Cache hits use
stable allocation identity and retain resolved registration segments in pending
writes. `registration_updates()` counts table-update attempts; warmed reuse must
leave it unchanged. `retire_slabs(&pool)` requires no pending writer requests and
releases that pool's registrations while existing readers remain valid.
