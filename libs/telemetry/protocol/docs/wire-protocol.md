# CPU telemetry Protobuf protocol v2

The canonical schema is [`telemetry.proto`](../proto/telemetry.proto).
Each payload is one `doramcap.telemetry.v1.Packet`, whose `oneof body` contains
a `SnapshotFrame` or a `DictionaryFrame`. The envelope version is 2; the CPU
counter schema version is 1. The package's `v1` identifies the Protobuf schema
family. Rust uses generated prost bindings; TypeScript uses generated protobufjs
bindings. The application uses DORA pub/sub and binary WebSocket transport.

| Boundary | Artifact |
|---|---|
| eBPF → collector | Native BPF records / task map entries |
| Collector → DORA | Protobuf Packet bytes in a UInt8 array |
| DORA → viewer / recorder | Original Protobuf Packet bytes |
| Viewer → browser `/stream` | Binary WebSocket Packet; snapshots gain ingress metrics |
| Recorder → MCAP | Original Packet bytes on two channels, with embedded schema |
| Standalone file / stdout | Repeated u32 little-endian byte length + Packet bytes |

## DORA publication buffers

The publisher builds an immutable Protobuf packet, checks `encoded_len()` against
`MAX_FRAME_BYTES`, and encodes directly into the destination supplied by DORA's
`send_output_raw()`. Both snapshot and dictionary topics use this path. DORA
owns buffer allocation and lifetime, selecting shared memory or its heap
fallback. The application creates no intermediate serialized byte vector on
this publication path. Constructing the generated packet and consumer-side
Protobuf decoding still allocate. Standalone output uses a reusable byte vector.

## Data and frequency

`snapshots` publishes at 10 Hz. Records contain integer cgroup/parent IDs,
CPU index, availability flags and cumulative nanosecond counters. Counters,
timestamps and IDs use `uint64` varints; CPU/flags use `uint32`. Rates and latency
metrics use optional `double` fields, preserving unavailable versus zero.
The 10 Hz path carries an empty dictionary and empty `task_records`; small
session/source strings remain in the envelope.

`dictionary` publishes full checkpoints at startup and about every 2 seconds.
Entries map numeric IDs to parent IDs and optional paths. An absent path means
unresolved; an empty path is a valid root name. Snapshots reference a
`session_id` and `dictionary_revision`. Dictionary revisions track metadata
changes, including path refreshes. Entries are bounded by configured capacity.
Standalone snapshots embed their dictionary for self-contained validation.

The viewer retains the latest packet of each type. It sends the dictionary
before the snapshot on reconnect. Browsers join only the same session with a
dictionary revision at least the snapshot's requested revision. Cross-topic
ordering can temporarily leave labels numeric. Slow browsers skip intermediate
snapshots. Display history expires after 30 seconds.

## Kernel accounting and timestamps

`FLAG_VIRTUAL_KERNEL` (128) selects a separate reporting namespace:
`cgroup_id=0` represents `virtual/kernel`, with `FLAG_RUNTIME`, CPU index and
cumulative PF_KTHREAD runtime. User/system counters are unavailable for this
series. Consumers must honor the flag and keep it separate from native cgroup
totals. BPF filters tasks before insertion; publication aggregates their counters
by CPU. Kernel task lifetimes remain internal to the bounded task map. Loss
counters remain on the wire.

`capture_unix_ns` is captured at collection; monotonic start/end timestamps
measure collection duration. Viewer ingress latency measures capture to backend
receipt. Browser capture latency uses a calibrated server-clock mapping from
`/clock`, with uncertainty and freshness checks. `viewer_ingress` is absent on
DORA snapshots and populated in browser packets. Byte rates include both DORA
topics; message rates and latency refer to numeric snapshots.

## MCAP and compatibility

Both `doramcap/snapshots` and `doramcap/dictionary` use `message_encoding=protobuf`
and a schema named `doramcap.telemetry.v1.Packet`, `encoding=protobuf`. Schema data
is a binary `FileDescriptorSet`, including dependencies, so readers can decode
a recording from its embedded schema. Channel metadata identifies
`protocol=doramcap.telemetry.protobuf.v2`. MCAP preserves original payload bytes,
capture timestamps and the low 32 bits of sequence; payloads preserve full u64
sequences. Recording remains an optional DORA subscriber.

Protobuf accepts unknown additive fields. Keep field numbers/types stable and
reserve deleted fields; incompatible changes require a new protocol version and
coordinated deployment. Decoders check version, body and a 128 MiB packet limit.
Old MessagePack/JSON recordings require their corresponding legacy reader.

JavaScript decoding uses Long-backed uint64 fields, then exact decimal strings
and BigInt for rate calculation. This conversion is in memory. The Rust fixture
`apps/cpu-viewer/frontend/src/fixtures/snapshot-v2.pb` tests u64::MAX across languages.

After schema edits, regenerate checked-in artifacts:

```sh
npm run generate:proto --prefix apps/cpu-viewer/frontend
cargo run -p telemetry-protocol --example schema -- libs/telemetry/protocol/proto/telemetry_descriptor.bin
cargo run -p telemetry-protocol --example fixture -- apps/cpu-viewer/frontend/src/fixtures/snapshot-v2.pb
```

Cargo generates Rust bindings and the embedded descriptor automatically using a
vendored protoc. Python validators use the official protobuf runtime (install
`libs/telemetry/protocol/python/requirements.txt`); the MCAP validator builds its decoder from the
recording's schema. `--check` remains a human-readable JSON capability report.
