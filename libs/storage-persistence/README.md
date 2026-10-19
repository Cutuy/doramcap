# Storage persistence

Single-owner storage primitives with bounded aligned buffers, transactional file
ranges, and one io_uring per writer. Accepted leases remain owned through write
completion and retries. Call `close` to observe final write and sync errors.

The MCAP layer provides direct payload reservations and uncompressed,
index-free files. The caller supplies schema/channel metadata and serializes
into the returned slice. Protobuf and Dora remain application responsibilities.

| Module | Responsibility |
|---|---|
| `file_offsets` | Checked, non-overlapping append ranges; abandoned reservations |
| `managed_buffers` | Fixed-capacity aligned allocations and exclusive leases |
| `writer` | Buffered/registered direct writes, retries, cancellation and retirement |
| `mcap` | File framing, schemas/channels and payload reservations |

Registered direct I/O supports configured 512-B and 4-KiB alignment. Small
messages share combining buffers; large messages encode through a segmented
`BufMut` reservation. Rotation retains the ring/pool and repeats metadata.
External leases follow a separate [gated plan](docs/101826-external-buffer-leases.md).

Call `finish` to flush combining buffers and seal MCAP. Dropping an unfinished
MCAP writer can leave an incomplete file. The owner must poll the exposed flush
deadline while input is idle. See [validation evidence](docs/101826-mcap-writer-design.md).

Validation: `cargo test -p storage-persistence` includes real Linux io_uring tests.
The tests fail explicitly when the host cannot create a ring.

## Optional diagnostics

Enable the `diagnostics` Cargo feature for `McapWriter::diagnostics`,
`write_events`, `finish_with_diagnostics`, and the underlying writer event/counter
accessors. It adds a fixed 256-event history and per-write clock reads. The demo
opts in explicitly. Default builds omit these APIs, event storage and timing work.
Web transport, Protobuf inputs, percentile windows and demo controls live in
`apps/storage-demo`. Cargo features unify across dependents in a build; build the
production application independently to keep diagnostics disabled.
