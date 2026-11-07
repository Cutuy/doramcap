![Launch forms architecture](110726-launch-forms.svg)

# Launch forms: Compose, systemd and Supervisor

Created: 2026-11-07. Status: design agreed with the owner; Milestone 1 not started.

Run an experiment with the launch method its repo uses: Compose, systemd or
Supervisor. Dora, system apps and user apps all launch the same way within a run.
Shared sub-workflows handle preparation, evidence and reporting; each form owns how
its processes start, are watched, stop and are removed.

High-level plan: move Compose onto a shared run sequence (M1), resolve launches from
app manifests and a per-machine deployment (M2), then add systemd over D-Bus (M3).
Supervisor and the remaining features follow on demand.

## Accepted design

Agreed with the owner on 2026-11-07.

| Area | Decision |
| --- | --- |
| Forms | Compose, systemd and Supervisor; the deployment picks one form per run. Unit tests use a fake launcher. |
| Apps | Dora, system apps and user apps launch the same way within a form. |
| Reuse | Step libraries grouped by substrate. Code is shared through sub-workflows or individual steps. |
| Sequence | Shared Prepare, Finalize and Report sub-workflows; form-owned start, stop and remove. Phases exchange protobuf records. |
| Handle | `start` returns a typed running handle, even after a failed start. `stop` yields a `StopRecord`; `remove(self)` consumes the handle. |
| Lifecycle authority | Each form's manager: unit state over D-Bus, supervisord events, Docker events. A pidfd is used only by the process's parent. |
| Monitoring | Layer 1: an optional status feed from the handle. Layer 2: Fault Watchdog consumes health that apps publish in code through the reporting library. Neither ends the run. |
| Run end | The run ends on a stop request. A failed unit or app is recorded in the `StopRecord`. |
| Removal | Remove runs only after Finalize issues a `RemovalPermit`. |
| Ordering | The plan declares start groups: Dora, then system apps, then user apps. Doramcap enforces the gates; stop runs in reverse. |
| systemd control | D-Bus through `zbus`, pending crate evaluation. |
| Supervisor control | XML-RPC over the owned daemon's socket, plus an event-listener relay. |
| MVP | Compose and systemd. Supervisor, the live status feed and other items are listed under Future items. |

## Config layers

![Config layers](110726-config-layers.svg)

| Layer | File | Owner | Holds |
| --- | --- | --- | --- |
| Variant config | e.g. `watchdog-mcap.pbtxt`, unchanged | App, own schema | What the app does |
| App manifest | `apps/x/deploy/app.pbtxt` | App | Launch facts, variants and the recordings each adds, graph, outputs |
| Deployment | `deployment.pbtxt`, one per machine | Operator | Form, apps, default variants, placement, install root, repo config |
| Repo config | `compose.yaml`, `.service`, `supervisord.conf` | User | User apps, in their native format |

Generated per run: Console Review variant overrides, `plan.pbtxt`, frozen config
copies and the backend input. The plan has the same shape for every form; only the
backend input differs.

## Run sequence

![Run handle and monitoring](110726-run-handle.svg)

```rust
let prepared = prepare(plan, |previous| launcher.reset(previous)).await?;
let (mut running, started) = launcher.start(&prepared).await;
let ended = match started {
    Ok(()) => monitor(&prepared, running.status(), faults, stop).await,
    Err(error) => Some(error),
};
let stopped = running.stop().await;
let permit = finalize(&prepared, &stopped).await;
let cleanup = running.remove(&permit).await;
report(&prepared, ended, &stopped, &cleanup).await
```

`experiment::<F>()` holds this sequence once. One `match` on the deployment's form
selects the launcher.

## systemd mode

![systemd mode](110726-systemd-slice.svg)

Each run owns one slice. The graph accepts only dynamic nodes, so each app is one unit
connected to the Dora daemon. Units share the host `/dev/shm`; units must not set
`PrivateIPC`. Ownership records the run's shared-memory segments for reset.

| Manifest fact | Unit property |
| --- | --- |
| Executable and config argument | `ExecStart=` |
| Dora participant identity | `User=`, `Group=` |
| Memlock | `LimitMEMLOCK=` |
| Placement | `AllowedCPUs=`, scheduler properties from `resource-policy` |
| Stop signal and grace | `KillSignal=`, `TimeoutStopSec=` |
| Capabilities | `AmbientCapabilities=` |
| Readiness | `Type=notify` once the readiness protocol is decided |

