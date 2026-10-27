# Component ownership

`libs/<name>/` contains standalone reusable crates. Related crates share
`libs/<domain>/<crate>/`, such as `libs/fault/{reporter,aggregation,protocol,schema}`.
Domain directories own a short architecture/index README; each crate keeps its
Cargo package name and API documentation. `apps/<name>/` contains an
independently runnable application and its supporting components. Each owner
keeps its docs, tests, examples and benchmarks alongside its code. Add these
subdirectories as needed. The root README is the component index; [libs/README.md](../libs/README.md)
indexes library groups and standalone crates. Groups cover resources, telemetry,
memory, and faults. Each group has a README and each leaf crate has a
`Cargo.toml`; group directories need no manifest or placeholder Rust source.

Applications can contain nested crates: `system-monitor/dora` owns publication
and direct-buffer Protobuf encoding; `cpu-viewer/server` owns HTTP/WebSocket
serving. These nested crates support their owning app. Cargo workspace membership
is explicit for all crates, including nested library and application crates.
Add each new crate path to the root `Cargo.toml`.

`cpu-viewer/dataflows` owns demo compositions of the monitor, viewer and optional
recorder. Recording lifecycle tests belong to `mcap-recorder/tests`; native
collector integration tests belong to `system-monitor/tests`. CPU protocol bindings and descriptors belong to `telemetry-protocol`; workflow
events belong to `workflow-protocol`. The CPU viewer owns its delivery schema
and ingress annotations. Browser-generated bindings stay in the consuming frontend.

Root `docs/` covers repository-wide conventions and platform plans. Root
`scripts/` covers workspace tooling such as CI. Library API examples stay in
that library's `examples/` directory. Benchmark evidence lives with the component
whose workload was measured.

All documented shell commands run from the repository root unless stated
otherwise. DORA YAML executable paths are relative to the YAML file.
