# Review warnings and rejections: implementation

Created November 12, 2026. [Architecture](111226-review-findings.md).

Both milestones are implemented.

## Code changes

| Area | Change |
| --- | --- |
| `libs/compose-apps/proto/compose.proto` | Remove `warnings` from `ResolvedComposeProject`; add `Diagnostic.check`. |
| `console/proto/execution.proto` | Add `Inspection.warnings` and a `Diagnostic` rejection on Inspection and profile-listing responses. |
| `console/proto/console.proto` | Add `ConsoleError.check`. |
| `compose-apps` configuration | Collect warnings beside the plan; `preparation::resolve` returns them on `ResolvedRun`. |
| `compose-apps` error | `Error::Invalid { message, check }`; `invalid()` and the `From` conversions use `#[track_caller]`. |
| Console addon (`console/server/native`) | Put warnings in `Inspection`; return `Invalid` as a rejection `Diagnostic`. |
| Console server and frontend | Read warnings from `Inspection`; pass `check` through the error body and show it. |

Saved JSON and protobuf plans with `warnings` still decode; the field is ignored.

## Milestone 1 — Warnings out of the plan

**Work items**

- [x] Remove plan field 9 and add `Inspection.warnings`
- [x] Return warnings from resolution beside the plan
- [x] Fill and read `Inspection.warnings` in the addon and Console

**Validation**

- [x] Compose warning tests
- [x] Console warning passthrough test
- [x] Older saved plan decode

## Milestone 2 — Located rejections

**Work items**

- [x] Record the check location in `Error::Invalid`
- [x] Return rejections as `Diagnostic` from the addon
- [x] Pass `check` through the Console error body and show it

**Validation**

- [x] Compose journal rejection check location
- [x] Console error body test
- [x] Native rejection regressions
