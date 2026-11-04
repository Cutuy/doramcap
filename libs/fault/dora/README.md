# Fault Dora

Drains a fault reporter on an application's existing Dora publication loop: bounded batches, periodic repair snapshots and a final state on shutdown.

```rust
let mut health = Health::new()?;
let check = health.register("recorder", "write", Some(Duration::from_secs(2)))?;
check.success()?;
health.tick(&mut node)?;
```

[API](src/lib.rs) · [Example participant](examples/health_fixture.rs) · [Reporter](../reporter/README.md)
