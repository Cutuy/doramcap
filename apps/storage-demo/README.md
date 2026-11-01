# Storage Observatory

Live MCAP writer dashboard: pipeline stages, active writes, registration reuse,
correlated events, write timing and on-disk record inspection.

![Live feed](docs/102026-live-storage-timeline.svg)

```sh
npm ci --prefix apps/storage-demo/frontend
npm run build --prefix apps/storage-demo/frontend
cargo build -p storage-demo
cargo run -p storage-demo
# http://127.0.0.1:8091
```

Writes use real O_DIRECT I/O (`--buffered` selects buffered I/O). Controls submit
managed buffers and sealed slabs, retain slab readers, and flush, seal or rotate
the recording. The default bind is loopback.

```sh
cargo test -p storage-demo -p storage-persistence
npm test --prefix apps/storage-demo/frontend
```

[Controls, timeline and validation](docs/usage.md) ·
[Live-feed architecture](docs/102026-live-storage-timeline.md) ·
[Demo plan](docs/101826-live-storage-demo.md) ·
[Docker deployment](deploy/README.md)
