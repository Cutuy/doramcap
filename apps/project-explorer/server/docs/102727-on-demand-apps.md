![On-demand app hosting](102727-on-demand-apps.svg)

# On-demand app hosting

Created: 2026-10-27. Status: complete for the enabled apps.

## High-level plan

1. Establish the server-owned app registry and singleton lifecycle.
2. Package Activity Lab as the first Compose app.
3. Connect gallery launch, shared viewing and streaming.
4. Close the idle-shutdown, recording and cleanup loop.
5. Qualify additional apps through reusable wrappers.

## M1 — Gallery server and instance ownership (complete)

### Work items

- [x] Add server-owned registrations for Compose definitions, readiness, endpoints and result paths.
- [x] Implement per-app generation/state transitions and join concurrent launch requests.
- [x] Return startup status promptly; expose failures and block replacement after unverified cleanup.
- [x] Detect existing manager-owned resources at startup and require cleanup before replacement.

### Evidence

- The per-app coordinator has Rust tests for concurrent launch joining,
  independent sessions, expiry, launch waiting for cleanup, blocked replacement,
  abandoned startup and stale generations.
- A trusted registration and reusable procedural workflow now use the existing
  orchestrator/Compose steps. Two local Docker runs retained readable MCAPs,
  verified cleanup and captured host-bind/container-layer metadata. Discovery and a local HTTP integration test also passed.
- Engine discovery finds labeled containers and networks. A real-Engine test
  verified a restarted manager blocks duplicate launch; explicit recovery collected
  evidence, removed the owned resources and allowed a new generation.
- The supervisor joined concurrent launches, expired their sessions, retained a
  readable recording and verified cleanup in a local Docker run.
- The hosting API test passed origin and launch-setting rejection, single-manager
  locking, joined launch, readiness, two heartbeats and shutdown cleanup. The
  multi-app test launched Activity Lab and Storage Observatory independently.

### Validation

- [x] Concurrent launches for one app share one startup operation and generation.
- [x] Different apps can start independently; unknown app IDs and client-supplied launch settings are rejected.
- [x] Existing owned resources prevent duplicate launch after manager restart.
- [x] State-transition tests cover startup failure and cleanup-blocked replacement.

## M2 — Activity Lab Compose wrapper (complete)

### Work items

- [x] Package prebuilt assets/binaries and a trusted Compose definition for Activity Lab.
- [x] Configure readiness, SIGINT shutdown, owned network and fresh per-generation writable paths.
- [x] Qualify bind/base-path support and make only required app adaptations; keep sessions in shared hosting code.
- [x] Retain recording output outside disposable storage with an explicit volume policy.

### Evidence

- Activity Lab supports an explicit bind address and prefix-relative frontend URLs.
- Compose configuration passes `docker compose config --quiet`. On the local Docker Engine, the image served its frontend and two live WebSocket clients. Two sequential Compose launches used separate bind mounts; SIGINT produced 184 and 174 readable MCAP messages. Both projects left no containers or networks. The gallery manager now uses the same wrapper.
- Activity Lab Rust tests, frontend build/tests and a local live-stream smoke test pass.
- A local test proxy served prefix-relative assets and status and streamed to two WebSocket clients. Caddy integration was subsequently verified in M3.
- The generic workflow launched the registered wrapper twice, retained both recordings and captured distinct host-bind/container-layer change categories with verified teardown.

### Validation

- [x] The packaged app becomes ready and serves its frontend and live stream.
- [x] Graceful container stop produces a readable MCAP recording.
- [x] Two sequential launches use distinct writable paths and preserve the earlier recording.
- [x] Wrapper launch works without adding app-specific branches to core orchestration.

## M3 — Launch, shared viewers and proxy (complete)

### Work items

- [x] Add Launch/Open and a shared viewer shell with startup/error status and session heartbeats.
- [x] Proxy generation-scoped HTTP, downloads, WebSocket and SSE traffic incrementally.
- [x] Support base-relative assets/API URLs and required redirect/cookie path handling.
- [x] Keep static gallery browsing available and expose launch only with a hosting backend.

### Evidence

- Caddy routed the registered app's frontend, assets, status and two WebSocket
  clients; a spoofed upstream header was ignored. Idle expiry produced a 34,833-byte
  verified MCAP download, and the old generation route returned 404.
