# Browser API

[console.proto](../proto/console.proto) defines the messages.
[transport.ts](../contract/transport.ts) binds them to HTTP routes, errors and limits.
Both builds generate codecs from the schema and copy this shared transport module.

Requests and responses use `application/x-protobuf`. POST bodies are protobuf
messages; command responses and errors are unframed messages. The browser consumes
generated message classes directly, including numeric enums and lossless 64-bit values.

| Method and path | Request | Response | Success |
| --- | --- | --- | --- |
| `POST /api/prepare` | `PrepareRequest` | `Review` | 200 |
| `POST /api/start` | `StartRequest` | `StartResponse` | 202 |
| `POST /api/stop` | `StopRequest` | `StopResponse` | 200 |
| `GET /api/current` | — | `Status` | 200 |
| `GET /api/live` | — | Framed `Status` messages | 200 |
| `GET /api/artifacts/:runId/:kind` | — | File bytes | 200 |

Omit `PrepareRequest.system_apps` to use configured defaults. Set an empty
`SystemAppSelection.ids` list to select no optional apps. Artifact kinds are declared
recording IDs or `report`; authorization checks the current completed run.

All routes use HTTP Basic authentication (`console` and the configured secret).
POST requests require the configured Origin. Live requests validate Origin when supplied.
Errors carry `ConsoleError.code`; the shared error table specifies status codes.
Authentication failures use 401 and Origin rejection uses 403. Preflight details stay private.

Each live frame has a four-byte unsigned little-endian payload length followed by a
`Status` message. Reconnect starts with a checkpoint; later frames replace matching
invocations and append metrics. A new run resets retained history. The server owns cursors.

Request bodies are limited to 64 KiB; messages/frames to 4 MiB. Request strings are limited
to 4096 characters and selections to 128 items. Live publication targets 100 ms with
at most four concurrent streams. Disconnects release the stream slot.

The server build type-checks generated codecs and route bindings. Server tests
exercise authentication, request bounds and live publication.
