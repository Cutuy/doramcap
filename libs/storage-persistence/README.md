# Storage persistence

Managed buffers, sealed slabs and leased mmap ranges feed one io_uring per writer. MCAP output is uncompressed and index-free.

![Writer lifecycle](docs/architecture/three-path-writer-lifecycle.svg)

## Prepared slab example

```rust
use storage_persistence::slabs::SlabPool;

let pool = SlabPool::new(8, 64 * 1024, 4096)?;
writer.prepare_slabs(&pool)?;
let mut records = writer.prepare_records(pool.acquire()?);
encode_into(records.reserve(message, encoded_len)?)?;
writer.append_prepared_records(records.seal()?)?;
writer.finish()?;
```

`writer`, `message` and `encode_into` are application-supplied.

## External SHM

```rust
writer.append_external_payload(message, published_payload)?;
writer.append_external_records(published_mcap_block)?;
```

Both inputs are caller-owned `ReadLease`s. Payloads receive a separate 4-KiB
managed prefix page; framed message blocks are validated and written unchanged.
No writable mmap builder is required or exposed. The prefix and payload advance
as two registered I/O operations within one logical append (further splitting
follows writer limits); independent appends retain independent progress.

[Architecture, contracts and validation](docs/101826-external-buffer-leases.md).

## Contracts

- `reserve(message, size)` provides managed payload space; prepared slabs supply complete records.
- Direct I/O requires `O_DIRECT` and configured 512-B or 4-KiB alignment.
- Each write occupies contiguous file offsets; cross-write placement and completion order are unspecified. Standalone writes leave combining batches buffered.
- Registrations survive reuse and rotation. Explicit idle retirement releases them.
- Rejected prepared blocks return intact; drive `flush_written()` before retrying pressure.
- Slab recycling waits for all readers and I/O. Sealing allocates one `Arc`; recycling uses a mutex.
- MCAP submissions drive completions and flush aged batches on the owner thread.
- `poll()` optionally refreshes idle writers. Idle batches wait for another call.
- `flush_written()` waits for writes; `finish()` finalizes and syncs MCAP. Check both results.
- Rings use SINGLE_ISSUER, DEFER_TASKRUN and TASKRUN_FLAG (Linux 6.1+).
- Inject events and optional stage metrics through `McapConfig`; both default to disabled.

## Observation

```rust
let (metrics, monitor) = McapMonitor::channel();
let writer = McapWriter::new(file, schema, &channels, McapConfig {
    observer: collector.observer(), metrics, ..Default::default()
})?;
// Use monitor.snapshot() and monitor.active_writes() from the owner thread.
writer.finish()?;
let final_state = monitor.snapshot();
```

[`McapConfig` API](src/mcap.rs) · [Injection architecture](docs/architecture/mcap-observation-injection.svg).
Enable `activity-tracing` for source-time combining/write/I/O spans through the
application's activity runtime. [Observation milestones](docs/101926-observability.md).

## Details and validation

| Topic | Reference |
|---|---|
| Input APIs and ownership | [Memory ingress](docs/101926-memory-ingress-cases.md) |
| Managed buffers and benchmarks | [Managed-pool milestones](docs/101926-growable-buffer-pool.md) |
| Slabs, limitations and evidence | [External-memory milestones](docs/101826-external-buffer-leases.md) |
| Live inspection and roadmap | [Observability milestones](docs/101926-observability.md) |

Run `cargo test -p storage-persistence --all-features` on Linux with io_uring support.
