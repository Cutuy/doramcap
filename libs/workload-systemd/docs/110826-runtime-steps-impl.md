# Systemd runtime steps: implementation

Created: 2026-11-08. Follows the shared [workflow-step design](../../workload-docker/src/workflows/README.md).

## Milestone 1 — Observe runtime resource ownership

**Work items**

- [x] Move native shm setup and mount creation into `systemd.runtime.prepare`.
- [x] Return the existing `Scope` identifiers and retain preparation/setup provenance.
- [x] Route run removal and explicit cleanup through `systemd.runtime.remove`.
- [x] Preserve namespace ownership before setup and existing verified-shutdown rules.

**Validation**

- [x] Run release/locked unit tests and compile native tests and the shm worker.
- [ ] Check successful shm sharing, setup/removal events, payloads and dependencies.
- [ ] Check failed native setup and stop during setup still emit cleanup outcomes.
- [ ] Check abandoned-run cleanup and startup-timeout cleanup.
- [ ] Run focused Clippy, formatting and library-boundary checks.

## Milestone 2 — Document and publish the boundary contract

**Work items**

- [x] Add progressive workflow-step guidance at the workflows root.
- [x] Link the implementation guide and backend documentation to the shared contract.
- [ ] Publish the follow-up PR and record validation on its source revision.

**Validation**

- [ ] Review the changed code against the implementation/design guides.
- [ ] Check relative documentation links and the embedded architecture SVG.
- [ ] Run ARM64 validation at the PR integration point.

Native systemd evidence, GitHub checks and ARM64 worker results are recorded
separately. Full local CI is deferred for this focused observation refactor.
