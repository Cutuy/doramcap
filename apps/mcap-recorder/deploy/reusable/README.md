# Recorder-only deployment

`recorder.yaml` supplies the existing CPU recorder without a collector, Dora
coordinator or daemon. `flow.yml` connects one selected CPU producer to it.
The experiment supplies the shared Dora connection, node identity, user and
writable `/results` bind mount. The recorder needs no BPF capabilities.

Build `mcap-recorder` with `cargo build --release --locked -p mcap-recorder`.
Stage that binary as `mcap-recorder` and build this Dockerfile with the stage as
its context, tagging the image `doramcap/mcap-recorder-only:local`.

The current binary accepts `mcap-recorder OUTPUT.mcap` and validates CPU protobuf
snapshot/dictionary inputs. It creates the file exclusively and embeds the
telemetry schema. Normal input closure drains the event stream before finalizing.
The workflow stops the producer first and drains the recorder afterward.

A finalized file establishes MCAP readability. Compare producer publication and
loss evidence separately to establish recording completeness. Backpressure queues
can stall publication, and the collector's bounded handoff can still lose batches.

`telemetry_mcap::RecordingSink` supports separately declared byte channels,
schemas, source/receipt timestamps and transport metadata. Configurable workload
recording in this binary is pending the pbtxt configuration implementation.
Docker startup and slow-storage queue behavior require runtime qualification.
