# MCAP recorder deployment

```sh
cargo build --release --locked -p mcap-recorder
# Stage the binary as mcap-recorder, then build this Dockerfile as doramcap/mcap-recorder:local.
mcap-recorder --config bytes.pbtxt
```

Mount a [`RecorderConfig`](../proto/recorder.proto) such as [bytes.pbtxt](bytes.pbtxt)
at `/config/recorder.pbtxt` and a writable directory at `/results`. Relative paths
resolve beside the config. No BPF privileges are needed.

[recorder.yaml](recorder.yaml) and [flow.yml](flow.yml) are templates. The assembler
fills in service and node IDs, graph sources, Dora settings and mounts.

## What recording an app needs

- A recorder input in the Dora graph for each output.
- A matching `input` in the recorder config, with a unique topic and an encoding.
- Non-null uint8 array payloads. Other Arrow types fail the recording.

An app that already publishes bytes needs no code changes.

## Schemas

Schemas are optional for your own messages. Add one when readers should decode them.

| `message_encoding` | `schema_encoding` | `schema_path` file | Without a schema |
|---|---|---|---|
| `protobuf` | `protobuf` | Binary `FileDescriptorSet` with imports | Refused at startup |
| `json` | `jsonschema` | JSON Schema | Readers parse plain JSON |
| `application/octet-stream` | None | None | Readers see raw bytes |

Set `schema_name`, `schema_encoding` and `schema_path` together:

```sh
protoc --include_imports --descriptor_set_out=schema.bin my_message.proto
```

```pbtxt
input {
  input: "data"
  topic: "my_app/data"
  message_encoding: "protobuf"
  schema_name: "my_app.v1.MyMessage"
  schema_encoding: "protobuf"
  schema_path: "/config/schema.bin"
}
```

The recorder reads the file at startup and stores it unchanged. A wrong schema still
records and fails later in the reader.

## Timing

- Log time is recorder receipt time.
- `timestamp_key` names metadata with decimal-string Unix nanoseconds, used as publish time. Default: zero.
- `sequence_key` names metadata with a decimal-string u32 sequence. Default: zero.
- Declared metadata fields must exist.
- Each input's `<topic>/dora_metadata` JSON channel keeps one message per payload: the recorder's `ordinal` and the Dora `metadata`, with the same times. Non-finite floats fail the recording.

## Readiness

The healthcheck marker appears once the subscriber is attached, the file exists and
channels are ready. A declared stream path also requires growth before workload launch.

## Reports

Outputs and reports are created exclusively. `report_path` defaults to
`output_path + ".report.pbtxt"`.

| State | Meaning |
|---|---|
| `RUNNING` | In progress, or the process was killed and the file may be unfinished |
| `DRAINED` | Dora stop or input closure, with buffered messages written |
| `INTERRUPTED` | Stream ended without a stop or closure; exits with an error |
| `FAILED` | Invalid input, upstream error or finalization failure; exits with an error |

`finalized` and the received and written counts are reported separately. A finalized
file is readable, which says nothing about whether it is complete.

## Loss

- Reports keep `transport_loss_unknown: true`, because Dora has no per-input drop count.
- Dora 1.1 backpressure queues cap at `max(10 * queue_size, 100)`, so overflow can lose messages.
- Slow storage can stall producers on backpressure queues.

Unit tests cover config, payloads, timing, closure drain and partial reports. Docker
startup, Dora routing, OS interruption and slow storage need runtime qualification.
