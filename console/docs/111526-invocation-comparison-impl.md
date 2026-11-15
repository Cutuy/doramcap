# Service invocation comparison implementation

Created 11/15/26. [Architecture](111526-invocation-comparison.md).

`compose.proto` owns `SetupRecord`; `records/setup.ts` reads it and older Compose JSON.

## Milestone 1: Typed comparison evidence

**Work items**
- [x] Persist typed setup and invocation facts atomically; exclude private records from file changes.
- [x] Read protobuf records safely and retain legacy captures.
- [x] Share History/Review comparisons and whole-invocation masks.

**Validation**
- [x] Generated values, image defaults and empty overrides
- [x] Private publication, save failures and incomplete records
- [x] Legacy captures, masking and History rendering
