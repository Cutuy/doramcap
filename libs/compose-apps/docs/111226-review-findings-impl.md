# Review warnings and rejections: implementation

Created November 12, 2026. [Architecture](111226-review-findings.md).

## Code changes

| Area | Change |
| --- | --- |
| `libs/compose-apps/proto/compose.proto` | Remove `warnings` from `ResolvedComposeProject`; reserve field 9 and its name. |
| `console/proto/execution.proto` | Add `repeated string warnings` to `Inspection`. |
| `compose-apps` configuration | Collect warnings beside the plan; `preparation::resolve` returns them on `ResolvedRun`. |
| `compose-apps` error | Add the named rejection variants; replace the matching `invalid(...)` sites. Add `Error::reason()` returning the variant name and subject. |
| Console addon (`console/server/native`) | Put warnings in `Inspection`. Pass any rejection's reason and subject; generalize the `EmptySelection` match. |
| Console server | Read warnings from `Inspection`. Keep `InvalidInput` and attach the reason and subject to the error body. |
| Console frontend | One message per reason; fall back to today's generic message and detail. |

Saved JSON plans with `warnings` read without it. Warnings are shown only at
Review, so no saved value is needed.

## Milestone 1 — Warnings out of the plan

**Work items**

- [ ] Reserve plan field 9 and add `Inspection.warnings`
- [ ] Return warnings from resolution beside the plan
- [ ] Fill and read `Inspection.warnings` in the addon and Console

**Validation**

- [ ] Compose warning tests
- [ ] Console warning passthrough test
- [ ] Older saved plan decode

## Milestone 2 — Typed rejections

**Work items**

- [ ] Add rejection variants and `Error::reason()`
- [ ] Replace user-fixable `invalid(...)` sites
- [ ] Pass reason and subject through the addon and Console error body
- [ ] Frontend message table with fallback

**Validation**

- [ ] Compose rejection tests by variant
- [ ] Console error body test
- [ ] Frontend message selection test
- [ ] Native journal rejection regression