- Prefix-hosted Caddy tests passed redirect/cookie rewriting, SSE, a 3,145,984-byte
  response and upstream failure handling. The viewer shell and conditional Launch
  control build successfully. The production gallery build, TypeScript check and
  schema-backed content generation passed on 2026-10-27.
- Two independent headless Chromium instances shared a live Activity Lab
  generation with distinct sessions. Desktop (1440×900) and mobile (390×844)
  visual checks passed for root and `/gallery` hosting, launch, refresh, return
  navigation and recording access. A static-only gallery remained browsable
  and hid Launch. A mobile viewport gap found during review was fixed and
  checked again.
- Caddy now explicitly binds its configured interface. A Tailscale-only listener
  was reached from the ARM64 worker; Chromium launched Activity Lab through it.

### Validation

- [x] Two browsers receive live results from the same instance and have independent sessions.
- [x] Launch opens the shell from the user click; refresh, deep links and prefix hosting work.
- [x] Assets, downloads, WebSockets and SSE work through the proxy; upstream errors remain visible.
- [x] Large responses continue across buffer boundaries without total-result truncation.
- [x] Gallery production build/schema checks and manual desktop/mobile browser checks pass.

## M4 — Idle shutdown and verified cleanup (complete)

### Evidence

- Local Docker tests exercised session expiry and manager shutdown during startup;
  recordings and categorized filesystem metadata were retained after cleanup.
  The HTTP test also killed an app, observed the runtime and recording errors,
  verified cleanup and relaunched it. Lifecycle tests cover reconnect before expiry,
  late heartbeat rejection and launch waiting for cleanup. The two-app Docker test
  verified one viewer expiry leaves the other active, then stopped both instances,
  retained their recordings and removed their containers and networks. An isolated
  Docker startup-failure test reported the failure and verified owned container
  and network removal.
- Chromium background tabs renewed sessions, while a frozen or offline viewer
  expired and exposed its recording after reconnect. Closing a browser during
  deliberately delayed startup led to verified cleanup and an accessible recording.

### Work items

- [x] Apply the documented heartbeat/expiry policy using monotonic time and generation IDs.
- [x] Serialize expiry, launch and stop transitions; wait for cleanup before replacement.
- [x] Stop writers, finalize recordings, collect categorized metadata and remove owned resources.
- [x] Handle browser closure during startup, unexpected app exit and manager shutdown.
- [x] Preserve artifacts and report failure when finalization or cleanup cannot be verified.

### Validation

- [x] Disconnecting one viewer preserves another viewer's instance; last-session expiry stops it.
- [x] Reconnect within the grace period preserves the generation; late heartbeats cannot revive it.
- [x] Launch during stopping waits for verified cleanup before creating a new generation.
- [x] Background tabs, refresh, suspension and network loss follow the documented timeout policy.
- [x] Startup failure, browser closure during startup and app exit release owned resources.
- [x] Failed finalization is visible and preserves evidence; successful cleanup leaves no owned containers/networks.
- [x] End-to-end serial runs retain readable recordings and separate storage-change categories.

Viewer follow-up: one polling loop now observes shutdown through finalization.
A focused check covers late errors, successful downloads, expiry while another
viewer remains active, and transient result-request failure. Terminal 404s stop
retrying. This closes the audit's hidden-finalization-error finding.

## M5 — Additional apps and release qualification (complete)

### Evidence

- ARM64 worker jobs 159, 160 and 164 passed the workspace and native suites.
  Job 161 passed Docker launch, recovery, expiry, Caddy streaming, results
  and concurrent two-app checks with correctly staged frontend assets.
  Job 165 failed while resolving Docker Hub before qualification began. After
  DNS recovered, job 166 passed the latest Rust source and startup-failure test,
  plus recovery, Caddy streaming, recordings and concurrent two-app checks.
  Job 167 passed the workspace and native suites after the viewer layout fix.
- Local multi-app tests ran Activity Lab and opt-in Storage Observatory together,
  with two viewers each, expiry, retained MCAPs and sequential readiness.
- Registry admission tests cover simultaneous conflicting launches, resource release
  after verified cleanup and unrelated launches while a generation stops. The two
  enabled synthetic apps declare no exclusive hardware resources.

