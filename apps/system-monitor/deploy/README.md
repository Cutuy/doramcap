# System Monitor deployment

The native BPF collector writes real CPU and kernel-thread telemetry to
`monitor.pb`. It requires host cgroup/BTF access and the capabilities declared
in `compose.yaml`.

```sh
cargo build --release --locked -p system-monitor --features bpf,task-bpf
stage=$(mktemp -d)
cp target/release/system-monitor "$stage/"
docker build -t doramcap/system-monitor:local -f apps/system-monitor/deploy/Dockerfile "$stage"
rm -r "$stage"
```

`registration.pbtxt` declares the headless gallery app. `recording.yaml` is the
shared Docker workflow's supporting collector: it writes `cpu.pb` and `cpu.mcap`
until stopped. It uses `seccomp=unconfined` for the direct MCAP writer’s io_uring calls.

A native run uses the fixed [Supervisor recipe](supervisor/recording.conf).
Set the executable paths, a fresh output directory and an existing delegated
cgroup parent in [run.pbtxt](supervisor/run.pbtxt), then run from the repository root:

```sh
cargo build --profile ci --locked -p system-monitor --features bpf
cargo build --profile ci --locked -p workload-supervisor --example supervisor_monitor
target/ci/examples/supervisor_monitor apps/system-monitor/deploy/supervisor/run.pbtxt 5
```

The host needs Linux >=6.8, cgroup v2, kernel BTF, Supervisor 4.2.5,
BPF/PERFMON/SYSLOG access and sufficient locked memory (256 MiB recommended).
The example records for five seconds after readiness; Ctrl-C requests orderly stop.
See the [backend](../../../libs/workload-supervisor/README.md) for architecture and
native tests, and the [plan](../../../libs/workload-lifecycle/docs/102926-supervisor-monitor-mvp.md)
for validation evidence. The `compose_monitor` example runs the Compose recipe
without foreign workloads.
