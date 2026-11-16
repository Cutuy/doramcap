# Console contracts

- `console-config.proto`: operator configuration and approved system apps.
- `execution.proto`: Compose run plans, inspection snapshots and execution facts.
- `analysis.proto`: comparison facts, shared comparison enums and analysis requests/responses.
- `console.proto`: browser requests, status, run history, legacy comparison rows and per-run analysis inputs.
- [HTTP transport](../contract/transport.ts): routes, status codes, errors, bounds and live framing.
- [Native API](../server/native/index.d.ts): addon capabilities and execution transport.

TypeScript and Rust generate these schemas and reuse the Compose, Dora and
workflow types. The native client adapter owns protobuf encoding and decoding.

Start resolves after workflow preflight accepts the supplied configuration snapshot.
Stop signals the workflow observation loop. Poll drains bounded step events; completion
remains available across polls. Close waits for workflow cleanup and owner-thread exit.

The server owns Console phases, request keys, configuration policy and display state.
The workflow owns repository admission, reset and resource cleanup.
