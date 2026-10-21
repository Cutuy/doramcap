# Telemetry MCAP

A reusable recording sink for Protobuf snapshot and dictionary packets. It owns
schema/channel registration, recording-boundary validation and finalization.
Callers provide encoded payloads; the sink writes their original bytes with
capture timestamps and an embedded FileDescriptorSet.

Run `cargo test -p telemetry-mcap`. The [recorder application](../../../apps/mcap-recorder/)
owns DORA subscriptions, process lifecycle and end-to-end recording tests.
