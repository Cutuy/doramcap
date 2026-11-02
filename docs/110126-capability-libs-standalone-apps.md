![Capability libs and standalone apps](110126-capability-libs-standalone-apps.svg)

# Capability libs and standalone apps

Created: 2026-11-01.

- **Libs:** one capability each, plus basic logging or telemetry integration.
- **Apps:** standalone executables, each in its own container, launched with a pbtxt.
- **Recording:** an app that emits data writes its own MCAP when its pbtxt asks for it.
- **Live data:** apps can publish to Dora; the live bridge relays those outputs to the frontend.
- **Gallery and Console:** launch the same apps. Gallery uses prebuilt pbtxt; Console offers profiles.
- **`mcap-recorder`:** records existing apps' messages from Dora, and ROS 2 later.

## Milestone 1: Generic recorder (PR 34)

**Work items**
- [x] Remove CPU validation and CPU protocol dependencies from `mcap-recorder`.
- [x] Fold `telemetry-mcap` into `mcap-recorder`.
- [x] Remove the recorder's gallery deployment and bundled monitor.
- [x] Record gallery CPU data inside `system-monitor`.

**Validation**
- [x] Recorder, System Monitor, workload lifecycle and Project Explorer tests pass; Docker test targets build.
- [x] Strict Clippy, formatting and dependency boundaries pass.
- [ ] Real Dora recording equality passes.
- [ ] Docker gallery and shared-IPC tests pass.

Evidence: `cargo test --release --locked` for the listed packages passes, with Docker-only tests ignored. BPF builds need `clang`, which this host lacks.

## Milestone 2: CPU app configuration

**Work items**
- [ ] Add `system-monitor --config FILE.pbtxt`, with optional `recording` and `dora` sections.
- [ ] Replace CPU flags and `recording.proto` with that configuration.
- [ ] Move cgroup dictionary state into a CPU lib; keep the sampling loop in the app.

**Validation**
- [ ] Config tests cover Dora-only, MCAP-only and combined launches.
- [ ] Native recording and Dora equality checks pass.

## Milestone 3: Lib boundaries

**Work items**
- [ ] Make the `activity-tracing` MCAP sink an optional feature; apps select sinks.
- [ ] Move the watchdog engine into a fault lib; keep sinks in the app.
- [ ] Remove `fault-demo`'s dependency on the `fault-watchdog` app.
- [ ] Add boundary rules that keep MCAP out of capability libs, except the tracing feature.

**Validation**
- [ ] Feature combinations for `activity-tracing` build and pass.
- [ ] Fault lib, watchdog and demo tests pass.
- [ ] Boundary checks reject a planted violation.

## Milestone 4: Watchdog variants

**Work items**
- [ ] Use one watchdog Compose file and registration.
- [ ] Express the MCAP and OTLP variants as pbtxt files.

**Validation**
- [ ] Gallery deployment tests cover each pbtxt variant.

## Milestone 5: Console profiles

**Work items**
- [ ] Add `launch_profiles` to `SystemAppDefinition`: label, pbtxt, recordings and live outputs.
- [ ] Add per-app profile selection to prepare and run requests.
- [ ] Add a profile picker to the System apps panel.
- [ ] Mount the selected pbtxt read-only into the app container.

**Validation**
- [ ] Server tests cover default, valid and rejected profile selections.
- [ ] Frontend tests cover the picker and review state.
- [ ] A Console run records and streams data according to the selected profile.
