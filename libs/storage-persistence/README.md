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

## Mapped batch example

```rust
let region = MmapRegion::new(mapping, observer);
writer.prepare_region(&region)?;
let mut records = writer.prepare_mapped_records(region.acquire(block_range)?);
encode_into(records.reserve(message, encoded_len)?)?;
writer.append_mapped_records(records.seal()?)?;
```

`mapping` is an exclusively owned `memmap2::MmapMut`. External publishers use
unsafe `MmapRegion::import` / `lease` with mapping and reuse guards.

## Contracts

- `reserve(message, size)` provides managed payload space; prepared slabs supply complete records.
- Direct I/O requires `O_DIRECT` and configured 512-B or 4-KiB alignment.
- Each write occupies contiguous file offsets; cross-write placement and completion order are unspecified. Standalone writes leave combining batches buffered.
- Registrations survive reuse and rotation. Explicit idle retirement releases them.
- Rejected prepared blocks return intact; drive `flush_written()` before retrying pressure.
- Slab recycling waits for all readers and I/O. Sealing allocates one `Arc`; recycling uses a mutex.
- Poll the combining deadline while idle. Call `finish()` to finalize MCAP and observe errors.
- Inject an `Observer` for bounded events; snapshots are always available. Legacy `diagnostics` remains supported.

## Activity timeline

Enable `activity-tracing` and install the existing `activity_tracing::Runtime`
subscriber on each registered writer thread. Spans carry writer/file/write IDs,
source path, region, byte range and direct-I/O mode.

| Span | Lifetime |
|---|---|
| `storage.combining` | First committed message → batch admission |
| `storage.write` | Admission → terminal retirement |
| `storage.io` | SQE queued → CQE observed; child of its write |

Combining links to its write; completion results, retries and terminal outcomes
are events. Timing includes queueing and polling delay. The feature defaults off;
capture is independent of the dashboard collector's mode.

## Details and validation

| Topic | Reference |
|---|---|
| Input APIs and ownership | [Memory ingress](docs/101926-memory-ingress-cases.md) |
| Managed buffers and benchmarks | [Managed-pool milestones](docs/101926-growable-buffer-pool.md) |
| Slabs, limitations and evidence | [External-memory milestones](docs/101826-external-buffer-leases.md) |
| Live inspection and roadmap | [Observability milestones](docs/101926-observability.md) |

Run `cargo test -p storage-persistence --all-features` on Linux with io_uring support.
