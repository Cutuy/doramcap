# MCAP recorder

A standalone Dora subscriber that records apps' byte messages to MCAP. It links no
message types: each channel declares its topic, encoding, schema file and source
metadata keys. Recording an app [needs configuration](deploy/README.md#what-recording-an-app-needs),
and protobuf channels require a schema. Payloads stay unchanged. ROS 2 input is planned.

![Recording paths](docs/110126-recording-paths.svg)

- **This recorder** captures declared Dora outputs through `storage-persistence`.
- **Embedded writers** stay in their apps and use the same writer.
- **Live view** uses Console's [live bridge](../../console/bridge/README.md) on the same outputs.

```sh
cargo build --release --locked -p mcap-recorder
mcap-recorder --config apps/mcap-recorder/deploy/bytes.pbtxt
```

The process requires a Dora attachment and writable configured output directory.
It creates recordings exclusively, drains normal input closure and writes a
separate report distinguishing finalization from drain and known loss evidence.

[Configuration, artifacts and deployment](deploy/README.md)
