# Fault watchdog

![Architecture](docs/production-watchdog.svg)

Production Dora host for fault aggregation, source liveness and bounded publication.
Producer definitions own recovery/deadline policy; Protobuf config owns operational limits.

```sh
cargo build -p fault-watchdog                 # aggregation + live output
cargo build -p fault-watchdog --features mcap,otel
```

Run as a Dora node with `fault-watchdog CONFIG.pbtxt`. Outputs are independently
queued; optional adapters run only when enabled in configuration.

- [Config example](config/default.pbtxt) · [Config schema](proto/watchdog.proto)
- [Design, milestones and validation](docs/102026-production-watchdog.md)
- [Runtime contract](docs/102026-runtime.md) · [Live demo](../fault-demo/README.md)

Rust API documentation covers host clocks, queue loss, retries and finalization.
