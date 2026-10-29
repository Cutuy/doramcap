![Console responsibilities](102926-console-boundary.svg)

# doramcap Console MVP

Created: 2026-10-29. Status: revised implementation plan; implementation pending.

**Console** lets users adopt an existing Compose project, review its settings,
run it with observation, and inspect saved evidence. Source and Compose files
remain unchanged. System apps supply monitoring; workflows supply execution.

## Deployment and scope

One on-prem Node.js server runs alongside the repository, Docker and artifacts.
It serves the frontend and API on a configured address/port; a browser opens that
URL over the local network. The host can be headless. Deployment includes the
native Rust addon and qualified monitoring assets.

| Location | Responsibility |
|---|---|
| `console/frontend/` | TypeScript, React, Vite; screens and interaction state |
| `console/server/` | TypeScript, Node.js; HTTP, product state, history, artifact delivery |
| `console/server/native/` | Rust N-API adapter over existing execution libraries |
| `console/docs/` | Plans and supporting diagrams |

MVP supports one host, one selected project and one active experiment. Require
prebuilt local images and qualified CPU-monitor prerequisites. Validate a small
real Compose fixture first; vEDGAR is the next compatibility pilot.

Defer A/B, queues, builds, recursive browsing, live CPU charts, video/robotics
viewers, host tuning, remote-device execution and automatic crash recovery.

## Four screens

| Screen | Minimum behavior |
|---|---|
| Adopt | Specify a server-side repository path. Suggest top-level Compose files; select ordered files, env files, profiles and services. |
| Review | Show resolved services, images, mounts, effective overrides and prerequisite failures. Highlight capture gaps. |
| Run | Show steps, container status and bounded diagnostics. Stop and reconnect by run ID. |
| Inspect | Show execution, evidence and cleanup outcomes separately; select saved runs and download CPU MCAP and reports. |

## TypeScript / Rust boundary

![Native library interop](102926-console-interop.svg)

