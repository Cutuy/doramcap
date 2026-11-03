# Writer contracts


- `reserve(message, size)` provides managed payload space; prepared slabs supply complete records.
- `McapWriter` creates each file exclusively with `O_DIRECT` and uses 4-KiB alignment.
- Each write occupies contiguous file offsets; completion order is unspecified.
- Reservations reach the file in commit order. Prepared and external appends leave combining batches buffered, so they can land ahead of earlier commits.
- Registrations survive reuse and rotation. Explicit idle retirement releases them.
- Rejected prepared blocks return intact; drive `flush_written()` before retrying pressure.
- Slab recycling waits for all readers and I/O. Sealing allocates one `Arc`; recycling uses a mutex.
- MCAP submissions drive completions and flush aged batches on the owner thread.
- `poll()` optionally refreshes idle writers. Idle batches wait for another call.
- `flush_written()` waits for writes; `finish()` finalizes and syncs MCAP. Check both results.
- Rings use SINGLE_ISSUER, DEFER_TASKRUN and TASKRUN_FLAG (Linux 6.1+).
- Inject events and optional stage metrics through `McapConfig`; both default to disabled.
