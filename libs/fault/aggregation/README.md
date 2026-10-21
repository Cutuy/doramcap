# Fault aggregation

Aggregates sequenced reports into fault health, freshness and revisioned domain outputs. The host injects diagnostics and owns event-loop scheduling and transport.

![Architecture](docs/aggregation-observability.svg)

```rust
let diagnostics = Arc::new(FaultObservability::new());
let mut agg = Aggregator::new(epoch, Config::default(), diagnostics.clone())?;
agg.reconcile(&producer_snapshot, now)?;
let counters = diagnostics.stats();
```

[API and clock contracts](src/lib.rs) · [Observability](src/observability.rs) · [Design and evidence](docs/102026-fault-aggregation.md) · [State transitions](../protocol/docs/102026-event-driven-faults.md) · [Wire contracts](../protocol/README.md)

Build the API reference: `cargo doc --no-deps -p fault-aggregation --open`.
