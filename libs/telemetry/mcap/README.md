# Telemetry MCAP

Generic MCAP storage for caller-declared topics, schemas, encodings and timestamps.
`RecordingSink` preserves supplied payload bytes and refuses existing output files.
Callers own payload validation and application policy.

Run `cargo test --release --locked -p telemetry-mcap`. The
[recorder application](../../../apps/mcap-recorder/) loads channel configuration
from pbtxt and owns Dora subscriptions and shutdown.
