# Standalone Fault Watchdog

Run the existing watchdog beside a workload through shared Dora.
Build for the target architecture, then stage the binary:

```sh
cargo build --release --locked -p fault-watchdog --features mcap,otel
cp target/release/fault-watchdog apps/fault-watchdog/deploy/standalone/fault-watchdog
docker compose -f apps/fault-watchdog/deploy/standalone/compose.yaml build
```

Connect [flow.yml](flow.yml) to the workload's fault output. The image runs as root,
which matches a release Console's Dora participant; override the Compose user for any other participant, and mount a fresh writable results directory.
The service mounts one self-contained pbtxt; Console selects one during Review through
[launch profiles](../../../../console/docs/configuration.md#launch-profiles).
The service keeps `seccomp=unconfined` for the MCAP writer's io_uring calls.
In Console, declare `fault-watchdog/faults` as a `FAULT_PUBLICATION` live output
to show faults through the [live bridge](../../../../console/bridge/README.md).

[Runtime contract](../../docs/102026-runtime.md) ·
[Fault protocol](../../../../libs/fault/protocol/README.md)
