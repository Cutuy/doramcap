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

[Writer contracts](docs/contracts.md): alignment, ordering, registration lifetime,
slab recycling and completion.

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
