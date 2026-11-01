# Reusable CPU collection and recording

One `system-monitor` publishes CPU telemetry to a separate `mcap-recorder`.
Console's [live bridge](../../../../console/bridge/README.md) relays the same
outputs to browsers when selected; declare them as `live_outputs`
(`cpu-monitor/snapshots`, `cpu-monitor/dictionary`, kind `TELEMETRY_PACKET`).

Build the existing system-monitor image with the `bpf,dora` features and the recorder image.

Compose inputs, in one experiment project:

- `apps/system-monitor/deploy/reusable/monitor.yaml`
- `apps/cpu-viewer/deploy/reusable/cpu-recording.yaml`

Use `cpu-flow.yml` as the Dora graph. The experiment assembler supplies the
approved `dora_app::environment` attachment settings for each service, shared IPC
and participant identity. Match graph IDs to service names when renaming nodes.

Supply explicit Compose overrides for images, identity, collector command
(capacities/backend) and recorder configuration. Mount PR34's
`apps/mcap-recorder/deploy/reusable/cpu.pbtxt` read-only at `/config/recorder.pbtxt`.
Bind a writable artifact directory to the recorder's `/results`; `cpu.mcap` must not exist.
Keep the BPF capabilities, host cgroup namespace and kernel mounts exclusive to the
collector. Select the existing task-bpf feature and a nonzero `--task-capacity` only when needed.

The collector waits for the recorder's attachment healthcheck before it starts.
The live bridge subscribes with `drop_oldest` and never delays collection.

## Recording and shutdown

The CPU recorder consumes original producer packets and creates `/results/cpu.mcap`
with the configured `doramcap/cpuacct` and `doramcap/cpuacct/dictionary` topics.
Workload recording uses another instance and destination. The recorder loads its
pbtxt configuration at startup.

For workflow startup, use attachment healthchecks before collection. With #34, omit the recording declaration's stream path to wait for MCAP file
creation. Configure a stream only when the collector also writes a growing file.

On stop, the workflow explicitly stops system nodes. The recorder drains buffered
Dora events before finalizing and the workflow verifies the recording afterward. Recorder queues are bounded even with `backpressure`; queue overload
can lose messages. A finalized footer alone does not establish complete capture.
BPF attachment and the combined Docker recording path require runtime validation
on a capable host.
