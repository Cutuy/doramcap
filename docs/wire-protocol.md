# Snapshot binary protocol v1

Dora carries an application-defined UInt8 array. The application payload is
MessagePack, encoded as positional arrays by `rmp-serde` (not named maps).
The browser receives the same schema in binary WebSocket messages at `/stream`.
There is no JSON, base64 or decimal-string encoding on either snapshot path.
This does not assume Dora's transport/control protocol is protobuf.

Every frame is one MessagePack value. Fields below are in exact array order:

| Tuple | Ordered fields |
|---|---|
| Frame (11) | version, snapshot, source, task_collection_enabled, task_capacity, session_id, dictionary_revision, dictionary_full, dictionary_overflow, dictionary, viewer_ingress |
| Snapshot (10) | schema_version, sequence, capture_unix_ns, monotonic_start_ns, monotonic_end_ns, dropped_batches, capacity_overflow, records, task_records, task_losses |
| CPU record (7) | cgroup_id, parent_id, cpu, flags, runtime_ns, user_ns, system_ns |
| Task record (2) | identity, charged_runtime_ns |
| Task identity (6) | start_boottime_ns, cgroup_id, tid, tgid, cpu, flags |
| Task losses (3) | dropped_events, dropped_runtime_ns, identity_errors |
| Dictionary entry (3) | id, parent_id, path (string or nil) |
| Viewer ingress (7) | total_bytes, total_messages, bytes_per_second, messages_per_second, capture_to_receive_ms, mean_capture_to_receive_ms, invalid_timestamps |

Versions are currently 1. Unsigned counters/IDs retain their full u64 range.
Rates and latency are floating point or nil during warmup/unavailable periods.
`viewer_ingress` is nil on Dora and filled by the viewer on browser frames.
Flags and counter semantics are defined in monitor-core and task-accounting.
Unknown versions, malformed/truncated data and trailing bytes are rejected.
Frame size is limited to 128 MiB; the browser also limits collection/string
lengths. Any field order/type change requires a version change and coordinated
producer/consumer update. This is a breaking change from the earlier JSON demo.

Dora publishes two topics: `snapshots` at 10 Hz and `dictionary` at startup
and approximately every 2 seconds (monotonic publication time). Snapshots have
an empty dictionary array and carry `session_id` + `dictionary_revision`.
Dictionary packets are full checkpoints with this separate six-element array:
`[version, session_id, revision, overflow, entries, capture_unix_ns]`.
Revision advances when entries are added; repeated unchanged checkpoints keep
the revision. Newly discovered groups can show numeric IDs until the next
checkpoint. The dictionary remains bounded by the configured record capacity.

WebSocket forwards the two packet types independently. The viewer retains only
the latest of each and sends the cached dictionary before the cached snapshot
on reconnect. Cross-topic ordering is not assumed: browsers use a dictionary
only for the same session with revision at least the requested revision (the
dictionary is append-only within a session). Missing dictionaries give numeric
labels, never another session's names. A late viewer gets a full checkpoint
within about 2 seconds. Slow browsers skip intermediate snapshots.

`FLAG_VIRTUAL_KERNEL` (128) puts a CPU record in a separate virtual namespace:
`cgroup_id=0` denotes `virtual/kernel`, with `FLAG_RUNTIME`, CPU and cumulative
PF_KTHREAD runtime. It is not real cgroup ID 0; user/system are unavailable.
No real dictionary entry is used for this reporting group. Consumers must honor
the flag, and never sum this series with native cgroup totals. Task filtering
happens in BPF before map insertion; publication aggregates by CPU and sends an
empty `task_records` array. Task-loss counters remain available. The task map
still retains kernel lifetimes internally and may eventually fill.

Ingress byte rates include both Dora topics; message counts/rates and latency
refer only to numeric snapshots. The 10 Hz path contains no cgroup names or task
names (only small envelope strings such as session/source). MCAP records both
`doramcap/snapshots` and `doramcap/dictionary` channels. Standalone framed output
remains self-contained with an embedded dictionary, unlike the Dora path.

The TypeScript decoder uses `useBigInt64` and converts integer fields into
in-memory strings for the existing BigInt rate model. That adapter does not
inflate network traffic. A Rust-generated fixture with u64::MAX verifies
cross-language decoding (`frontend/src/fixtures/snapshot-v1.msgpack`).

MCAP records the original binary payload, with `message_encoding=messagepack`
and channel metadata `protocol=doramcap.snapshot.msgpack.v1`. There is no JSON
schema registration. Dora sequence/capture metadata are checked against the
binary body; MCAP timestamps preserve the original capture time. Legacy JSON
MCAP files remain legacy recordings, not inputs to this decoder.

Standalone monitor output (`--output`, or stdout without `--dora`) is a stream
of u32 little-endian byte lengths followed by MessagePack frames. `--check`
remains a human-readable JSON capability report; it is not telemetry transport.
