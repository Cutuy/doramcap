# Console Rust bridge

Executes the recorded Compose workflow on its owner thread and exposes native
parsing, file and lock capabilities to the TypeScript server.

See the [API declarations](index.d.ts) and [protobuf contracts](../../proto/README.md).
The server's TypeScript adapter implements this typed API using generated protobuf
messages. The compiled N-API library exchanges protobuf bytes behind that adapter;
application callers use the generated message types. The server generation step
provides the referenced `src/generated/console.cjs` module.

```sh
cargo build --release --locked -p console-native
cargo test --release --locked -p console-native
```

Start is acknowledged by workflow preflight. Stop uses a separate control queue.
Poll drains bounded events while retaining completion facts. Close waits for
workflow cleanup before joining the owner thread.
