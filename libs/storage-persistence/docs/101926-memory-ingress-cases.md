# Three storage input paths

![Three input paths and the shared MCAP writer lifecycle](architecture/three-path-writer-lifecycle.svg)

Implemented contract · corrected 2026-10-20. Managed reservations, intra-process
slabs and both immutable external inputs are supported. The writable-mmap MCAP
builder has been removed.

## 1 · Managed reservations

`reserve(message, size)` supplies writable payload space. Small records share a
managed combining allocation; large records acquire one contiguous allocation.
The caller encodes directly into that slice. Storage supplies framing, alignment,
file ranges and allocation reuse.

## 2 · Existing external payloads

`append_external_payload(message, lease)` accepts caller-owned immutable bytes.
Storage leases one aligned prefix page containing valid MCAP padding and the
message header in its last 31 bytes. Original SHM bytes immediately follow it in
the file. Both sources are retained through I/O; the payload is never copied.

Preflight source addresses, lengths, registration and capacity. Reserve one
contiguous file range for the whole append. Unsupported layouts return the lease
intact. External inputs bypass combining regardless of size.

## 3 · Already-framed external blocks

`append_external_records(lease)` validates caller-stamped MCAP message
records, channel compatibility and alignment padding, then submits the same bytes.
The producer need not use our builder. File headers, metadata, rotation and
finalization remain writer responsibilities.

The existing intra-process `prepare_records(WritableSlab)` helper remains available.
Its `PreparedBlock` is distinct from the external read-lease entry point.
Each append occupies contiguous offsets. Cross-append placement and completion
order remain unspecified; existing combining batches stay buffered.

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
