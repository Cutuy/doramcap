![Console responsibilities](102926-console-boundary.svg)

# doramcap Console MVP

Created: 2026-10-29. Status: implementation pending.

**Console** runs one existing Compose application with CPU capture. The user
selects a project, reviews it, starts it, watches workflow steps, then stops it
and downloads the result. One page presents these successive states.

## Fixed scope

- One on-prem Linux host, one Node.js server, one user and one active run.
- One project directory with a default Compose entry point and prebuilt local images.
- Optional profiles declared by that project. Start the complete selected application.
- Existing Rust Compose workflow, CPU monitor and artifact/report formats.
- First acceptance target: a small real Compose fixture. vEDGAR `singlepc` is a
  subsequent compatibility pilot; its GPU, simulator and interactive requirements
  are outside MVP acceptance.

The host can be headless; a browser opens the server URL over the local network.
Startup config supplies the listen address, approved project root, output directory
and single-user access. Package one declared Linux host target first.

Defer custom Compose/env-file pickers, file ordering, individual service selection,
project presets, builds/pulls, saved-run browsing, durable event replay, live CPU
charts, container dashboards, workflow editors, A/B, queues and remote execution.

## One page, four states

| State | User sees and does |
|---|---|
| Adopt | Enter a server-side project directory; optionally enable its declared profiles. Click Review. |
| Review | See services that will run, CPU capture availability and blocking prerequisites. Click Start or return to Adopt. |
| Run | See workflow step names, status, duration and redacted errors. Click Stop; continue watching finalization. |
| Inspect | See execution, evidence and cleanup outcomes, and download the current run's CPU MCAP and report. Start another run after verified cleanup. |

Let Compose resolve its default files, overrides and `.env` in the selected project
directory. Enumerate profiles with `docker compose config --profiles`; use Compose's
resolved model for review. Resolve and launch with the same recorded inputs.
Service-level `env_file` and included files remain project-owned configuration.
If a project needs a custom invocation or missing settings, explain the unsupported
requirement. The user prepares that project outside Console.

Review is read-only: no environment editor, mount editor or per-service toggles.
Changing the directory or profiles invalidates the review. Start revalidates the
prepared inputs and requires a fresh review if they changed. Keep the existing
runner's input identities, coverage diagnostics and admission checks.

## Existing libraries and the native boundary

![Native library interop](102926-console-interop.svg)

| Location | Responsibility |
|---|---|
| `console/frontend/` | TypeScript, React, Vite; the four page states |
| `console/server/` | TypeScript, Node.js; HTTP, current-run ownership and downloads |
| `console/server/native/` | Rust N-API adapter over the existing runner and telemetry |
| `console/docs/` | This plan and its diagrams |

