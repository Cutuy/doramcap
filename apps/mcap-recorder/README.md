# MCAP recorder

An independent DORA subscriber that records snapshot and dictionary topics using
[`telemetry-mcap`](../../libs/telemetry/mcap/). Each recording embeds the Protobuf
schema and preserves original payloads, sequence and capture timestamps. The
process refuses to overwrite an existing recording and drains normal input
closure before finalization.

```sh
cargo build -p system-monitor -p mcap-recorder --features system-monitor/dora
dora run apps/cpu-viewer/dataflows/phase0-dataflow.yml
```

The demo configuration records ten synthetic samples. See the
[viewer guide](../cpu-viewer/docs/running.md) for composing live consumers.
Install `libs/telemetry/protocol/python/requirements.txt`, then validate transport,
embedded schemas, recording contents and finite-session shutdown:

```sh
python3 apps/mcap-recorder/tests/validate_dora.py --dora /path/to/dora --runs 2
```

Use `--bin-dir` with a custom Cargo target directory. `--backend bpf` requires
native build features and permission to load BPF programs.
