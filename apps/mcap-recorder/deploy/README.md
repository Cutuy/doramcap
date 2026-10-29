# MCAP Recorder deployment

Runs independent System Monitor and MCAP Recorder binaries in Dora 1.1.0. Each
headless generation retains `recording.mcap` using native BPF CPU telemetry.

```sh
cargo build --release --locked -p mcap-recorder -p system-monitor --features system-monitor/dora,system-monitor/bpf
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/system-monitor target/release/mcap-recorder "$stage/"
cp apps/mcap-recorder/deploy/flow.yml "$stage/"
cp libs/workload-docker/deploy/dora-supervise.sh "$stage/"
docker build -t doramcap/mcap-recorder:local -f apps/mcap-recorder/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app mcap-recorder`. The wrapper drains the recorder and
checks published counts before releasing its result. See [remaining work](../../project-explorer/server/docs/102727-remaining-apps.md#m2--mcap-recorder-complete).
