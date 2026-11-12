# Supervisor run-driver migration

Created: 2026-11-10. [Architecture](111026-supervisor-run-driver.md).

Applicable rules: D-01/02/03/06/07/10/13/14; I-03/05/07/09.
The user approved report-owned recording policy on 2026-11-12. The migration adds no recording hook.

## Milestone 1: Agree on the shared lifecycle contract

**Work items**
- [x] Map every Supervisor `Phases` hook and result onto `run::Running`.
- [x] Keep launch/forced-shutdown policy in Supervisor’s report; make shared readiness errors observable.

**Validation**
- [x] Lifecycle and caller mapping.
- [ ] Independent recording and cleanup outcomes.

### Lifecycle mapping

| Supervisor hook/result | Shared sequence | Contract to retain |
| --- | --- | --- |
| `launch` | `Running::start` | Prepare, create accounting cgroup, launch, await recording readiness, register pidfds and inspect liveness. Retain attempted resources before awaiting. |
| `ready` | `Running::ready` | Return readiness-send errors to the shared driver; teardown follows. Other backends retain their behavior. |
| `inspect`, `wait_for_change` | `Running::ended` | Inspect before waiting; either pidfd becoming readable ends the recording attempt. |
| `quiesce` | `Running::stop` | Keep stopped writers separate from forced shutdown. Retain quiescence diagnostics and the observed step output. |
| `finalize` | Shared recording check | Check files after writers stop; Supervisor’s report rejects incomplete launch or forced shutdown. The common check has the same input contract as Compose and systemd. |
| `remove` | `Running::remove` | Remove only owned, empty cgroups after daemon reaping; save verified cleanup in the ownership ledger. |
| `finish` | Handle lifetime | Remove the former no-op hook; executor factories retain runtime ownership through teardown and report persistence. |
| `Outcome`, `RunReport` | Shared outcome plus backend evidence | Preserve execution error, stopped writers, recording validity, cleanup verification and diagnostics independently. |

Compose and systemd currently ignore readiness-send failures. Their behavior stays
unchanged when the shared readiness result becomes fallible for Supervisor.
The public `MonitorRun`, `Ownership` and `RunReport` formats stay unchanged.

## Milestone 2: Migrate Supervisor and delete the legacy driver

**Work items**
- [x] Implement `Running` for Supervisor while retaining its backend steps, pidfds, ownership ledger and public report.
- [x] Route runs through `run::run`; keep readiness errors observable and cgroup operations ordered around launch and removal.
- [x] Migrate lifecycle coverage, then remove `recorded_run`, `Phases` and their obsolete references.

**Validation**
- [ ] Startup, readiness, stop and process exit.
- [ ] Forced shutdown with a readable MCAP.
- [ ] Dropped futures, descendants and admission.
- [ ] Focused lifecycle, Supervisor, Compose and systemd checks.
- [ ] Formatting, Clippy and library boundaries.
- [ ] Native cgroup and Supervisor cleanup.
- [ ] ARM64 integration qualification.
