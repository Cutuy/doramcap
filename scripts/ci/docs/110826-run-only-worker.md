# Run-only ARM64 CI

![ARM64 source, build and execution flow](110826-run-only-worker.svg)

Created: 2026-11-08. [Implementation](110826-run-only-worker-impl.md).

The submitter snapshots source, queues native ARM64 builds on WSL `.20`, then
streams checksummed nextest archives and source to Mac mini Linux `.22`.
The Mac runs tests serially; a failed build stops transfer.

Matching paths under `/tmp/doramcap-ci-run-only` preserve embedded fixtures;
worker locks protect caches. The builder supplies nextest and Docker/Compose CLI
tools. Privileged BPF, doctests, Node and CUDA checks use existing workers.

See the [README](../README.md#run-only-arm64-worker) for usage and requirements.
