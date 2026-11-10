# Run-only ARM64 CI

![ARM64 source, build and execution flow](110826-run-only-worker.svg)

Created: 2026-11-08.

The submitter snapshots source, queues native ARM64 builds on WSL `.20`, then
streams checksummed nextest archives and source to Mac mini Linux `.22`.
The Mac runs tests serially; a failed build stops transfer.

Matching paths under `/tmp/doramcap-ci-run-only` preserve embedded fixtures;
worker locks protect caches. The builder supplies nextest and Docker/Compose CLI
tools. Privileged BPF, doctests, Node and CUDA checks use existing workers.

See the [README](../README.md#run-only-arm64-worker) for usage and requirements.

## Milestone 1: build and run

**Work items**

- [x] Queue source snapshots and nextest archives through WSL Pueue.
- [x] Stream archives and user-level tools with SCP `-3`; serialize Mac execution.
- [x] Retain source, checksums, logs and per-suite exit status in `~/jobs`.

**Validation**

- [x] Mac: 453 default, 459 all-feature and 65 storage tests without default features passed; 199 existing skips, zero OOM kills.
- [x] 20 CI-tool tests, TypeScript, shell, boundaries and six hosted checks passed.
- [x] Failed-build gating, corrupt archives and stale-result reset checked.

Evidence: code `18f26316`, Pueue 623, job `c6a3e8469dc74a5a92e74d872ebb0bd7` on both hosts.
