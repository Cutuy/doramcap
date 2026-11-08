![Shared launch contracts](110726-launch-contracts.svg)

# Shared run fields

Created: 2026-11-07. Implements [PR #207](https://github.com/Cutuy/doramcap/pull/207)
from remote main at #201, with the owner's fixed-phase decision.

Rust starts Dora, then system apps and waits for recordings, then user apps.
Shutdown reverses those phases. The request declares app roles; configurable groups
and per-group recording gates are removed.

| Shared `RunPlan` | Backend launch inputs |
| --- | --- |
| Identity, paths, recordings and budgets | Compose projects keyed by app ID |
| System-app IDs and MCAP-writer IDs | systemd programs, unit files and app binds |
| Dora graph, ports, runtime directory, memlock and participant | Compose image and shm capacity; systemd executable, launch user and binaries |

Console builds shared fields once. Rust rejects duplicate or unknown app roles.
Compose resolves writer app IDs to every selected service in each project.
The in-memory request has no wire-compatibility requirement; field numbers may change.

## Milestone 1: Shared contracts and fixed startup

**Work items**
- [x] Share app roles, recording writers and Dora settings.
- [x] Flatten backend plans; delete `UnitGroup` and its recording-gate flag.
- [x] Enforce fixed phases in Rust and share Console's request builder.
- [x] Remove obsolete imports, reserved fields and outdated diagrams.

**Validation**
- [x] Native x86_64 systemd tests cover ordering, recording readiness, Dora and cleanup.
- [x] Real Docker mapping covers two differently named writer services; real Console Compose runs pass.
- [ ] Regenerate and recheck the renumbered contracts on the final revision.
- [ ] Full local and cloud CI; ARM64 worker validation.
- [ ] Real Console systemd execution: deferred until #204, absent from this base.

Latest evidence: [PR #211](https://github.com/Cutuy/doramcap/pull/211).
Pre-renumbering focused checks and native tests pass; final contract checks are in progress.

## Schema ownership

`launch.proto` defines shared inputs. `compose_plan.proto` and `systemd_plan.proto`
remain active backend contracts. `dora.proto` still defines Compose's resolved runtime
and machine configuration; `DoraUnits` now contains only systemd launch settings.

#204–#206 and #207 remain unchanged. Systemd retains its launch-user setting;
numeric Dora participant IDs remain Compose-only.
