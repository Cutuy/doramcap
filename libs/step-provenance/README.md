# Step provenance

Bounded output ancestry and event-driven latency pairs, with versioned protobuf
records, definitions, and results.

- `Engine::new`: validate and index fixed measurement definitions.
- `Engine::ingest` / `decode_update`: merge one patch and return terminal results.
- `Engine::record` / `lineage`: inspect direct inputs or source roots and gaps.
- `Engine::stats`: inspect retained state, pending instances, and evaluation count.

The caller serializes updates and consumes results. The core has no background
worker; Dora integration is the next milestone.

[Contracts and selectors](docs/contracts.md) ·
[Implementation plan](docs/102026-step-provenance.md) ·
[Protobuf schema](proto/provenance.proto) ·
[Demo proposal](../../apps/provenance-demo/docs/102026-provenance-demo.md)

```sh
cargo test --locked -p step-provenance --all-features
cargo clippy --locked -p step-provenance --all-targets --all-features -- -D warnings
```