## Dependencies for evaluation

| Need | Candidates |
| --- | --- |
| D-Bus client | `zbus`; required for systemd control |
| systemd proxies | `zbus_systemd` generated proxies, or hand-written `zbus` proxies for the few manager and unit calls; `systemd-transient` and `systemd-run` are references |
| Supervisor XML-RPC (later) | Existing `hyper`/`hyperlocal` plus a small codec, or `quick-xml` |

## Milestone 1: Shared run sequence with Compose

**Work items**
- [ ] Define record messages: `Prepared`, `StopRecord`, `RemovalPermit`, `CleanupRecord`, following the protobuf contract style.
- [ ] Move Prepare, Finalize and Report out of `recorded_compose` into `workload-lifecycle`. Decouple file evidence and machine capture from `DockerAdapter`; Compose adds its Docker facts.
- [ ] Add the launcher and running-handle interfaces and `experiment::<F>()`. Compose implements them and keeps `RunSignals`.
- [ ] Add a fake launcher for unit tests.
- [ ] Record the form in the ownership record; reset handles the same form only.

**Validation**
- [ ] Native Compose end-to-end tests pass with unchanged report fields.
- [ ] Console runs a project end to end; History, compare and recordings are unchanged.
- [ ] Fake-launcher tests: a failed start still removes; remove waits for a permit; a stop during start skips readiness; admission reopens only after clean removal.
- [ ] Focused Clippy, formatting and library-boundary checks.

## Milestone 2: Manifests and deployment

**Work items**
- [ ] Decide with the owner where the deployment and manifest protos live.
- [ ] Add the app manifest message: launch facts, requirements, graph, outputs, live outputs and variants with their recordings.
- [ ] Extend the deployment with the form, app selection, variants, install root and repo config. Existing `resource-policy` configs keep parsing.
- [ ] Write manifests for Dora, CPU monitoring, data recording, Fault Watchdog and the live bridge.
- [ ] Prepare resolves `plan.pbtxt` and frozen configs; Console Review supplies variant overrides.
- [ ] Console config lists manifests; launch fields move out of `SystemAppDefinition`.

**Validation**
- [ ] Resolution tests from the release manifests and deployment produce the expected plan.
- [ ] Compose runs from a plan freeze the same Compose project as before.
- [ ] Console Review variant selection passes the existing Console suite.
- [ ] Existing `resource-policy` configs parse unchanged.

## Milestone 3: systemd launcher

**Work items**
- [ ] Spike: adopt repo `.service` units unchanged through runtime linking and place them in the run slice. Record the result here before building on it.
- [ ] Present the `zbus` and proxy options to the owner and adopt the chosen crates.
- [ ] systemd control steps: subscribe, start transient units, await job removal, read unit properties, stop units, reset failed units.
- [ ] Dora coordinator and daemon units from the Dora manifest; graph registration; binaries from the install root.
- [ ] System app units from manifests using the property mapping above.
- [ ] Ownership of the slice, units and `/dev/shm` segments; reset stops the slice and removes owned segments.
- [ ] `StopRecord` from unit `Result`, main exit status and accounting, read before units are collected.

**Validation**
- [ ] Native run of CPU monitoring, Fault Watchdog and one user unit: recordings finalize and the slice is gone after removal.
- [ ] Doramcap killed mid-run: the next run's reset removes the slice and shared-memory segments.
- [ ] A failing user unit appears in the `StopRecord` and the run continues until stop.
- [ ] Fault Watchdog receives health from a user app under systemd.
- [ ] ARM64 worker run, or a recorded reason it is blocked.

## Future items

| Item | Notes |
| --- | --- |
| Supervisor launcher | Move `workload-supervisor` onto the shared sequence; XML-RPC; event-listener relay; then delete `recorded_run::Phases`. |
| Live status feed | Layer 1 monitoring for the Console live view. |
| Readiness protocol | `sd_notify` everywhere, or a ready file; decide with the owner. |
| Child-process launcher | Real processes for integration tests; pidfd and `waitid` as parent. |
| Profiler attach | By cgroup or PID, in every form. |
| Native filesystem isolation | Compose keeps OverlayFS; native forms use baseline diffs until then. |
| Cross-form reset | Reset a previous run of a different form from its ownership record. |
| Generated Compose files | Build system-app Compose services from manifests. |
| Crate layout | Decide where launchers and native steps live before Milestone 3. |
