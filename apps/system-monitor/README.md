# System monitor

A standalone CPU collector using native cgroup counters and optional
PF_KTHREAD runtime accounting. Collection runs on a dedicated thread at 10 Hz
with bounded reusable buffers. The application owns configuration, metadata
lookup, reporting, shutdown and publication scheduling.

```sh
cargo run -p system-monitor -- --check
cargo run -p system-monitor -- --backend synthetic --samples 10 --output /tmp/monitor.pb
cargo build -p system-monitor --features bpf,task-bpf,dora
```

Standalone output is length-prefixed Protobuf. `--dora` publishes snapshots and
dictionary checkpoints through the [DORA wrapper crate](dora/). The native
[launch wrapper](scripts/run-native-monitor.sh) follows the shared credential
procedure in the repository's AGENTS.md.

- [Collection behavior and qualification](docs/collection.md)
- [Native integration test](tests/validate_native.py)
- [Measured encoding sizes](benchmarks/encoding/)
- [Original implementation history](docs/history/101826-phase0-cgroup-monitor.md)
- [Live dashboard launch](../cpu-viewer/docs/running.md)

Install `libs/telemetry-protocol/python/requirements.txt` into a virtual environment
before running Python validation. Native validation needs BPF privileges:

```sh
sudo /path/to/venv/bin/python apps/system-monitor/tests/validate_native.py --monitor "$PWD/target/debug/system-monitor"
```
