# MCAP recorder deployment

`recorder.yaml` starts the recorder without a producer or Dora runtime. `flow.yml`
connects its dynamic `workload-recorder` node to one selected workload output. The assembler supplies matching service/node IDs, graph sources,
shared Dora settings, process user, read-only configuration and writable results
mounts. The recorder requires no BPF privileges.

Build with `cargo build --release --locked -p mcap-recorder`, stage the binary as
`mcap-recorder`, then build this Dockerfile as `doramcap/mcap-recorder:local`.
Mount a configuration such as [bytes.pbtxt](bytes.pbtxt) at `/config/recorder.pbtxt`
and a writable directory at `/results`. Paths must not contain existing outputs.

## Configuration

`mcap-recorder --config FILE.pbtxt` loads generated
[`RecorderConfig`](../proto/recorder.proto) directly. Relative recording, report
and schema paths resolve beside that file.

- Map each Dora input to a unique topic and declared message encoding.
- `schema_name`, `schema_encoding` and `schema_path` embed a supplied schema; set
  all three or none. See [Schemas](#schemas).
- Accept only non-null uint8 arrays. Other Arrow types fail explicitly.
- `timestamp_key` and `sequence_key` name decimal-string metadata fields.
  Declared fields must exist. Source sequences must fit MCAP's u32 field.
- Log time is recorder receipt time. Publish time is the supplied source Unix
  nanoseconds, or zero when unavailable. Missing source sequence is zero.

Each `dora.message` MCAP metadata record preserves typed Dora metadata as JSON,
receipt time and a recorder-local input ordinal. Non-finite float parameters fail
explicitly because JSON cannot preserve them. Channel metadata declares timing
and source-key conventions. Recorder ordinals never represent source sequences.

## Schemas

The recorder links no message types, so it can record any app's messages. Each
input's configuration supplies the schema that readers use to decode them.

| `message_encoding` | `schema_encoding` | `schema_path` file | Without a schema |
|---|---|---|---|
| `protobuf` | `protobuf` | Binary `FileDescriptorSet`, including imports | Readers cannot decode the channel |
| `json` | `jsonschema` | JSON Schema | Readers can still parse the JSON |
| Other, such as `application/octet-stream` | None | None | Readers see opaque bytes |

- `schema_name` is the message type's full name, such as `doramcap.telemetry.v1.Packet`.
- The recorder reads the file once at startup and fails if it cannot.
- It stores the schema bytes and payloads unchanged, without validating either.
  A wrong schema, or `protobuf` with no schema, still records and fails later in the reader.
- Inside a container, mount the file read-only and use its mounted path.

Generate a protobuf schema and declare it:

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

Console's [topic recording](../../../console/release/system-apps/cpu-recorder/recorder.pbtxt)
follows this pattern.

## Readiness and artifacts

The existing approved `dora_app::environment` readiness marker appears after
subscriber attachment, file creation and channel initialization. The provided
healthcheck uses that marker; consumers become ready before producers publish.
A recording declaration with an empty stream path waits for file creation.
A configured stream additionally requires growth before workload launch.

The recorder creates output and report files exclusively. `report_path` defaults
to `output_path + ".report.pbtxt"`. The generated report records:

- `RUNNING` initially; `DRAINED` after Dora stop or confirmed input closure and buffered drain.
- `INTERRUPTED` on stream end without a stop request or confirmed input closure.
- `FAILED` on invalid inputs, upstream errors or finalization failure.
- Independent `finalized` and received/written counts.

The workflow explicitly stops system nodes. The recorder drains buffered events
after every Dora stop cause until stream closure. On other errors it attempts footer finalization and reports both source
and finalization failures. Interruption/error exits unsuccessfully even when the
partial file is readable. Process kill or OS termination can leave `RUNNING`
evidence and an unfinished file; the workflow must not publish it as complete.

Dora exposes no trustworthy per-input transport-drop total here, so reports retain
`transport_loss_unknown: true`. With Dora 1.1, backpressure queues have a safety cap
of `max(10 * queue_size, 100)`; overflow can still lose messages. Slow storage can
also stall producers that publish to backpressure queues.
A finalized footer establishes readability, independently of recording completeness.

Docker startup, real Dora/SHM routing, OS interruption and slow-storage queue behavior
require runtime qualification. Unit tests exercise config, payloads, timing, closure drain and partial reports without claiming transport coverage.
