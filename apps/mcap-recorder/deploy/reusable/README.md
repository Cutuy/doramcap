# Recorder-only deployment

`recorder.yaml` starts the existing recorder binary without a producer or Dora
runtime. `flow.yml` connects its dynamic `workload-recorder` node to one selected
workload output. The assembler supplies matching service/node IDs, graph sources,
shared Dora settings, process user, read-only configuration and writable results
mounts. The recorder requires no BPF privileges.

Build with `cargo build --release --locked -p mcap-recorder`, stage the binary as
`mcap-recorder`, then build this Dockerfile as `doramcap/mcap-recorder-only:local`.
Mount [bytes.pbtxt](bytes.pbtxt) or [cpu.pbtxt](cpu.pbtxt) at `/config/recorder.pbtxt`
and a writable directory at `/results`. Paths must not contain existing outputs.
CPU recording uses the same binary in a separate instance with its own destination.

## Configuration

`mcap-recorder --config FILE.pbtxt` loads generated
[`RecorderConfig`](../../proto/recorder.proto) directly. Relative recording, report
and schema paths resolve beside that file. [cpu.pbtxt](cpu.pbtxt) declares CPU
accounting channels and their validation.

- Map each Dora input to a unique topic and declared message encoding.
- Optional `schema_name`, `schema_encoding` and `schema_path` embed supplied schema
  bytes together. Raw payloads remain unchanged; the producer owns their encoding.
- Accept only non-null uint8 arrays. Other Arrow types fail explicitly.
- Raw `timestamp_key` and `sequence_key` name decimal-string metadata fields.
  Declared fields must exist. Source sequences must fit MCAP's u32 field.
- Raw log time is recorder receipt time. Publish time is the supplied source Unix
  nanoseconds, or zero when unavailable. Missing source sequence is zero.
- CPU validation embeds the telemetry schema and preserves capture time and the
  payload's full sequence; its MCAP sequence uses the low 32 bits.

Each `dora.message` MCAP metadata record preserves typed Dora metadata as JSON,
receipt time and a recorder-local input ordinal. Non-finite float parameters fail
explicitly because JSON cannot preserve them. Channel metadata declares timing
and source-key conventions. Recorder ordinals never represent source sequences.

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
- Independent `finalized`, received/written counts and available CPU producer loss.

The workflow explicitly stops system nodes. The recorder drains buffered events
after every Dora stop cause until stream closure. On other errors it attempts footer finalization and reports both source
and finalization failures. Interruption/error exits unsuccessfully even when the
partial file is readable. Process kill or OS termination can leave `RUNNING`
evidence and an unfinished file; the workflow must not publish it as complete.

Dora exposes no trustworthy per-input transport-drop total here, so reports retain
`transport_loss_unknown: true`. With Dora 1.1, backpressure queues have a safety cap
of `max(10 * queue_size, 100)`; overflow can still lose messages. Slow storage can
also stall publication while CPU collection's bounded handoff drops batches.
A finalized footer establishes readability, independently of recording completeness.

Docker startup, real Dora/SHM routing, OS interruption and slow-storage queue behavior
require runtime qualification. Unit tests exercise config, payloads, timing, CPU
validation, closure drain and partial reports without claiming transport coverage.