Use a native addon. Evaluate [napi-rs](https://napi.rs/) before adding a dependency.
Execute Rust work on a dedicated thread with its own async runtime/local task
context; construct the existing `Rc`-based runner there. Transfer owned records
through bounded queues and keep JavaScript values on the Node thread. Reuse
generated protobuf records and pbtxt settings. Artifacts stay on disk; HTTP streams
registered downloads. See [Node-API](https://nodejs.org/api/n-api.html).

The adapter needs only `prepare(directory, profiles)`, `start(preparedId, runId)`,
`run.status()`, `run.stop()`, `run.completed` and `close()`. Preparation, Start and
completion are async; Stop accepts an idempotent cooperative request, including
during startup. `close()` stops admission and awaits finalization. The server owns
the run until completion; garbage collection never controls shutdown.

### Run shows workflow telemetry

Pass the existing [TelemetryObserver](../../libs/orchestrator/src/telemetry.rs)
into the [Compose runner](../../libs/workload-docker/src/workflows/recorded_compose/mod.rs).
Reuse the telemetry group's [workflow-protocol](../../libs/telemetry/workflow-protocol/)
[event schema](../../libs/telemetry/workflow-protocol/proto/workflow.proto).

- Keep one row per `(session, executor, invocation)`, with the emitted step name.
  Repeated invocations get separate rows; child rows follow the emitted parent ID.
- Map `STARTED` to Running; show `SUCCEEDED`, `FAILED`, `CANCELLED` and `PANICKED`
  as their corresponding terminal states. Use emitted durations for completed steps.
- Poll the current status snapshot once per second. Refreshing the page reloads it;
  the browser does not replay an event log or reconstruct the workflow.
- The observer sink only admits owned events to a bounded queue. A consumer reduces
  them into a bounded current-run snapshot; slow browsers never block execution.
  Expose sequence gaps, dropped-event counts and retention overflow as incomplete
  progress. Never infer a missing step outcome from a container's state.
- Strip payloads and redact errors before exposing records. The existing runner can
  emit sensitive configuration in event payloads. Keep final completion/report
  available independently of telemetry delivery.

## Minimal server state and lifecycle

Expose prepare, Start, current status, Stop and current-result downloads over HTTP.
Atomically persist a current-run marker with the run ID and Start idempotency key before launch;
a repeated Start returns that run, and overlapping Start is rejected. Keep progress
in memory. Preserve existing run artifacts on disk; Console has no history index.
Retain the latest result reference in the marker so Inspect survives a restart.

Browser disconnect leaves execution running. Stop and server shutdown request
cooperative finalization and await the report within a documented deadline.
An unfinished marker after process loss blocks new runs and shows Interrupted /
cleanup unknown. Document manual reconciliation using the owned-resource evidence;
require verified cleanup before clearing the marker. Automatic recovery is deferred.
A failed workload may still produce valid CPU evidence; retain the report's separate
execution, evidence and cleanup outcomes.

Validate HTTP/native inputs, protect mutations against cross-origin requests, keep
project reads within the approved root and downloads within registered artifacts.
Keep secret-bearing manifests private. Persistence failures block launch or remain
visible in the result. Reuse the runner's recorded provenance and coverage limits.

## M1 — Review one project

**Work items**

- [ ] Serve the single page and API; configure host paths and single-user access.
- [ ] Resolve the default Compose project and list its declared profiles.
- [ ] Evaluate the binding and implement async native preparation and read-only review.
- [ ] Package the server/addon for the first declared Linux host target.

**Validation**

- [ ] From a second machine, review a fixture using only its host directory.
- [ ] Default files, `.env` and an optional profile match the equivalent Compose CLI invocation.
- [ ] Missing prerequisites, unsupported custom invocation and escaping paths fail clearly.
- [ ] The packaged addon loads; preparation leaves HTTP responsive and secrets redacted.

## M2 — Run with workflow progress

**Work items**

- [ ] Start the existing Compose/CPU workflow; retain run ownership and a durable marker.
- [ ] Connect TelemetryObserver and workflow-protocol to the bounded step snapshot.
- [ ] Render emitted step states, durations and errors; add Stop and awaited shutdown.

**Validation**

- [ ] Real workflow events produce the expected Running and terminal step rows, including child/repeated invocations.
- [ ] Failure, cancellation and panic events render correctly; gaps/overflow show incomplete progress.
- [ ] Refresh/disconnect and a slow consumer leave capture and cleanup responsive.
- [ ] Changed review inputs and overlapping Start are rejected; duplicate Start returns the same run.
- [ ] Stop during startup and normal Stop await finalization; unfinished restart blocks a new run.

## M3 — Finish and download

**Work items**

- [ ] Show the authoritative report and stream the current run's registered artifacts.
- [ ] Restore the latest result after restart; allow another run after verified cleanup.
- [ ] Document startup, prerequisites, supported versions and manual reconciliation.

**Validation**

- [ ] Complete directory → Review → Start → workflow progress → Stop → downloads from a remote browser.
- [ ] Download readable CPU MCAP and report; verify separate execution/evidence/cleanup outcomes.
- [ ] Reopen the latest result after restart, then complete a second run with fresh identity.
- [ ] Source/Compose files remain unchanged; failed cleanup blocks the next run.
- [ ] Client/native-boundary tests and relevant Rust/real-Docker checks pass; record commands and evidence here.

MVP is complete when these checks pass. Record evidence beside each milestone
before checking its items. Qualify vEDGAR and other repositories in follow-up work.

[Existing execution](../../libs/workload-docker/docs/102626-compose-workflow.md)
