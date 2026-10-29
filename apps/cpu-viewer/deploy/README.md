# Native CPU Viewer wrapper

Runs CPU Viewer, BPF System Monitor and MCAP Recorder in an isolated Dora flow;
retains `cpu.mcap`. Requires Dora CLI 1.1.0 and a compatible kernel with BTF and
cgroup v2.

```sh
cargo build --release --locked -p cpu-viewer -p mcap-recorder -p system-monitor --features system-monitor/dora,system-monitor/bpf,system-monitor/task-bpf
npm ci --prefix apps/cpu-viewer/frontend
npm run build --prefix apps/cpu-viewer/frontend
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/cpu-viewer target/release/system-monitor target/release/mcap-recorder "$stage/"
cp -r apps/cpu-viewer/frontend/dist "$stage/dist"
cp apps/cpu-viewer/deploy/flow.yml "$stage/"
cp libs/workload-docker/deploy/dora-supervise.sh "$stage/"
docker build -t doramcap/cpu-viewer:local -f apps/cpu-viewer/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app cpu-viewer`. Only the collector acquires BPF, PERFMON
and SYSLOG file capabilities; cgroup/BTF mounts are read-only. The manager checks
collector losses and recorded counts before publishing. See [native qualification](../../project-explorer/server/docs/102727-remaining-apps.md#m8--native-collection-profiles-planned).
