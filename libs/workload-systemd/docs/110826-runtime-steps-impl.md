# Systemd runtime steps: implementation

Created: 2026-11-08. Follows the shared [workflow-step design](../../workload-docker/src/workflows/README.md).

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

- [x] Add progressive workflow-step guidance at the workflows root.
- [x] Link the implementation guide and backend documentation to the shared contract.
- [x] Publish the follow-up PR and record validation on its source revision.

**Validation**

- [x] Review the changed code against the implementation/design guides.
- [x] Check relative documentation links and the embedded architecture SVG.
- [x] Run ARM64 validation at the PR integration point.

This replacement for PR #246 extracts existing implementation into its owning
steps and deletes the shm module and forwarding cleanup methods.
Source `8086eefc`: local x86_64 release/locked checks pass: 22 unit tests, six native
scenarios, all-target Clippy, boundaries, formatting and documentation links.
Native coverage includes live sharing/lineage, setup rejection, stop during setup,
abandoned-run cleanup, startup timeout and native environment round-trip. Builds
ran unprivileged; only native test executables were elevated.
ARM64 worker job **625** passed its default/all-feature Rust and native suite.
GitHub Rust CI (ARM64/x86_64) and Node CI passed for source `8086eefc`.

Native systemd evidence, GitHub checks and ARM64 worker results are recorded
separately. Full local CI is deferred for this focused observation refactor.
