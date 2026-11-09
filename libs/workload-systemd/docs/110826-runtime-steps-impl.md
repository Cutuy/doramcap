# Systemd runtime steps: implementation

Created: 2026-11-08. Follows the shared [workflow-step design](../../orchestrator/docs/102626-function-steps.md#choosing-workflow-steps).

## Milestone 1 — Observe runtime resource ownership

**Work items**

- [x] Move native shm setup and mount creation into `systemd.runtime.prepare`; remove `shm.rs`.
- [x] Return the existing `Scope` identifiers and retain preparation/setup provenance.
- [x] Move verified runtime teardown into `systemd.runtime.remove`; remove Manager and Running forwarding methods.
- [x] Preserve namespace ownership before setup and existing verified-shutdown rules.

**Validation**

- [x] Run release/locked unit tests and compile native tests and the shm worker.
- [x] Check successful shm sharing, setup/removal events, payloads and dependencies.
- [x] Check failed native setup and stop during setup still emit cleanup outcomes.
- [x] Check abandoned-run cleanup and startup-timeout cleanup.
- [x] Run focused Clippy, formatting and library-boundary checks.

## Milestone 2 — Document and publish the boundary contract

**Work items**

- [x] Extend the existing orchestrator design with step-boundary guidance.
- [x] Link the design guide and backend documentation to the shared contract.
- [x] Publish the follow-up PR and record validation on its source revision.

**Validation**

- [x] Review the changed code against the implementation/design guides.
- [x] Check relative documentation links and the embedded architecture SVG.
- [x] Run ARM64 validation at the PR integration point.
