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

The [Supervisor recipe](supervisor/README.md) runs the same recording policy on a
native host through the shared lifecycle driver. The `compose_monitor` example
runs this Compose recipe without foreign workloads.
