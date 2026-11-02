# System monitor

A standalone CPU collector using native cgroup counters and optional
PF_KTHREAD runtime accounting. Collection runs on a dedicated thread at 10 Hz
with bounded reusable buffers. The application owns configuration, metadata
lookup, reporting, shutdown and publication scheduling.

```sh
cargo run -p system-monitor -- --check
cargo run -p system-monitor --features bpf -- --samples 10 --output /tmp/monitor.pb
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

Install `libs/telemetry/protocol/python/requirements.txt` into a virtual environment
before running Python validation. Native validation needs BPF privileges:

```sh
sudo /path/to/venv/bin/python apps/system-monitor/tests/validate_native.py --monitor "$PWD/target/debug/system-monitor"
```

## Embedded MCAP recording

The [native BPF Compose template](deploy/recording.yaml) lets the shared Docker workflow retain CPU recordings without a separate recorder process. The workflow supplies
the image and output directory, then snapshots the selected
template into each run’s private artifacts.

```sh
system-monitor --backend bpf --dora --record capture.mcap --record-rotate-bytes 67108864
```

Recording topics come from [recording.pbtxt](config/recording.pbtxt), parsed at
startup. Pass `--record-config FILE.pbtxt` to load a custom topic configuration.

Recording uses the publication owner thread and an eight-buffer, 512-KiB pool.
Direct I/O uses 4-KiB alignment; `--record-buffered` selects buffered file I/O.
The maximum encoded message is bounded by the pool. Files are created exclusively.
Rotation is a soft limit at message boundaries and creates `capture.000001.mcap`,
then subsequent numbered files, each with schema/channel and dictionary metadata.

Small messages flush within the owner's 100-ms combining deadline. Storage
pressure stalls the owner; the dedicated collector counts dropped batches when
its handoff fills. Recording/publication errors terminate the session visibly.
Callers of the library use `finish` for a finalized recording.
