# Component ownership

`libs/<name>/` contains reusable functionality. `apps/<name>/` contains an
independently runnable application and its supporting components. Each owner
keeps its docs, tests, examples and benchmarks alongside its code. Add these
subdirectories as needed. The root README is the component index.

Applications can contain nested crates: `system-monitor/dora` owns publication
and direct-buffer Protobuf encoding; `cpu-viewer/server` owns HTTP/WebSocket
serving. These nested crates support their owning app. Cargo workspace membership
is explicit for application crates and includes `libs/*`.

`cpu-viewer/dataflows` owns demo compositions of the monitor, viewer and optional
recorder. Recording lifecycle tests belong to `mcap-recorder/tests`; native
collector integration tests belong to `system-monitor/tests`. Protocol bindings
and descriptors belong to `telemetry-protocol`, with browser-generated bindings
kept in the consuming frontend.

Root `docs/` covers repository-wide conventions and platform plans. Root
`scripts/` covers workspace tooling such as CI. Library API examples stay in
that library's `examples/` directory. Benchmark evidence lives with the component
whose workload was measured.

All documented shell commands run from the repository root unless stated
otherwise. DORA YAML executable paths are relative to the YAML file.
