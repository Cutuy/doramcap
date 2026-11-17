# Choose the write policy at Review: implementation

Created November 15, 2026. [Architecture](111526-write-policy-choice.md).

## Code changes

| Area | Change |
| --- | --- |
| [console.proto](../proto/console.proto) | `PrepareRequest` gains `doramcap.filesystem.v1.WritePolicy write_policy = 5`; `WRITE_POLICY_UNSPECIFIED` means the project default |
| [release template](../release/console.pbtxt) | Adds `write_policy: WRITE_POLICY_KEEP` |
| [config.ts](../server/src/deployment/config.ts) | An absent policy loads as write back |
| [service.ts](../server/src/service.ts) | Prepare resolves the request's policy, else the config's; `Review.write_policy` carries it to Start and the saved marker |
| [run-plan.ts](../server/src/run/run-plan.ts) | Takes the prepared review's policy in place of `config.writePolicy` |
| [run module](../server/src/analysis/modules/run/module.ts) | Adds a `write_policy` setup fact |
| History **Run again** | Sends the earlier run's policy with its profiles and system apps |
| [WritableFolders.tsx](../frontend/src/WritableFolders.tsx) | Review renders a three-option radio group with one result line each; Result and History keep the read-only label |
| [App.tsx](../frontend/src/App.tsx) | Holds the chosen policy like system apps and prepares again on change |
| [configuration.md](configuration.md) | Documents `write_policy`, its three values and the Review choice |

Builds on [system-app preferences](111526-system-app-preferences.md): policy changes
keep accepted app choices, and Run again uses its saved apps and policy. Older runs
use configured app defaults and write back. Saved Status owns the historical policy.

No new environment input. The enum and its effect stay with `host-filesystem`.

## Milestone 1: per-run policy

**Work items**
- [x] Add `PrepareRequest.write_policy` and resolve it in Prepare.
- [x] Drive the run plan from the prepared review.
- [x] Add the Compare fact.
- [x] Carry the policy through **Run again**.

**Validation**
- [x] Prepare resolves request, then config
- [x] Each policy reaches the run plan
- [x] Compare shows a policy change
- [x] Run again keeps the policy

## Milestone 2: Review choice and default

**Work items**
- [x] Review radio group, hidden without overlaid folders.
- [x] Release template defaults to keep.
- [x] Document the setting.

**Validation**
- [x] Review renders and changes the choice
- [x] Old config still writes back
- [x] New project defaults to keep
- [ ] Packaged Compose run on ARM64 VM, each policy
