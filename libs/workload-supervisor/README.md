# Supervisor workload lifecycle

Rust steps run one System Monitor under an owned foreground Supervisor daemon.

![Lifecycle](../workload-lifecycle/docs/102926-supervisor-monitor-mvp.svg)

See the [runnable deployment](../../apps/system-monitor/deploy/supervisor/README.md),
[protobuf inputs](proto/supervisor.proto) and Rust API documentation.
Native fault and BPF recording tests live in [tests/native.rs](tests/native.rs).

Build the test executables unprivileged:

```sh
rustc --edition=2024 --crate-name supervisor_fixture libs/workload-supervisor/tests/fixture.inc -o target/ci/supervisor-test-fixture
cargo test --profile ci --locked -p workload-supervisor --test native --no-run
```

Run `tests/validate_native.py --monitor /absolute/path/to/target/ci/system-monitor`
with the test capabilities and a 256 MiB locked-memory limit. The existing native
monitor aggregate and ARM64 worker invoke this suite when Supervisor is provisioned;
missing runtime prerequisites print an explicit skip.
