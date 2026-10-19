# doramcap

A workspace of reusable telemetry libraries and independently runnable applications.
Each component owns its documentation, tests, examples, and benchmarks.

| Component | Purpose |
|---|---|
| [cgroup-bpf](libs/cgroup-bpf/) | Native per-CPU cgroup accounting |
| [task-accounting](libs/task-accounting/) | Experimental kernel-thread runtime accounting |
| [monitor-core](libs/monitor-core/) | Bounded snapshot buffers and collection handoff |
| [telemetry-protocol](libs/telemetry-protocol/) | Protobuf schemas, codecs, and descriptors |
| [telemetry-mcap](libs/telemetry-mcap/) | Reusable MCAP recording sink |
| [system-monitor](apps/system-monitor/) | Standalone collector with an optional DORA publisher |
| [cpu-viewer](apps/cpu-viewer/) | Demo web server and React dashboard |
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

Start with the [live dashboard guide](apps/cpu-viewer/docs/running.md).
See [repository layout conventions](docs/repository-layout.md), the
[platform roadmap](docs/roadmap.md), and [CI operations](scripts/ci/README.md).
