# Reusable CPU collection and recording

One BPF `system-monitor` publishes CPU telemetry to an `mcap-recorder`.
Console's [live bridge](../../../../console/bridge/README.md) relays it to browsers.

| Input | Purpose |
| --- | --- |
| `apps/system-monitor/deploy/reusable/monitor.yaml` | Collector; sole holder of BPF capabilities and kernel mounts |
| `cpu-recording.yaml` | Recorder; writes `/results/cpu.mcap` |
| `cpu-flow.yml` | Dora graph; node IDs match service names |

- **Overrides:** images, identity, collector capacities and a writable `/results`.
- **Recorder config:** mount #34's `cpu.pbtxt` read-only at `/config/recorder.pbtxt`.
- **Live view:** declare `cpu-monitor/snapshots` and `cpu-monitor/dictionary` as `CPU_ACCOUNTING` `live_outputs`.
- **Startup:** the collector waits for the recorder's healthcheck.
- **Stop:** the recorder drains queued input, then finalizes; overloaded queues can still lose messages.
