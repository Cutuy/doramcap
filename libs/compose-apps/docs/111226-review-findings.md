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
2. **Rejections become typed errors.** A user-fixable rejection is a named
   `compose::Error` variant with its service or resource. Console shows its own
   message for each one.

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

The Review checks below return named variants. The `#[error]` text stays for logs
and the diagnostics file. Other validation errors use the generic fallback.

| Variant | Case |
| --- | --- |
| `EmptySelection` | No service is selected. |
| `NoComposeFile` | The project names no Compose file. |
| `UnresolvedEnvironment` | Compose could not resolve an environment value. |
| `ImageNotPrepared { service }` | A service needs a named local image. |
| `AnonymousVolume { service }` | A volume has no retention settings. |
| `ImageVolume { service, target }` | An image-declared volume needs an explicit mount. |
| `ZeroReplicas { service }` | Replicas are zero; deselect through profiles. |
| `NameOccupied { name }` | A fixed container name is already in use. |
| `ReservedLabel { label }` | The project sets a label Console owns. |
| `UnscopedResource { resource }` | A named network or volume is not declared external. |
| `ForeignResource { resource }` | An existing project resource belongs to another run or controller. |
| `NoJournal { services }` | The host lacks the journald driver or a reachable journal. |

The addon already turns `EmptySelection` into a code. It does the same for every
variant: a reason and its subject. Console keeps HTTP status `InvalidInput` and
adds the reason; the frontend picks the message for that reason.

The addon carries designed rejections in its protobuf Review responses, including
profile listing. TypeScript raises an error with the reason and subject while
preserving the backend detail for diagnostics. Engine and I/O faults keep the
existing error path.

## What stays the same

- Internal `diagnostics` stay in the plan as free text.
- Engine, I/O, JSON, timeout and output-limit errors keep today's path.
- Failures after Review keep their codes and raw detail.
- Policy changes stay in the resolution record.

Applicable rules: D-03 (core contracts independent of UI), D-07 (facts separate
from interpretation), D-08 (saved plans), D-09 (one resolution for Review and
Start).
