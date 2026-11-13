![Review warnings and rejections](111226-review-findings.svg)

# Review warnings and rejections

Created November 12, 2026. Status: implemented.
[Implementation plan](111226-review-findings-impl.md).

Compose resolution gives the user two kinds of result at Review. A warning lets
the run go ahead; a rejection stops it. Both come from designed checks, and each
tells the user what to change. Failures after Review are unexpected and keep
their raw evidence.

This design makes two changes:

1. **Warnings move out of the frozen plan.** The plan is launch identity. Review
   warnings belong in `Inspection`, the message that answers Review.
2. **Rejections carry their failed check.** A rejected input is one
   `compose::Error::Invalid` with a message and the source line of the check
   that failed. Console shows the message and the check.

## Previous behavior

![Review messages today](111226-review-findings-today.svg)

- `ResolvedComposeProject` carries `warnings` (project logging, from #314) beside
  internal `diagnostics`. Text meant for the user is saved with every plan.
- Other notes stay as free text in `diagnostics`. Review does not show them.
- Rejections are `Error::Invalid(String)`. Console maps them to `InvalidInput`
  and shows a generic message followed by the backend's English text.
  `EmptySelection` is the only rejection with its own variant.

## Warnings

| Message | Holds |
| --- | --- |
| `ResolvedComposeProject` | Launch identity and internal `diagnostics` |
| `Inspection` | Plan, snapshot, machine state and Review `warnings` |

Resolution returns warnings beside the plan. The addon puts them in
`Inspection`, and Console lists them at Review. Saved plans no longer carry
user-facing text.

## Rejections

A rejection is a structured error:

| Field | Meaning |
| --- | --- |
| `operation` | Coarse step: `Review` or `Profiles`. |
| `kind` | `validation`. |
| `message` | Backend text naming the service, mount or resource; may be empty. |
| `check` | Source line of the failed check, recorded by `invalid()`. |

These fields are the existing `compose.v1.Diagnostic` plus `check`. The addon
returns a rejection as a `Diagnostic` in the Inspection or profile-listing
response. Console keeps HTTP status `InvalidInput`, shows its generic message,
the backend message and the check. Engine and I/O faults keep the error path.
`EmptySelection` keeps its own variant because Review offers profiles for it.

## What stays the same

- Internal `diagnostics` stay in the plan as free text.
- Engine, I/O, JSON, timeout and output-limit errors keep today's path.
- Failures after Review keep their codes and raw detail.
- Policy changes stay in the resolution record.

Applicable rules: D-02 (one rejection shape, no per-cause table), D-03 (core
contracts independent of UI), D-07 (facts separate from interpretation), D-08
(saved plans), D-09 (one resolution for Review and Start).
