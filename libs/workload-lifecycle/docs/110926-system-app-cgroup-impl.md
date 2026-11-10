# System-app cgroup implementation

Created: 2026-11-09. [Architecture](110926-system-app-cgroup.md).

Applicable rules: D-02/03/04/06/07/09/12/13/14; I-03/04/05/06/07/09.
The owner approved a shared step with backend mechanics and retention of empty
runtime-owned parents. Existing filesystem, Docker and native systemd APIs provide
the mechanics; no new third-party package is introduced.

## Milestone 1: Shared accounting lifecycle

**Work items**
- [x] Move `SystemAppsCgroups`, its protobuf input and kernel verification into `workload-lifecycle`.
- [x] Keep path/ID identity independent of systemd names and APIs.
- [x] Preserve attempted ownership across failure/cancellation and report empty retained parents explicitly.

**Validation**
- [x] Verify accounting, PID membership, failed identity writes, cancellation and foreign/live preservation.
- [x] Review lifecycle inputs/outputs, ownership and dependency boundaries against both guides.

## Milestone 2: Native backend mechanics

**Work items**
- [x] Remove Compose's dependency on `workload-systemd`; support both Docker cgroup drivers.
- [x] Use distinct per-run Docker parents and normalize generated placement during review comparison.
- [x] Keep native slice allocation in systemd and filesystem allocation/removal in Supervisor.

**Validation**
- [x] Run the same Compose membership, restart, recovery, shared-memory and misplacement cases against both drivers.
- [x] Verify native systemd startup, cancellation, restart, isolation and recovery.
- [x] Verify Supervisor failure/cancellation and directly owned filesystem cleanup.
- [x] Verify generated parent changes preserve review identity and native user placement changes invalidate it.

## Milestone 3: Publication and qualification

**Work items**
- [x] Update [PR #252](https://github.com/Cutuy/doramcap/pull/252) and the local guide review record.
- [x] Retain the owner's acceptance of local native workflow qualification for this PR.

**Validation**
- [x] Focused locked release tests, all-target Clippy, formatting and library boundaries.
- [x] Cloud Rust/Node/MCAP checks on x86_64 and ARM64 for code revision `3fd9abd5`.

## Milestone 4: Review clarifications

**Work items**
- [x] Document native parent formats, absolute host paths, kernel IDs and step inputs; remove obsolete reservations and prefix action names.
- [x] Write `system-app-cgroup.pb` directly from the generated message; remove lifecycle reflection and descriptor output. Generated binaries remain outside source control.
- [x] Match Dora/System roles explicitly; audit cgroup sequencing across all three launchers.
- [x] Keep Supervisor driver consolidation in [planning PR #262](https://github.com/Cutuy/doramcap/pull/262), as requested.

**Validation**
- [x] Focused locked release library tests, formatting and dependency boundaries.
- [x] Native binary-artifact accounting/restart/recovery and Supervisor cleanup checks.
- [x] Focused all-target release Clippy.
- [ ] Current cloud checks.

Latest review evidence for code revision `4a1296f4`:
- Focused locked release libraries, all-target Clippy, formatting and boundaries pass.
- Local native accounting/ownership, both Docker drivers, systemd restart and Supervisor failure/cleanup checks pass with binary identity artifacts. Logs: `/tmp/system-app-cgroup-validation`.
- [Current cloud checks](https://github.com/Cutuy/doramcap/pull/252/checks) are pending. The owner requested local native qualification; native ARM64 workflow execution remains deferred.

Supervisor's legacy driver remains in use; its forced-shutdown, readiness and
ownership behavior is preserved for the separate migration plan.
