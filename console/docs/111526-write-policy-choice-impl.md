# Choose the write policy at Review: implementation

Created November 15, 2026. [Architecture](111526-write-policy-choice.md).

## Code changes

| Area | Change |
| --- | --- |
| [console.proto](../proto/console.proto) | `PrepareRequest` gains `doramcap.filesystem.v1.WritePolicy write_policy = 5`; `WRITE_POLICY_UNSPECIFIED` means the project default |
| [release template](../release/console.pbtxt) | Adds `write_policy: WRITE_POLICY_KEEP` |
| [config.ts](../server/src/config.ts) | Unchanged: an absent policy still loads as write back |
| [service.ts](../server/src/service.ts) | Prepare resolves the request's policy, else the config's; `Review.write_policy` carries it to Start and the saved marker |
| [run-plan.ts](../server/src/run-plan.ts) | Takes the prepared review's policy in place of `config.writePolicy` |
| [compare.ts](../server/src/compare.ts) | Adds a `write_policy` setup fact |
| History **Run again** | Sends the earlier run's policy with its profiles and system apps |
| [WritableFolders.tsx](../frontend/src/WritableFolders.tsx) | Review renders a three-option radio group with one result line each; Result and History keep the read-only label |
| [App.tsx](../frontend/src/App.tsx) | Holds the chosen policy like system apps and prepares again on change |
| [configuration.md](configuration.md) | Documents `write_policy`, its three values and the Review choice |

No new environment input. The enum and its effect stay with `host-filesystem`.

## Milestone 1: per-run policy

**Work items**
- [ ] Add `PrepareRequest.write_policy` and resolve it in Prepare.
- [ ] Drive the run plan from the prepared review.
- [ ] Add the Compare fact.
- [ ] Carry the policy through **Run again**.

**Validation**
- [ ] Prepare resolves request, then config
- [ ] Each policy reaches the run plan
- [ ] Compare shows a policy change
- [ ] Run again keeps the policy

## Milestone 2: Review choice and default

**Work items**
- [ ] Review radio group, hidden without overlaid folders.
- [ ] Release template defaults to keep.
- [ ] Document the setting.

**Validation**
- [ ] Review renders and changes the choice
- [ ] Old config still writes back
- [ ] New project keeps files
- [ ] Packaged Compose run on ARM64 VM, each policy
