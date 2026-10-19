# Storage Observatory

Live web demo of the managed-buffer MCAP writer. CPU packets from the system
monitor and irregular small heartbeat packets share a writer with one io_uring.

```sh
cargo build -p storage-demo -p system-monitor
cargo run -p storage-demo
# Open http://127.0.0.1:8091
```

The default CPU backend is synthetic and is labeled in the UI. The writer performs
real O_DIRECT writes with 4096-byte alignment. `--buffered` selects buffered I/O.
`--monitor PATH` selects a monitor executable; `--backend bpf` uses native CPU
collection and requires the monitor process's Linux capabilities. Build that
monitor with `cargo build -p system-monitor --features bpf`. Use the AGENTS.md
procedure for capability-required validation. The existing launcher keeps the web
server unprivileged:

```sh
target/debug/storage-demo --backend bpf \
  --monitor "$PWD/apps/system-monitor/scripts/run-native-monitor.sh"
```

The web server binds loopback.

The page shows:

- Combining occupancy, age and free managed buffers.
- Actual queued write ranges and retired write CQEs, retained in a bounded log.
- Parsed MCAP records and the actual final 256 bytes read back from disk.
- CPU, heartbeat and large-probe activity, plus input drop counters.

Use **Stop recording** to drain writes, seal and sync the current MCAP, and stop
the monitor. The page remains available for inspection and restart.

Use **Start new session** to restart collection and reset counters, including
after the capture timer expires. It removes the previous MCAP captures in this
demo’s directory. Other files remain untouched.

Use **+16 heartbeats** to combine a burst, **+96 KiB probe** to exercise contiguous
encoding, **Flush now**, and **Seal & rotate** to inspect the footer and final
MCAP magic. The last sealed file is downloadable. The ribbon displays recent
record order with logarithmic widths; the table gives exact byte offsets/sizes.

`--dir NEW_DIR` selects an exclusively created capture directory. The default is
a unique directory under `/tmp`. `--port N` selects a port; `--seconds N` stops
recording and seals the file after N seconds while keeping the page available.
Ctrl-C stops the server, seals the current file and reaps its monitor subprocess.

The demo input queue holds 16 packets. The MCAP pool starts with eight 256-KiB buffers and grows on demand.
UI snapshots run at 25 Hz. File inspection reads complete records once outstanding
writes retire. Event sequence numbers expose history eviction. A write CQE is
completion evidence; seal/rotate also establishes logical EOF and calls fsync.

Diagnostics serialization, Protobuf decoding and the subprocess pipe allocate
and copy in the demo. Storage diagnostics retain fixed-size events; existing
storage tests enforce allocation-free write/encoding paths. No external frontend
packages or assets are loaded.

## Validation

```sh
cargo test -p storage-demo -p storage-persistence
cargo build -p storage-demo --bins --examples
cargo build -p system-monitor
python3 apps/storage-demo/validate.py --bin-dir target/debug
```

The HTTP test verifies the SSE stream, input channels, commands, CQE evidence,
rotation, exact tail, download bytes and independent MCAP/Protobuf readback.
The ARM64 worker runs it after building the workspace.
See the [engineering plan](docs/101826-live-storage-demo.md).

## Latency and load

Timing panels show rolling last/p99/max values over 256 observations. Write
latency runs from the submission call to CQE observation and includes polling
delay. CPU sampling deviation uses collector monotonic timestamps; arrival
deviation also includes transport and owner stalls. Sequence gaps adjust the
expected interval and are counted separately.

Set probe count (1–256) and spacing (0–1000 ms), then choose **Run 96 KiB probe
burst**. Zero spacing runs back-to-back with control checks between probes.
Positive spacing has 5 ms loop granularity. **Cancel burst** leaves collection
running; **Stop recording** cancels the remainder and seals the file.

Use `--start-paused` to open the page without starting collection or creating an
MCAP file. Choose **Start new session** when ready.

The single-injection buttons produce persistent writer-confirmed receipts showing
encoded count/bytes, MCAP sequence range and encode/commit time. The last eight
receipts stay visible through a session; manual activity rows are highlighted.
Encoding receipts precede write completion and durability.

The demo explicitly enables `storage-persistence/diagnostics`. Production writer
builds can omit that feature to remove event-history storage, write timing and
diagnostic-only methods. Build the production application separately because
Cargo unifies dependency features within a build.
