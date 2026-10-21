# doramcap

A workspace of reusable telemetry libraries and independently runnable applications.
Each component owns its documentation, tests, examples, and benchmarks.

| Component | Purpose |
|---|---|
| [pbtxt-config](libs/pbtxt-config/) | Typed pbtxt composition, app registry, provenance, and deployment fingerprints |
| [resource-policy](libs/resource-policy/) | Composable Protobuf CPU/RT goals and caller enforcement |
| [cgroup-bpf](libs/cgroup-bpf/) | Native per-CPU cgroup accounting |
| [task-accounting](libs/task-accounting/) | Experimental kernel-thread runtime accounting |
| [activity-tracing](libs/activity-tracing/) | Bounded activity capture and embedded MCAP/live/OTel outputs |
| [fault-reporter](libs/fault-reporter/) | Bounded success/failure observations and recovery evidence |
| [fault-schema](libs/fault-schema/) | Generated fault Protobuf contracts, enums and compatibility baseline |
| [fault-protocol](libs/fault-protocol/) | Protobuf reporter IPC and aggregation storage/publication contracts |
| [fault-aggregation](libs/fault-aggregation/) | Fault lifecycle, recovery hysteresis and freshness aggregation |
| [monitor-core](libs/monitor-core/) | Bounded snapshot buffers and collection handoff |
| [telemetry-protocol](libs/telemetry-protocol/) | Protobuf schemas, codecs, and descriptors |
| [telemetry-mcap](libs/telemetry-mcap/) | Reusable MCAP recording sink |
| [storage-persistence](libs/storage-persistence/) | Bounded io_uring MCAP writer, aligned buffer pool, combining and rotation |
| [system-monitor](apps/system-monitor/) | Standalone collector with an optional DORA publisher |
| [provenance-demo](apps/provenance-demo/) | Four-node Dora pipeline, linked provenance/activity timelines, and measured latency |
| [activity-demo](apps/activity-demo/) | Thread pub/sub simulator and live TypeScript trace viewer |
| [cpu-viewer](apps/cpu-viewer/) | Demo web server and React dashboard |
| [storage-demo](apps/storage-demo/) | Live io_uring, write-combining and MCAP tail web inspector |
| [mcap-recorder](apps/mcap-recorder/) | Independent DORA-to-MCAP recording process |

## Build

Run from the repository root. Rust is pinned in `rust-toolchain.toml` and
Cargo dependencies are locked. Component READMEs document launch and validation.

```sh
cargo build --locked --workspace
cargo test --locked --workspace
# Native Linux eBPF support requires Clang, libelf/zlib and Linux headers:
cargo build --locked --workspace --all-features
```

## Formatting and linting

VS Code formats Rust on save and runs Clippy through rust-analyzer.
All workspace crates inherit the default Clippy rules from the root manifest.
CI checks formatting and treats compiler and Clippy warnings as errors for
both default and all-feature configurations.

```sh
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --no-deps --all-targets --locked -- -D warnings
cargo clippy --workspace --no-deps --all-targets --all-features --locked -- -D warnings
```

Start with the [live dashboard guide](apps/cpu-viewer/docs/running.md).
See [repository layout conventions](docs/repository-layout.md), the
[platform roadmap](docs/roadmap.md), and [CI operations](scripts/ci/README.md).
