# Console contracts

- `console-config.proto`: operator configuration and approved system apps.
- `execution.proto`: explicit Compose inputs, inspection snapshots and execution facts.
- `console.proto`: browser requests, status and private display checkpoints.
- [HTTP transport](../contract/transport.ts): routes, status codes, errors, bounds and live framing.
- [Browser API](../docs/api.md): HTTP payloads and authentication.
- [Native API](../server/native/index.d.ts): addon capabilities and execution transport.

The TypeScript server generates all three schemas and uses their message types in
the native client API. Its adapter owns protobuf encoding and decoding. Rust generates configuration and execution
schemas, reusing the existing Compose, Dora and workflow types.

Start resolves after workflow preflight accepts the supplied configuration snapshot.
Stop signals the workflow observation loop. Poll drains bounded step events; completion
remains available across polls. Close waits for workflow cleanup and owner-thread exit.

The server owns Console phases, request keys, configuration policy and display state.
The workflow owns repository admission, reset and resource cleanup.
