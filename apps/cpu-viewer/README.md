# CPU viewer

A demo consumer of system-monitor telemetry. The [Rust server](server/) subscribes
to DORA and serves HTTP plus a binary WebSocket stream. The [React frontend](frontend/)
plots per-CPU cgroup activity and virtual kernel runtime over a 30-second window.

- [Build, launch and dashboard behavior](docs/running.md)
- [Architecture](docs/architecture.svg)
- [DORA demo configurations](dataflows/)
- [Streaming and latency integration checks](tests/)

The server runs independently of the collector and recorder. Unit tests live
alongside the frontend model and protocol adapter. Run from the repository root:

```sh
npm ci --prefix apps/cpu-viewer/frontend
npm test --prefix apps/cpu-viewer/frontend
npm run build --prefix apps/cpu-viewer/frontend
# With the dataflow running:
npm run test:live --prefix apps/cpu-viewer/frontend
apps/cpu-viewer/frontend/node_modules/.bin/tsx apps/cpu-viewer/tests/check-latency.mts
```

[Docker deployment and gallery registration](deploy/README.md).
