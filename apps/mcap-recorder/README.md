# MCAP recorder

A Dora subscriber that records configured byte inputs using
[`telemetry-mcap`](../../libs/telemetry/mcap/). Each channel declares its topic,
encoding and optional schema. CPU snapshot/dictionary modes validate telemetry
before writing and preserve its original payloads and capture timestamps.

```sh
cargo build --release --locked -p mcap-recorder
mcap-recorder --config apps/mcap-recorder/deploy/reusable/bytes.pbtxt
```

The process requires a Dora attachment and writable configured output directory.
It creates recordings exclusively, drains normal input closure and writes a
separate report distinguishing finalization from drain and known loss evidence.
The existing `mcap-recorder OUTPUT.mcap` invocation records CPU telemetry.

[Configuration, artifacts and reusable deployment](deploy/reusable/README.md) ·
[Gallery deployment](deploy/README.md) ·
[CPU viewer guide](../cpu-viewer/docs/running.md)
