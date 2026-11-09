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

Runtime source: `16707045`, published in [PR #246](https://github.com/Cutuy/doramcap/pull/246).
Local x86_64: 22 unit tests passed (1 ignored), all-target release Clippy,
boundaries, formatting and documentation links passed. Five real systemd scenarios
passed: live shm exchange with step lineage, native setup rejection, stop during
runtime setup, abandoned-run cleanup and startup-timeout cleanup. Native tests and
the shm worker were built unprivileged; only test execution was elevated.

ARM64 worker job **624** passed against `16707045`, covering the worker's default
and all-feature Rust tests and native validation suite. The five systemd scenarios
above ran locally. GitHub Node CI passed; GitHub Rust CI was still running when
this record was written. The final follow-up commit changes this validation record only.

Native systemd evidence, GitHub checks and ARM64 worker results are recorded
separately. Full local CI is deferred for this focused observation refactor.
