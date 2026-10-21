# Telemetry protocol

Shared Protobuf schema and Rust codec for CPU snapshots and dictionary
checkpoints. The library owns the [schema](proto/telemetry.proto), generated MCAP
descriptor, [wire contract](docs/wire-protocol.md), and [Python validation reader](python/).
Cargo generates Rust bindings with vendored protoc. The browser consumer keeps
its generated TypeScript declarations and JavaScript codec in its frontend.

Run `cargo test -p telemetry-protocol`. See the wire contract for regeneration
commands. Library examples generate cross-language fixtures and inspect packets.
