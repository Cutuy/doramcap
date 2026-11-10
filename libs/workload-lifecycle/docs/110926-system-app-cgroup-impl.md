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
- [ ] Review lifecycle inputs/outputs, ownership and dependency boundaries against both guides.

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
- [ ] Focused locked release tests, all-target Clippy, formatting and library boundaries.
- [ ] Cloud Rust/Node/MCAP checks on x86_64 and ARM64.

Local native checks pass: accounting/ownership cases, both Docker drivers, native
systemd startup/restart/cancellation/recovery and real Supervisor fault scenarios.
The per-run identity check reuses the caller ID across fresh artifact directories.
Focused locked release libraries and boundary checks pass. Logs:
`/tmp/system-app-cgroup-validation`.

Cloud checks pass for the first neutral revision; final-source checks and the guide
review are in progress. Live results: [PR checks](https://github.com/Cutuy/doramcap/pull/252/checks).
The owner requested "just do local native tests for now"; native ARM64 workflow
execution remains deferred. Prior worker 630 results apply to the earlier source.
