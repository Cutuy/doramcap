# Workflow protocol

Transport-independent Protobuf workflow events, codecs and descriptors.
Publishers own sequence spaces, session identities and clocks; transports own
framing and size admission.

```rust
let event = workflow_protocol::Event {
    version: workflow_protocol::VERSION,
    ..Default::default()
};
let bytes = workflow_protocol::encode(&event);
```

[Wire contract](docs/wire-protocol.md) · [Contract ownership](../docs/102626-contract-ownership.md).
Run `cargo test -p workflow-protocol`.
