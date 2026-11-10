# System-app cgroup implementation

Created: 2026-11-09. [Architecture](110926-system-app-cgroup.md).

Applicable rules: D-02/03/04/06/07/09/12/13/14; I-03/04/05/06/07/09. Reuse the approved
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
- [x] Record the owner's acceptance of local native tests for this PR.

**Validation**
- [x] [Cloud CI](https://github.com/Cutuy/doramcap/pull/252/checks): Rust, Node and MCAP pass on x86_64 and ARM64 for code revision `f9b2c265`.
- [x] ARM64 worker 630 passes standard CI and its configured native accounting suite on the same source.
- [x] New native systemd/Compose tests on ARM64 are deferred by the owner; local native coverage is accepted for this PR.

The owner requested: "just do local native tests for now". The new grouping tests
pass natively on x86_64. ARM64 compiles them; their native execution is deferred.
Worker snapshot:
`/home/doramcap-ci/jobs/359faba852e94441ad1adae911c37be1`.

## Milestone 4: Observed cgroup operations

**Work items**
- [x] Use one backend-owned `SystemAppsCgroups` step with protobuf `Create`/`Remove` actions.
- [x] Create after preparation/review; reuse removal for recovery and final cleanup.
- [x] Share ownership and kernel identity across failed or cancelled step invocations.

**Validation**
- [ ] Verify real Compose/systemd events, ordering, dependencies and identity output.
- [ ] Verify cancellation and failed identity persistence retain cleanup ownership.
- [ ] Run focused locked release tests, Clippy, formatting and boundaries; update PR #252.

The owner approved this ordering and requested updates to the existing PR branch.
Focused validation is in progress. Native ARM64 workflow tests remain deferred.
