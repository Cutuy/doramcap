# Service names in Top CPU consumers: implementation

Created: 2026-11-13. [Architecture](111326-service-names-in-cpu-consumers.md).

Applicable rules: D-02/03/04/07/13/15; I-04/07/09.

`streams.ts` exports `cgroupName(path)`. `CpuStreams` stores its result per cgroup ID in
place of the last path segment. It checks, in order:

1. The leaf or its parent is a Compose service slice: unescape `\xHH` in the service part.
2. Four segments match the cgroupfs run tree, followed by the container: the service segment.
3. The leaf is a native unit `doramcap-‹repo›-‹id›.service`: `‹id›`.
4. Otherwise the last segment, as today.

## Milestone 1: show names

**Work items**
- [x] Add `cgroupName` with named patterns, and use it in `CpuStreams`.
- [x] Check Rust-generated per-app paths against `console/frontend/src/fixtures/app-cgroup-paths.json`.
- [x] Publish systemd-driver per-app paths from visitor runs of the demo. The built-in run's
  published recording keeps its earlier `docker-<id>.scope` paths.

**Validation**
- [x] Shared path fixture: Rust naming matches it; the frontend names its apps.
- [x] Frontend tests: host cgroups, distinct rows per ID.
- [x] Demo walk in light and dark.