### Remaining-app inventory

This milestone qualified the original Activity/Storage pair. The
[remaining-app plan](102727-remaining-apps.md) tracks the expanded implementation
and release checks; the [hosting guide](hosting.md#app-registrations) lists current
selectable and pending profiles.

### Work items

- [x] Inventory remaining apps and record hosting gaps, readiness and shutdown behavior for each.
- [x] Add Compose wrappers individually, reusing launch/session/proxy machinery.
- [x] Document BPF/GPU/device needs, host-resource conflicts and allowed concurrent app combinations.
- [x] Document launch, result access, idle expiry and recovery from failed cleanup.

### Validation

- [x] Each enabled app passes launch, two-viewer streaming, expiry and sequential-relaunch checks.
- [x] Each wrapper preserves required results and removes its disposable resources.
- [x] Compatible apps run together; declared host-resource conflicts surface before launch.
- [x] Required Rust/native tests and ARM64 CI pass; capability-specific tests run on suitable hardware.
- [x] Final gallery build/schema validation and manual launch/status/result-access checks pass.

Record evidence under the relevant milestone before checking its items. Use
Arrange/Act/Assert in Rust tests; keep frontend verification to builds, schema
validation and manual checks per SPA instructions.

## M6 — Declarative, independent app registration (complete)

### Work items

- [x] Load validated app registrations from files beside their Compose wrappers.
- [x] Keep app images and binaries separately built; allow explicit opt-in without app-specific manager branches.
- [x] Preserve recovery, lifecycle, result handling and existing app behavior.

### Evidence

- The manager loads protobuf text registrations from each app’s `deploy/`
  directory using the shared protobuf toolchain. Unit tests cover discovery, explicit enablement, unknown
  IDs, duplicate IDs, unsafe paths and lifecycle coordination.
- The manager dependency tree contains no Activity Demo, Storage Demo or
  System Monitor app binary. App images remain separately built.
- ARM64 job 168 passed the workspace/native suite. Job 169 passed recovery,
  proxying, two-app lifecycle and a newly added opt-in manifest launched without
  rebuilding the manager.

### Validation

- [x] Tests cover discovery, opt-in, invalid registrations and lifecycle coordination.
- [x] Both apps launch, stream, retain results and clean up using declarative registrations.
- [x] Required Rust checks and ARM64 CI pass.


## Contract and ownership

One gallery manager owns at most one active instance per registered app on its server.
Multiple clients share that instance and its application state. Different apps may run
concurrently. Exclusive client ownership and conflicting-control detection are deferred.

| Owner | Responsibilities |
| --- | --- |
| Gallery frontend | Launch/Open, startup/error status, app viewer shell |
| Gallery server | App registry, instance state, viewer sessions, routing and streaming proxy |
| Procedural Rust workflow | Resolve, start, await readiness, stop, collect evidence, remove |
| `workload-docker` | Compose semantics, Engine inspection, verified resource cleanup |
| `orchestrator` | Typed step execution, injected services and execution telemetry |
| App + Compose wrapper | App process, readiness endpoint, mounts, signal and device settings |

Session policy stays in the gallery server. Backend-neutral lifecycle contracts remain
in `workload-lifecycle` when demonstrated common semantics warrant extraction.
Reuse Docker steps; select recording/monitoring by app needs rather than automatically
running the complete recorded-experiment workflow around every viewer.

## Launch and stop

`Stopped → Starting → Ready → Stopping → Stopped`. Startup/runtime/cleanup errors
remain visible; unverified cleanup blocks replacement of that app's instance.

1. Launch by app ID. The server allocates a generation and viewer session, or joins
   the existing startup/ready generation. A serialized per-app transition prevents
   duplicate creation; unrelated apps can launch independently.
2. Resolve the server-owned Compose definition, create fresh writable paths, start
   services, and wait for registered readiness. Return a shell URL and status promptly.
3. The shell renews its viewer session while showing startup progress, then embeds
   the app frontend. Launch opens this shell directly from the user's click to avoid
   popup blockers; same-tab navigation is also supported.
4. After the last session expires, stop writers, finalize recordings, collect available
   filesystem changes and remove owned containers/networks. Retain results separately.
5. A new launch during stopping waits for verified cleanup, then creates a new generation.
   A reconnect after expiry creates a new session only if that generation is still ready.

On manager shutdown, drain owned instances. Full crash recovery is deferred. On startup,
existing manager-labeled resources must be surfaced and block duplicate launch until
explicit cleanup; process-local state alone cannot establish the singleton guarantee.

## Viewer lifetime

The shared shell owns heartbeats; apps need no session SDK. Browsers heartbeat to the
server hosting the shell, independent of an app's data-stream connections. Opening an
app through the gallery always uses the shell so each viewer is counted.

Proposed policy: renew every 15 seconds, expire a session after 120 seconds without a
renewal. These are explicit, configurable idle-lifecycle settings, including startup;
there is no maximum active-session duration. Use server monotonic time and generation
IDs. Best-effort unload notification may shorten cleanup but is never required.

All live sessions keep the instance alive. Browser suspension beyond the grace period
can stop the instance; returning viewers see an expired-session state and can relaunch.
A still-open gallery tab alone does not keep an app alive. Heartbeat expiry and launch
transitions are serialized so cleanup cannot race a newly accepted session.

## Minimal app adaptations

Keep launch mechanics in `apps/<app>/deploy/` and trusted app registrations.
Build images ahead of launch; avoid compiling Rust/Node on each browser request.
Each registration identifies Compose files, service/port, readiness, stop policy,
result paths and required capabilities. Browser input selects an app ID only.

Prefer existing CLI/environment options. Adapt app code only where packaging exposes
an actual hosting gap: bind address, URL prefix, assets/results paths or graceful stop.
A common frontend URL helper and relative asset build settings can cover prefix routing;
shared proxy/session code belongs in the gallery server and viewer shell.

Activity Lab is the first candidate. It already accepts `--assets`, `--record` and
`--port`, serves `/api/status` and `/stream`, and finalizes MCAP on Ctrl-C. Its current
loopback bind and root-relative frontend API paths need qualification: add bind/base
support if necessary, or use a small proxy sidecar sharing its network namespace.
Its wrapper can set Docker's stop signal to SIGINT to reuse existing shutdown behavior.

## Routing and streaming

Use `/running/{app}/{generation}/` for the viewer shell and
`/app/{app}/{generation}/...` for proxied application traffic. Remove the upstream
prefix and handle redirect/cookie paths as needed. Apps must construct their API,
WebSocket and asset URLs relative to their assigned base; avoid brittle HTML rewriting.

The proxy streams HTTP bodies, downloads, SSE and WebSockets with backpressure.
Buffer/chunk sizes govern incremental work; they do not impose total-result or
session-length caps. Surface upstream closure and errors. App logs may preview a
small number of entries while full artifacts remain accessible.

A static/GitHub Pages gallery continues to browse documentation. On-demand Launch is
available when served with this backend. First deployment targets a trusted server;
external access requires an explicit authentication/access policy. Validate launch
origins and keep Docker control and upstream ports inaccessible to browser requests.

## Isolation and retention

- Give each app generation an owned Compose project/network and fresh writable directory.
- Use read-only image/config mounts where supported; place temporary data in container/tmpfs storage.
- Keep finalized recordings and metadata outside disposable storage; expose them through result routes.
- Preserve host-bind, container-layer and volume change categories. Metadata collection alone does
  not reset storage; host OverlayFS mounting/write-back remains separate planned work.
- Choose volume retention explicitly per wrapper. Never silently reuse writable scratch volumes.
- Qualify BPF, GPU, devices and shared host resources per app. Record incompatibilities before enabling
  simultaneous launch; container isolation alone cannot make conflicting hardware use safe.
- Keep the manager and its self-observation persistent. App services and their monitors stop together.

## Implementation choices

Reuse existing Tokio, Axum, tower-http and Bollard dependencies. The user selected
Caddy for reverse proxying after comparing it with Hyper/Hyper-util and
Tokio-tungstenite. Caddy integration and generation routing remain to be implemented.
Keep routing/proxy mechanics out of core orchestration.

## Evidence and scope

Inspected Activity Lab's README, server routes/bind/shutdown and frontend URL handling;
reviewed the gallery hosting contract and `workload-docker` step interfaces. These
support the proposed first integration but do not establish container/proxy compatibility.
Implementation checkboxes remain open until their complete work and validation pass.
