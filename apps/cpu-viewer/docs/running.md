# Run CPU Viewer

![Live CPU dataflow](architecture.svg)

Requires Linux 6.8+, cgroup v2, kernel BTF, Node.js 22.12+ and Dora CLI 1.1.0.
See [BPF prerequisites](../../../libs/resources/cgroup-bpf/README.md).
Build unprivileged; the native collector wrapper uses the checkout's sudo setup.

```sh
npm ci --prefix apps/cpu-viewer/frontend
npm run build --prefix apps/cpu-viewer/frontend
cargo build -p system-monitor -p cpu-viewer --features system-monitor/dora,system-monitor/bpf,system-monitor/task-bpf
dora run apps/cpu-viewer/dataflows/live-cpu-dataflow.yml
```

Open <http://127.0.0.1:8080>; stop with Ctrl+C. The viewer shows real BPF activity
per CPU, a 30-second history and collector/stream metrics. Cgroup counters include
descendants; kernel-thread accounting can overlap them. See
[accounting details](../../../libs/resources/task-accounting/CPU-ACCOUNTING.md).

The viewer accepts `--bind`, `--port` and `--assets`. Adjust the dataflow arguments
when changing them. Frontend development uses `npm run dev --prefix
apps/cpu-viewer/frontend`, with Vite proxying the backend on port 8080.
For container hosting, see [deployment](../deploy/README.md).

## Recording

The live flow has no recorder. The sample recording flow writes ten BPF samples
to `phase0.mcap`, refuses to overwrite it and finalizes when input closes:

```sh
cargo build -p system-monitor -p mcap-recorder --features system-monitor/dora,system-monitor/bpf
dora run apps/cpu-viewer/dataflows/phase0-dataflow.yml
```

## Verification

```sh
npm test --prefix apps/cpu-viewer/frontend
cargo test -p telemetry-protocol -p telemetry-mcap -p cpu-viewer
# With the live dataflow running:
npm run test:live --prefix apps/cpu-viewer/frontend
```

[Wire format and MCAP encoding](../../../libs/telemetry/protocol/docs/wire-protocol.md) ·
[Browser delivery schema](../server/proto/delivery.proto)
