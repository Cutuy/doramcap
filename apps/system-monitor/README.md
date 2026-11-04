# System monitor

A standalone CPU collector using native cgroup counters and optional
PF_KTHREAD runtime accounting. Collection runs on a dedicated thread at 10 Hz
with bounded reusable buffers. The application owns configuration, metadata
lookup, reporting, shutdown and publication scheduling.

```sh
cargo run -p system-monitor -- --check
cargo run -p system-monitor --features bpf -- --config apps/system-monitor/deploy/monitor.pbtxt
cargo build -p system-monitor --features bpf,task-bpf,dora
```

Each launch reads one [pbtxt configuration](proto/monitor.proto); `--check` and
`--check-bpf` probe host support. `output_path` writes length-prefixed Protobuf. A
`dora` section publishes snapshots and dictionary checkpoints through the
[DORA wrapper crate](dora/); a `recording` section writes MCAP. The native
[launch wrapper](scripts/run-native-monitor.sh) follows the shared credential
procedure in the repository's AGENTS.md.

- [Collection behavior and qualification](docs/collection.md)
- [Native integration test](tests/validate_native.ts)
- [Measured encoding sizes](benchmarks/encoding/)
- [Original implementation history](docs/history/101826-phase0-cgroup-monitor.md)
- [Live dashboard launch](../cpu-viewer/docs/running.md)

Native validation needs BPF privileges:

```sh
sudo node apps/system-monitor/tests/validate_native.ts --monitor "$PWD/target/debug/system-monitor"
```

## Embedded MCAP recording

The [native BPF Compose template](deploy/recording.yaml) lets the shared Docker workflow retain CPU recordings without a separate recorder process. The workflow supplies
the image and output directory, then snapshots the selected
template into each run’s private artifacts.

```pbtxt
app {
  id: "monitor"
  config { [type.googleapis.com/doramcap.monitor.v1.MonitorConfig] {
    records: 4096
    queue_slots: 8
    dora {}
    recording {
      path: "capture.mcap"
      rotation_bytes: 67108864
      samples_topic: "doramcap/cpuacct"
      dictionary_topic: "doramcap/cpuacct/dictionary"
    }
  } }
}
```

Configs compose through [`pbtxt-config`](../../libs/pbtxt-config/README.md) includes.
`system-monitor --resolve-config FILE` prints one complete pbtxt.

Recording uses the publication owner thread and an eight-buffer, 512-KiB pool.
Recordings use direct I/O with 4-KiB alignment.
The maximum encoded message is bounded by the pool. Files are created exclusively.
Rotation is a soft limit at message boundaries and creates `capture.000001.mcap`,
then subsequent numbered files, each with schema/channel and dictionary metadata.

Small messages flush within the owner's 100-ms combining deadline. Storage
pressure stalls the owner; the dedicated collector counts dropped batches when
its handoff fills. Recording/publication errors terminate the session visibly.
Callers of the library use `finish` for a finalized recording.
