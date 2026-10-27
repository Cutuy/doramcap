![On-demand app hosting](102727-on-demand-apps.svg)

# On-demand app hosting

Created: 2026-10-27. Status: design proposal; implementation deferred.

## Milestones

- [ ] M1: Add the gallery server, registered app definitions and per-app instance state.
- [ ] M2: Wrap Activity Lab in Compose; implement readiness and graceful shutdown.
- [ ] M3: Add Launch/Open, the viewer shell, heartbeats and HTTP/WebSocket proxying.
- [ ] M4: Expire unused instances, finalize recordings and verify cleanup.
- [ ] M5: Package additional apps individually; record their required capabilities and adaptations.

## Validation

- [ ] Two concurrent launches create one instance and return the same app generation.
- [ ] Two browsers receive live results; disconnecting one preserves the other's instance.
- [ ] Reconnect during the grace period preserves the instance; last-session expiry stops it.
- [ ] Late heartbeats cannot revive a stopped generation; launch during stopping waits for cleanup.
- [ ] Startup failure, browser closure during startup and app exit release owned resources.
- [ ] Graceful stop produces readable MCAP; failed finalization is visible and preserves artifacts.
- [ ] Sequential launches use distinct writable paths; no app containers/networks remain after cleanup.
- [ ] Prefix hosting, assets, downloads, WebSockets, SSE and deep links work through the proxy.
- [ ] Background tabs, refresh, suspended clients and network loss exercise the documented timeout policy.
- [ ] Gallery build/schema checks, manual browser checks and Rust/native integration tests pass.
- [ ] Required ARM64 CI passes; app-specific capability tests run on suitable hardware.

Record evidence beside each completed milestone. Use Arrange/Act/Assert in Rust tests;
keep frontend verification to builds, schema validation and manual checks per SPA instructions.

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

Keep launch mechanics in `server/compose/<app>/` and trusted app registrations.
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

## Implementation choices to evaluate

Reuse existing Tokio, Axum, tower-http and Bollard dependencies. Evaluate Hyper/
Hyper-util for streaming HTTP and Tokio-tungstenite for WebSocket forwarding against
an established reverse proxy such as Caddy. Present dependency choices before adoption;
this proposal adds no dependencies. Keep routing/proxy mechanics out of core orchestration.

## Evidence and scope

Inspected Activity Lab's README, server routes/bind/shutdown and frontend URL handling;
reviewed the gallery hosting contract and `workload-docker` step interfaces. These
support the proposed first integration but do not establish container/proxy compatibility.
Implementation checkboxes remain open. No app packaging or launch code changes in this milestone.
