# System-app cgroup implementation

Created: 2026-11-09. [Architecture](110926-system-app-cgroup.md).

Applicable rules: D-02/03/04/06/07/12/13; I-03/04/05/06/07/09. Reuse the approved
`zbus` and `zbus_systemd` dependencies; keep systemd mechanics in `workload-systemd`.
Compose uses that slice API. Existing app inputs remain the placement authority.

## Milestone 1: Owned accounting boundary

**Work items**
- [ ] Add shared typed cgroup identity and native slice creation/removal.
- [ ] Persist the identity in run artifacts; retain attempted resources on cancellation.
- [ ] Reject foreign or populated slices during removal.

**Validation**
- [ ] Native creation, nested membership, CPU/memory accounting and empty removal.
- [ ] Cancellation, partial creation and ownership failures preserve recoverable resources.

## Milestone 2: Both launchers

**Work items**
- [ ] Place Dora and system apps under the slice using existing app categories.
- [ ] Verify placement before starting user apps.
- [ ] Integrate normal removal and each backend's existing recovery path.

**Validation**
- [ ] Native systemd and Compose startup, nested groups, restart and isolation.
- [ ] Placement failure prevents user startup; stopped runs remove their slices.
- [ ] Focused locked release tests, formatting, Clippy and library boundaries.

## Milestone 3: Publication and qualification

**Work items**
- [ ] Review against the implementation and design guides; publish the focused PR.

**Validation**
- [ ] Cloud CI on the final revision.
- [ ] ARM64 validation, recording native platform checks separately.

Evidence: implementation in progress; native validation and publication pending.
