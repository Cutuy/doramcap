# System Monitor wrapper

The default headless profile retains synthetic Protobuf samples in `monitor.pb`;
collector diagnostics remain private.

```sh
cargo build --release --locked -p system-monitor
stage=$(mktemp -d)
cp target/release/system-monitor "$stage/"
cp apps/system-monitor/deploy/entrypoint.sh "$stage/"
docker build -t doramcap/system-monitor:local -f apps/system-monitor/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app system-monitor`. Install one trusted registration template
before starting the gallery to select another profile.

## Embedded recording

`registration.recording.pbtxt` and `compose.recording.yaml` enable rotated MCAP
output with the reviewed io_uring seccomp profile. Build the default image first:

```sh
stage=$(mktemp -d)
cp apps/system-monitor/deploy/recording-entrypoint.sh "$stage/"
docker build -t doramcap/system-monitor:recording -f apps/system-monitor/deploy/Dockerfile.recording "$stage"
rm -r "$stage"
```

## Native collection

`registration.native.pbtxt` and `compose.native.yaml` select BPF with a host cgroup
namespace, read-only cgroup/BTF mounts and BPF, PERFMON, SYSLOG and DAC_OVERRIDE.

```sh
cargo build --release --locked -p system-monitor --features bpf
stage=$(mktemp -d)
cp target/release/system-monitor "$stage/"
cp apps/system-monitor/deploy/entrypoint.sh "$stage/"
docker build -t doramcap/system-monitor:native -f apps/system-monitor/deploy/Dockerfile.native "$stage"
rm -r "$stage"
```

For kernel-thread accounting, use `registration.native-task.pbtxt`, build with
`--features bpf,task-bpf`, and tag the image `doramcap/system-monitor:native-task`.
See [native qualification](../../project-explorer/server/docs/102727-remaining-apps.md#m8--native-collection-profiles-planned).
