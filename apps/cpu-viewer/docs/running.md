# Dora CPU monitor demos

![Live CPU dataflow and wire formats](architecture.svg)

See [exact binary field definitions](../../../libs/telemetry/protocol/docs/wire-protocol.md). The dashed MCAP
branch is optional: the live viewer YAML does not launch the recorder.

Use Dora CLI 1.1.0, matching the pinned `dora-node-api`. Build as your regular
user. The native wrapper elevates only the collector, following main's
`AGENTS.md`: it resolves the first Git worktree, verifies the shared
`.sudo-password` is ignored and mode 0600, and passes it only through sudo stdin.
Dora and the localhost web server stay unprivileged.

## Native live viewer

From the checkout root (Node.js 22.12+):

```sh
npm ci --prefix apps/cpu-viewer/frontend
npm run build --prefix apps/cpu-viewer/frontend
cargo build -p system-monitor -p cpu-viewer --features system-monitor/dora,system-monitor/bpf,system-monitor/task-bpf
dora run apps/cpu-viewer/dataflows/live-cpu-dataflow.yml
```

Open <http://127.0.0.1:8080>. Stop with Ctrl+C. The dataflow uses real eBPF
counters and enables experimental PF_KTHREAD-only accounting with capacity 32768.
`live-cpu-native-dataflow.yml` is an explicit alias. Native requirements are
Linux >=6.8, cgroup v2, kernel BTF, build dependencies and privileged BPF access;
see `libs/resources/cgroup-bpf/README.md` for kernel qualification details.

```text
system-monitor → Dora snapshots (10 Hz) + dictionary (~0.5 Hz) → cpu-viewer
                                                   ↓ binary WebSocket
                                                React + uPlot
```

One chart per CPU automatically shows its eight busiest active cgroups, ranked
by time-weighted activity over five seconds and refreshed once per second.
Charts refresh at 10 Hz and retain 30 seconds, expiring even during silence.
The compact overview shows kernel threads and the two busiest current cgroups;
each card expands to the full path tree for the selected cgroups. Stream and collector metrics are shown above the plots.
100% means one occupied core. Cgroup counters include descendants, so lines are
not stacked or summed. Root cgroup runtime, first samples and resets show as
gaps. History is bounded by 600 points, 64 CPUs and 8192 cgroup records per
snapshot. Colors remain stable across rankings. Labels show cgroup paths.

The persistent dashed virtual/kernel line uses backend-aggregated PF_KTHREAD
runtime by charged CPU. The BPF probe excludes all non-PF_KTHREAD tasks before
map insertion. Individual task records are not published. This reporting
aggregate may overlap cgroup runtime. Its scope is PF_KTHREAD runtime;
application syscalls and separate IRQ/idle accounting are outside it. Task-map
capacity and losses are visible; records persist until restart and the bounded
map can fill. Remove `--task-capacity 32768` to disable this experimental
diagnostic. See [accounting
nuances](../../../libs/resources/task-accounting/CPU-ACCOUNTING.md).

The server keeps one latest frame shared across browser clients; slow clients
skip intermediate frames to keep buffering bounded. Browser reconnects are
automatic. Names arrive as separate full dictionary checkpoints about every 2
seconds, with session/revision matching; reconnects receive the cached
checkpoint first. Snapshot payloads are binary on both hops; see
`libs/telemetry/protocol/docs/wire-protocol.md` for the versioned Protobuf schema, 64-bit precision, MCAP
encoding, and standalone output framing.

The viewer owns the [browser delivery schema](../server/proto/delivery.proto).
Its field layout preserves existing browser packets; the shared CPU schema reserves
the former annotation field. Regenerate its unchanged cross-language fixture with
`cargo run -p cpu-viewer --example fixture -- apps/cpu-viewer/frontend/src/fixtures/snapshot-v2.pb`.

Ingress metrics count original Dora snapshot plus dictionary bytes; the message
count/rate counts snapshots only, independent of browser count, before
dictionary expansion or WebSocket framing. Rates use completed intervals of at
least one second; cumulative counters begin at viewer startup. These rates
measure application payload bytes. Capture-to-receive latency uses the shared
host wall clock from producer capture to subscriber dequeue, including
collection, publication and queueing. Browser rendering follows receipt.
Latest/mean latency are shown; invalid timestamps are excluded. Stale rates and
latency are hidden after two seconds. Throughput uses monotonic time. Browser
latency uses `/clock` calibration (fastest of three probes every 30 s) and
browser monotonic time, so it does not require matching wall clocks. It is an
estimate: half the probe round trip is shown as timing uncertainty, and network
asymmetry/drift can affect it. It ends at WebSocket callback entry, before
decode/render. Calibration expires after 90 s.

The server defaults to IPv4 loopback; use `--bind` to select another IP address.
Change `--port 8080` in the YAML if needed. `/health/ready` returns success after
the first snapshot and dictionary have arrived.
Assets default to this checkout's `apps/cpu-viewer/frontend/dist`; use `--assets /absolute/path`
when relocating the binary. For frontend development run `npm run dev --prefix
apps/cpu-viewer/frontend`; Vite proxies WebSocket `/stream` to 8080.

## Synthetic transport test and recording

Select synthetic mode explicitly for transport testing:

```sh
dora run apps/cpu-viewer/dataflows/live-cpu-synthetic-dataflow.yml
```

It publishes four CPUs at approximately 50% through the same real Dora and
binary WebSocket path. To record ten synthetic samples to MCAP:

```sh
cargo build -p system-monitor -p mcap-recorder --features system-monitor/dora
dora run apps/cpu-viewer/dataflows/phase0-dataflow.yml
```

The recorder refuses to overwrite `phase0.mcap`. It finalizes when its input
closes. It stores original Protobuf payloads and capture times; MCAP's
32-bit sequence wraps while the payload retains u64. Add the recorder as an
additional Dora subscriber for persistent recording alongside the live viewer.

## Verification

```sh
npm test --prefix apps/cpu-viewer/frontend
npm run build --prefix apps/cpu-viewer/frontend
cargo test -p telemetry-protocol -p telemetry-mcap -p cpu-viewer
# With either live dataflow running (about 32 seconds):
npm run test:live --prefix apps/cpu-viewer/frontend
```

The live check exercises actual binary messages, the shared frontend decoder,
CPU rates, 30-second eviction, and full name context on browser reconnect.

Protobuf schema: [`telemetry.proto`](../../../libs/telemetry/protocol/proto/telemetry.proto).
MCAP embeds its FileDescriptorSet on both channels. Install validation dependencies
with `python3 -m pip install -r libs/telemetry/protocol/python/requirements.txt` (prefer a virtual environment).
