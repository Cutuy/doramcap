# CPU Viewer deployment

CPU Viewer combines BPF System Monitor and MCAP Recorder in a shared Dora experiment.
It requires a compatible kernel with BTF and cgroup v2.

Build native image inputs from the repository root:

```sh
cargo build --release --locked -p cpu-viewer -p mcap-recorder -p system-monitor \
  --features system-monitor/bpf,system-monitor/task-bpf,system-monitor/dora
```

The Dockerfile takes these binaries and frontend `dist/`. The gallery uses
`registration.pbtxt` and starts shared Dora before the app services.

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
