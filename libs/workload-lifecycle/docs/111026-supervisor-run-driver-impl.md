# Supervisor run-driver migration

Created: 2026-11-10. [Architecture](111026-supervisor-run-driver.md).

Applicable rules: D-01/02/03/06/07/10/13/14; I-03/05/07/09.
This PR records the plan; implementation and runtime qualification remain pending.

## Milestone 1: Agree on the shared lifecycle contract

**Work items**
- [ ] Map every Supervisor `Phases` hook and result onto `run::Running`.
- [ ] Agree on the smallest typed boundary for Supervisor's launch/forced-shutdown recording guard and readiness-send errors.

**Validation**
- [ ] Review the mapping against the architecture's behavior table and existing callers.
- [ ] Confirm recording failure can coexist with stopped writers and successful cleanup.

## Milestone 2: Migrate Supervisor and delete the legacy driver

**Work items**
- [ ] Implement `Running` for Supervisor while retaining its backend steps, pidfds, ownership ledger and public report.
- [ ] Route runs through `run::run`; keep readiness errors observable and cgroup operations ordered around launch and removal.
- [ ] Migrate lifecycle coverage, then remove `recorded_run`, `Phases` and their obsolete references.

**Validation**
- [ ] Check normal stop, startup failure, cancellation before/during launch, a dropped readiness receiver and early process exit.
- [ ] Verify forced shutdown suppresses recording success while allowing independently verified cleanup.
- [ ] Verify dropped futures, descendants and abandoned-run admission against native Supervisor tests.
- [ ] Run focused locked release tests for lifecycle, Supervisor, Compose and systemd; check formatting, Clippy and dependency boundaries.
- [ ] Qualify native cgroup and Supervisor cleanup, and record x86/ARM64 results separately under the repository's integration policy.

Current evidence: source tracing confirms Supervisor is the sole production
`Phases` caller. The existing shared-driver and Supervisor native tests cover
startup cancellation and independent outcomes; migration qualification is pending.
