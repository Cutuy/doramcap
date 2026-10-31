# Reusable CPU collection, live feed and recording

One `system-monitor` publishes CPU telemetry to the existing CPU feed and a
separate `mcap-recorder`. The gallery frontend remains optional.

Build `cpu-viewer` with `cargo build --release --locked -p cpu-viewer`, then copy
the binary beside this Dockerfile and build `doramcap/cpu-feed:local`. Build the
existing system-monitor image with the `bpf,dora` features and the recorder image.

Compose inputs, in one experiment project:

- `apps/system-monitor/deploy/reusable/monitor.yaml`
- `apps/cpu-viewer/deploy/reusable/feed.yaml`
- `apps/cpu-viewer/deploy/reusable/cpu-recording.yaml`

Use `cpu-flow.yml` as the Dora graph. The experiment assembler supplies the
approved `dora_app::environment` attachment settings for each service, shared IPC
and participant identity. Match graph IDs to service names when renaming nodes.
The shared runtime and generated attachment settings require the incoming Dora
workflow; these fragments do not start another daemon/coordinator.

Supply explicit Compose overrides for images, identity, collector command
(capacities/backend), feed bind/port and recorder destination. Bind a writable
artifact directory to the recorder's `/results`; `cpu.mcap` must not exist.
No artifact or kernel mounts are needed by the feed. Keep the BPF capabilities,
host cgroup namespace and kernel mounts exclusive to the collector. Select the
existing task-bpf feature and a nonzero `--task-capacity` only when needed.

## Feed contract

| Endpoint | Behavior |
| --- | --- |
| `GET /stream` | WebSocket binary protobuf packets: `server/proto/delivery.proto` snapshots and telemetry dictionaries |
| `GET /clock` | Server Unix nanoseconds as decimal text; `Cache-Control: no-store` |
| `GET /health/ready` | 503 until the first validated snapshot and dictionary, then 200 |

`--headless` serves these routes without frontend files. Without it, the existing
`--assets` gallery mode remains available. The feed binds localhost by default;
choose another address explicitly when deploying behind a browser-facing proxy.
Readiness establishes received data, not current producer liveness. Source closure
or Dora stop ends the server; invalid packets terminate it with an error.

Each connection receives the latest dictionary before the latest snapshot. A
single shared latest snapshot/dictionary is retained; slow or disconnected
browsers skip intermediate snapshots and cannot block collection. Producer
sequence, dropped-batch, dictionary revision/overflow and task-loss fields remain
available; viewer ingress annotations describe the feed's received traffic.
Dora input queues use `drop_oldest`; periodic full dictionaries permit recovery.
Clients should use matching dictionary revisions before resolving numeric IDs.

## Recording and shutdown

The CPU recorder consumes original producer packets and creates `/results/cpu.mcap`
with separate snapshot/dictionary topics. Workload recording uses another instance
and destination. This example uses the existing CPU recorder invocation; the new
configurable recorder can replace its command after that contract lands.

Start consumers before collection. On stop, the workflow stops the producer,
allows consumer input closure/drain, then finalizes and publishes recording
artifacts. Recorder queues are bounded even with `backpressure`; queue overload
can lose messages. A finalized footer alone does not establish complete capture.
BPF attachment and the combined Docker recording path require runtime validation
on a capable host.
