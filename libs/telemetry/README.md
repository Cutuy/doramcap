# Telemetry

Publication contracts, codecs, and recording adapters for measured data.
Current contracts describe CPU snapshots, dictionary checkpoints and versioned
workflow events. Applications supply live transports and bounded replay.

| Crate | Responsibility |
|---|---|
| [telemetry-protocol](protocol/) | Protobuf schema, wire codec, descriptors, and Python reader |
| [telemetry-mcap](mcap/) | Schema/channel registration, packet validation, and MCAP recording |

[Resources](../resources/) supplies the current measurement types.
[Storage persistence](../storage-persistence/) supplies the underlying writer.
Applications own transport subscriptions and recording lifecycle.
