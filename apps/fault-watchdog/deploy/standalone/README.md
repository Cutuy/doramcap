# Standalone Fault Watchdog

Run the existing watchdog beside a workload through shared Dora.
Build for the target architecture, then stage the binary:

```sh
cargo build --release --locked -p fault-watchdog --features mcap,otel
cp target/release/fault-watchdog apps/fault-watchdog/deploy/standalone/fault-watchdog
docker compose -f apps/fault-watchdog/deploy/standalone/compose.yaml build
```

Connect [flow.yml](flow.yml) to the workload's fault output. Override the Compose
user to match the Dora participant and mount a fresh writable results directory.
Choose the [live](config/watchdog.pbtxt), [MCAP](config/watchdog-mcap.pbtxt) or
[OTLP](config/watchdog-otel.pbtxt) configuration through the service command.

[Runtime contract](../../docs/102026-runtime.md) ·
[Fault protocol](../../../../libs/fault/protocol/README.md)
