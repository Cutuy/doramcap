# Fault reporter

Bounded success/failure capture with urgent queues, recovery evidence and optional success limiting. The host event loop drains reports; the library creates no threads or I/O.

![Architecture](../fault-protocol/docs/fault-event-architecture.svg)

```rust
let (reporter, mut publisher) = Reporter::new(source_id, Config::default())?;
let check = reporter.register(Definition::new("recorder", "disk", "write_failed")?)?;
check.failure(Text::new("disk unavailable")?)?;
```

[API and scheduling contracts](src/lib.rs) · [Policy and records](src/model.rs) · [Design and evidence](docs/102026-fault-reporter.md) · [Wire contracts](../fault-protocol/README.md)

Build the API reference: `cargo doc --no-deps -p fault-reporter --open`.
