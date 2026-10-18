# Phase 0 Dora recording

From the repository root, build both processes with Dora enabled:

```sh
cargo build --workspace --features dora
dora run examples/phase0-dataflow.yml
```

Use a Dora CLI version matching the pinned `dora-node-api` version (1.1.0).
The monitor explicitly selects synthetic data and exits after ten sampling
attempts. The recorder exits when its snapshot input closes and finalizes MCAP.
The recorder refuses to overwrite an existing `phase0.mcap`; move the previous
recording before repeating the run. Relative recording paths are resolved by
the recorder's working directory as set by Dora.

Messages are UTF-8 JSON transported as Dora UInt8 arrays. Each message contains
its dictionary context and original `sequence` and `capture_unix_ns`, also
carried in Dora metadata as decimal strings. MCAP registers one JSON Schema and
one `doramcap/snapshots` channel. Both MCAP timestamps use the original capture
time; MCAP's 32-bit sequence wraps while the JSON retains the full 64-bit value.
The schema deliberately permits additional backend-specific fields.

MCAP encoding, JSON validation, buffering and disk writes all occur in the
recorder process. The system monitor's publisher may allocate; the periodic
collection thread has a separate allocation contract. The example's Dora input
queue is bounded but is not a guarantee of lossless delivery through arbitrary
consumer stalls. Sequence numbers expose gaps.

Run the real publisher/recorder shutdown regression after building:

```sh
python3 examples/validate_dora.py --dora /path/to/dora --bin-dir target/debug
```

The script isolates binaries and output in a temporary directory, launches Dora,
and reads the resulting uncompressed MCAP to verify all ten final samples,
sequences, and original timestamps. This specifically guards against stopping
the recorder on a lifecycle notification before buffered inputs are drained.
