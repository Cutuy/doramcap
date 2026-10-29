# Storage Observatory

Live MCAP writer dashboard: pipeline stages, active writes, registration reuse,
correlated events, CPU timing and on-disk record inspection.

```sh
npm ci --prefix apps/storage-demo/frontend
npm run build --prefix apps/storage-demo/frontend
cargo build -p storage-demo -p system-monitor
cargo run -p storage-demo
# http://127.0.0.1:8091
```

- Real O_DIRECT writes; synthetic CPU input by default. `--buffered` selects buffered I/O.
- **Collection** selects Off, Summary, Inspect or timed Capture; optional region filter.
- **Timeline filter** matches a write or region ID. Loss/eviction counts remain visible.
- **Stop recording** seals and syncs; **Start new session** clears previous capture files.
- **Large managed buffer** selects 1/4/16 MiB; each click submits one message using writer defaults.
- **Sealed slab** writes two 256-KiB records from a four-slab pool.
- Slab writes recycle by default. **Retain slab reader** holds bytes after I/O;
  four retained readers fill the pool. **Release readers** enables recycling.
- The 1/4/16 MiB selector controls managed payloads. Slabs always hold two 256-KiB payloads.
- **Retire slab registrations** releases idle registrations; the next use registers again.
- **Probe burst**, **Flush now** and **Seal & rotate** exercise the pipeline.
- `--start-paused`, `--seconds N`, `--port N`, `--bind IP`, `--dir NEW_DIR` configure the session. The default bind is loopback.
- `--backend bpf --monitor PATH` uses a capability-enabled native monitor.

Snapshots publish at 25 Hz through a latest-value Protobuf WebSocket. Encoding
runs outside the writer thread; slow viewers skip intermediate frames. Collection retains
4,096 events; the dashboard retains 256. IDs travel as uint64 and decode to exact strings. Queue-to-CQE timing
includes time until the next submission. The owner submits heartbeats roughly every
5 ms; record submissions drive completions without explicit writer polling.
**Flush now** waits for accepted writes; **Seal & rotate** also finalizes and syncs. Workloads cover managed buffers and sealed slabs. Hardware DMA is not measured.

## Live activity timeline

React/SVG groups instances into combining, combined, large and slab lanes, with
child I/O beneath each write. Mapped and other paths appear when present. Select a
bar for IDs, file offsets and completion results; scroll to zoom, pause or filter.
History, capture loss and transport gaps remain visible. Activity is enabled by
default; `--no-default-features` disables capture. `--assets PATH` selects a built UI.

Add `--activity /tmp/storage-activity.mcap` to save the same source-time activity.
Ctrl-C drains capture and reports losses. Use a new path outside the session directory.

[Live-feed architecture and validation](docs/102026-live-storage-timeline.md)

## Optional tracing export

```sh
STORAGE_TRACE_STDERR=1 cargo run -p storage-demo --features tracing-export
```

A single independent worker exports metadata to stderr through a scoped subscriber.
A blocked sink cannot block the writer; history gaps are reported. No events feed
back into the recorded MCAP. Runtime Off stops new event collection.

## Validation

```sh
cargo test -p storage-demo -p storage-persistence
cargo build -p storage-demo --bins --examples --features tracing-export,activity-tracing
cargo build -p system-monitor
python3 apps/storage-demo/validate.py --bin-dir target/debug
python3 apps/storage-demo/tests/tracing_export.py --bin-dir target/debug
python3 apps/storage-demo/tests/activity_capture.py --bin-dir target/debug
npm test --prefix apps/storage-demo/frontend
(cd apps/storage-demo/frontend && npm run test:live)
```

[Observability milestones](../../libs/storage-persistence/docs/101926-observability.md)
· [Demo plan](docs/101826-live-storage-demo.md)

The page uses `/api/snapshots/stream`; `/api/state` and legacy `/events` provide
on-demand JSON diagnostics. Control requests remain JSON.

[Docker deployment and gallery registration](deploy/README.md).
