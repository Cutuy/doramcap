# Console Rust bridge

Executes the recorded Compose workflow on its owner thread and exposes native
parsing, file and lock capabilities to the TypeScript server.

See the [API declarations](index.d.ts) and [protobuf contracts](../../proto/README.md).
The server loads the compiled library through N-API.

```sh
cargo build --release --locked -p console-native
cargo test --release --locked -p console-native
```

Start is acknowledged by workflow preflight. Stop uses a separate control queue.
Poll drains bounded events while retaining completion facts. Close waits for
workflow cleanup before joining the owner thread.
