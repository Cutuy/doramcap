![Console project recording](110226-project-recording.svg)

# Console project recording

Created: 2026-11-02. Status: on hold.

Record a project's own Dora messages from Console, with no app code changes.

- **Graph:** the project supplies its Dora graph beside its Compose file.
- **Choice:** Review lists the graph's outputs; the user ticks the topics to record.
- **Recorder:** Console generates its graph inputs, config and Compose project, as it does for the live bridge.
- **Schemas:** optional. doramcap's own topics keep theirs; project topics default to raw bytes.

Build after PRs 85–88, which change `discover` and the project flow.

## Decisions to make before Milestone 2

| Decision | Options | Recommended |
|---|---|---|
| Queue policy | `backpressure` can stall publishers; `drop_oldest` loses messages with no per-input count | `backpressure`, matching `cpu-recorder` |
| Default selection | All outputs, or none | None, because high-rate topics are costly |

## Milestone 1: Project graph

**Work items**

- [ ] `discover` accepts an optional `flow.yml` beside the Compose file.
- [ ] `Graphs.build` merges it with the base and system app graphs.
- [ ] Review surfaces graph errors, including nodes with no same-named host-networked service.

**Validation**

- [ ] Server tests: a project with and without `flow.yml`, a malformed graph and a duplicate node ID.
- [ ] Real run: a Dora project publishes and subscribes through Console.

## Milestone 2: Record selected topics

**Work items**

- [ ] Add the project's outputs and the selected topics to the Review and Prepare protobuf messages.
- [ ] Show outputs with "Record" checkboxes in Review.
- [ ] `bindRecorder` writes the graph fragment, recorder pbtxt and Compose project, following `bindBridge`.
- [ ] Label inputs `application/octet-stream` and declare `topics.mcap` as a run recording.
- [ ] Run again restores the selected topics.

**Validation**

- [ ] Server tests: generated graph inputs and recorder config, with an unknown topic refused.
- [ ] Frontend tests: the checkboxes and a restored selection.
- [ ] Real run: messages in `topics.mcap` match what the app published, and the file downloads.
- [ ] Real run: a project with nothing selected starts no recorder.

## Milestone 3: Arrow payloads

**Work items**

- [ ] The recorder stores non-uint8 outputs as Arrow IPC, under a declared encoding.
- [ ] Generated configs select this encoding, keeping uint8 outputs as raw bytes.
- [ ] Update the recorder guide.

**Validation**

- [ ] Recorder test: an Arrow struct round-trips with its types.
- [ ] Real run: a project publishing a non-uint8 output records without failing.

## Milestone 4: Optional topic schemas

**Work items**

- [ ] A project can name a schema file for a topic.
- [ ] Console mounts it, and sets the input's encoding and schema settings.
- [ ] Result shows which recorded topics have no schema.

**Validation**

- [ ] Server tests: a valid schema, a missing file and a path outside the project.
- [ ] Real run: a protobuf topic opens decoded in a viewer, and a topic with no schema still records.
