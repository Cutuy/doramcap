# System-app cgroup implementation

Created: 2026-11-09. [Architecture](110926-system-app-cgroup.md).

Applicable rules: D-02/03/04/06/07/12/13; I-03/04/05/06/07/09. Reuse the approved
`zbus` and `zbus_systemd` dependencies; keep systemd mechanics in `workload-systemd`.
Compose uses that slice API. Existing app inputs remain the placement authority.

## Milestone 1: Owned accounting boundary

**Work items**
- [x] Add shared typed cgroup identity and native slice creation/removal.
- [x] Persist the identity in run artifacts; retain attempted resources on cancellation.
- [x] Reject foreign or populated slices during removal.

**Validation**
- [x] Native creation, nested membership, CPU/memory accounting and empty removal.
- [x] Cancellation, partial creation and ownership failures preserve recoverable resources.

Evidence: local native cgroup tests pass, including cancellation after the parent
appears, foreign/live preservation, recovery and immediate ownership-name reuse.

## Milestone 2: Both launchers

**Work items**
- [x] Place Dora and system apps under the slice using existing app categories.
- [x] Verify placement before starting user apps.
- [x] Integrate normal removal and each backend's existing recovery path.

**Validation**
- [x] Native systemd and Compose startup, nested groups, restart and isolation.
- [x] Placement failure prevents user startup; stopped runs remove their slices.
- [x] Focused locked release tests, formatting, Clippy and library boundaries.

Evidence: local x86_64/Linux 7.0/systemd 255 checks pass. Real Docker tests cover
membership, controller loss, restart and deliberate misplacement before user startup.
Native systemd startup, restart, concurrent repositories and cancellation pass.
The Console stale-review regression passes. Local logs: `/tmp/system-app-cgroup-validation`.

## Milestone 3: Publication and qualification

**Work items**
- [x] Review against the implementation and design guides; publish [PR #252](https://github.com/Cutuy/doramcap/pull/252).
- [ ] Resolve the remaining native ARM64 qualification choice, then mark ready and release the lease.

**Validation**
- [x] [Cloud CI](https://github.com/Cutuy/doramcap/pull/252/checks): Rust, Node and MCAP pass on x86_64 and ARM64 for code revision `f9b2c265`.
- [x] ARM64 worker 630 passes standard CI and its configured native accounting suite on the same source.
- [ ] New native systemd/Compose tests on ARM64: worker sudo permits only the installed standard test launcher; owner scope decision pending.

The new grouping tests pass natively on x86_64. ARM64 compiles them but its current
privileged runner does not execute them. Worker snapshot:
`/home/doramcap-ci/jobs/359faba852e94441ad1adae911c37be1`.