The Node process loads a native addon through **N-API**. Use existing Rust library
APIs behind a small adapter. Proposed binding: [napi-rs](https://napi.rs/);
evaluate and pin it during implementation. No dependency has been added by this plan.

| Owner | Responsibilities |
|---|---|
| TypeScript | HTTP validation/access, selections, review records, idempotency, run admission, history and downloads |
| Rust | Resolve and revalidate inputs, execute Compose, capture evidence, stop/finalize, verify cleanup and produce the authoritative report |
| Native adapter | Owned input/output conversion, async completion, bounded event transfer and explicit lifetime management |

Rust retains the execution lifecycle and its admission safety checks. TypeScript
persists the product view of that lifecycle; it cannot mark cleanup successful
or override a blocked runner. Artifacts stay on disk and cross the boundary as
registered references. MCAP bytes are streamed by the server on download.

### Async contract

Names below describe the intended API; finalize signatures beside the implementation.

| Call | Result / guarantee |
|---|---|
| `prepare(selection)` | Promise of a redacted review and opaque prepared-input identity |
| `start(preparedId, runId)` | Promise of a run handle after revalidation and admission; readiness arrives separately |
| `run.readEvents(after, limit)` | Promise of a bounded batch, next cursor and any gap indicator |
| `run.stop()` | Idempotent stop acceptance; usable during startup |
| `run.completed` | Final report after evidence finalization and cleanup attempt, including failures |
| `close()` | Stop admission, request Stop, await finalization, release native resources |

Run native work on a dedicated Rust thread with its own async runtime/local task
context. Construct the existing `Rc`-based runner there. Transfer owned records
through bounded queues; keep JavaScript values on the Node thread. Native work
must leave HTTP and the Node event loop responsive. Start retains the run in a
server-owned registry until completion; JavaScript garbage collection is never
the shutdown mechanism.

Persist activity with monotonically increasing cursors. Bound queue depth, batch
size and diagnostic retention; coalesce replaceable status updates. Surface lost
activity explicitly. Terminal state and the final report remain independently
readable. Slow or disconnected browsers must not stall capture or cleanup.

See [Node N-API](https://nodejs.org/api/n-api.html) and the
[thread-safe bridge rules](https://napi.rs/docs/concepts/threadsafe-function).

## HTTP and durable state

- Typed HTTP: prepare/review, Start, Stop, run status/events, history and downloads.
  Poll active status/events every second; SSE is deferred. Return structured errors.
- Validate inputs at both HTTP and native boundaries. Prefer generated protobuf
  types for records and pbtxt for settings; validate the addon/server schema version.
- Store preparation records, run metadata, activity and reports outside the repo.
  Use atomic metadata updates and an index rebuilt from run directories; no database.
- Persist the run ID and Start idempotency key before launching. Retry returns the
  same run. Reject overlapping starts; stale review requires a fresh review.
- Rust rechecks review identity before launch. Record hashes of reviewed Compose
  inputs and effective settings, available Git revision/dirty state, image IDs,
  mounts and basic host/runtime identity. Mutable mounts, dirty contents and runtime
  drift are declared evidence gaps unless captured; MVP does not promise equivalence.
- Configure single-user access at startup. Protect mutations against cross-origin
  requests. Restrict repository reads to approved roots and downloads to registered
  artifacts; reject traversal/symlink escapes. Redact secrets in UI/events and keep
  raw secret-bearing manifests private. Document trusted-network deployment.

### Stop, shutdown and interruption

Browser disconnect leaves the experiment running. Stop requests cooperative
finalization and stays visible until the report arrives. A failed workload may
still have valid evidence; report execution, evidence and cleanup independently.

Server shutdown stops admission, requests Stop and awaits finalization within a
documented deadline. Persist an interrupted/cleanup-unknown outcome if orderly
completion cannot be established. A native fatal failure shares the Node process
crash boundary; cleanup cannot be guaranteed after process loss.

At startup, reconcile unfinished records against owned Docker resources and
persisted reports. Keep new runs blocked while cleanup is unknown. Provide a
clear manual reconciliation procedure; do not automatically delete resources or
resume workloads. Disk exhaustion and persistence errors remain visible failures.

## M1 — Adopt and review

**Work items**

- [ ] Serve React assets and validated APIs from the on-prem TypeScript server.
- [ ] Add repository/file selection, redacted resolution preview and prerequisites.
- [ ] Evaluate the N-API binding; define generated records and the async adapter contract.
- [ ] Implement native preparation on the dedicated Rust execution thread.
- [ ] Package matching server/addon builds for the declared Linux host targets.

**Validation**

- [ ] A second machine completes review against a headless host using its server URL.
- [ ] File order, env, profiles and service selection resolve without source edits.
- [ ] Bad credentials, escaping paths, missing images and unsupported inputs fail clearly.
- [ ] A packaged clean install loads the addon; incompatible/missing binaries fail at startup.
- [ ] Preparation leaves the Node event loop responsive; native errors become typed failures.

## M2 — Run and observe

**Work items**

- [ ] Bind existing Rust Compose execution and CPU capture with explicit run ownership.
- [ ] Add durable admission/idempotency, input rechecks, Stop and shutdown handling.
- [ ] Persist bounded activity and render live status with reconnect by run ID.
- [ ] Implement interruption detection and blocked admission pending reconciliation.

**Validation**

- [ ] Real workloads produce advancing CPU evidence and visible lifecycle events.
- [ ] Stale review, duplicate/concurrent Start and Stop during startup behave correctly.
- [ ] Browser disconnect and slow event consumers leave execution responsive; gaps are visible.
- [ ] Stop and graceful shutdown await finalization; handles release after completion.
- [ ] Process termination leaves an interrupted run; restart blocks until cleanup is verified.
- [ ] Workload failure, capture failure and disk exhaustion report distinct outcomes.

## M3 — Inspect the complete experiment

**Work items**

- [ ] Add saved-run selection, final reports and streamed artifact downloads.
- [ ] Document startup, supported host/Node versions, prerequisites and reconciliation.
- [ ] Record end-to-end acceptance evidence beside the checks below.

**Validation**

- [ ] Complete Adopt → Review → Run → Stop → Inspect from a remote browser.
- [ ] Download readable CPU MCAP; reopen history and reports after server restart.
- [ ] Run twice with fresh identities; add a Compose service without workflow edits.
- [ ] Source/Compose files remain unchanged; provenance gaps are visible in the report.
- [ ] Client/server, native-boundary and relevant Rust checks pass with real Docker coverage.
- [ ] Validate declared Linux packages, including serial ARM64 CI where supported; record
  host, commands and results. Qualify GPU prerequisites separately if later claimed.

MVP is complete when all three milestones pass. Broader mockups remain follow-up scope.

[Product direction](../../apps/project-explorer/docs/102926-compose-adoption-experience.md) ·
[Mockups](../../apps/project-explorer/docs/102926-workflow-mockups-latest.md) ·
[Existing execution](../../libs/workload-docker/docs/102626-compose-workflow.md)
