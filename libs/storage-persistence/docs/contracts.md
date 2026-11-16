# Writer contracts

- `reserve(message, size)` provides managed payload space; prepared slabs supply a complete uncompressed Chunk, optional indexes and padding.
- `McapWriter` creates each file exclusively with `O_DIRECT` and uses 4-KiB alignment.
- Each write occupies contiguous file offsets; completion order is unspecified.
- Reservations take ascending file offsets in commit order. Prepared and external appends leave combining batches buffered, so they can take lower offsets than earlier commits.
- Registrations survive reuse and rotation. Explicit idle retirement releases them.
- Each combining flush, large message and slab occupies one Chunk. Optional complete per-channel Message Indexes use existing trailing slack, immediately after the Chunk; a residual gap must be zero or at least nine bytes.
- Caller slabs use an omitted CRC, messages on declared channels and indexes in message order per channel. Admission validates framing and submits the original pointer, length and bytes.
- Chunk Index summary storage grows by 73 bytes per chunk and 10 per indexed channel, plus vector capacity, until finish/rotation. Rotation clears entries and reuses capacity.
- Official sequential readers cover mixed indexed/unindexed chunks. Legacy external mmap APIs retain unchunked records; indexed readers cannot cover those messages.
- Rejected prepared blocks return intact; drive `flush_written()` before retrying pressure.
- Slab recycling waits for all readers and I/O. Sealing allocates one `Arc`; recycling uses a mutex.
- MCAP submissions drive completions and flush aged batches on the owner thread.
- `poll()` optionally refreshes idle writers. Idle batches wait for another call.
- `flush_written()` waits for writes; `finish()` finalizes and syncs MCAP. Check both results.
- Rings use SINGLE_ISSUER, DEFER_TASKRUN and TASKRUN_FLAG (Linux 6.1+).
- Inject events and optional stage metrics through `McapConfig`; both default to disabled.
