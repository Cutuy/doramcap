# App health without probes: implementation

Created: 2026-11-15. [Architecture](111526-exits-from-docker-events.md).

Applicable rules: D-02/04/10/11/13/15; I-03/04/05/07.

## Milestone 1: remove probes and ready files

Console readiness logic stays: no health status counts as started, and a customer
healthcheck is honored. Exits keep coming from Docker `die` events.

**Work items**
- [x] Delete `READY_FILE` and `ready()` from `dora_app::environment` and their callers.
- [x] Remove `healthcheck` and `DORAMCAP_READY_FILE` from Console system apps and app deploy files.
- [x] Remove the demos' HTTP and file healthchecks; demo validation scripts check output instead.
- [x] Change demo `depends_on` conditions from `service_healthy` to `service_started`.
- [x] Rework tests that ordered services through ready files.

**Validation**
- [x] Focused crate tests and Clippy
- [x] Docker workload tests
- [ ] Demo Compose stacks start
- [x] ARM64 host CPU before and after

## Milestone 2: system apps report health

**Work items**
- [x] live-bridge publishes a `health` output through `fault-dora`.
- [x] The watchdog consumes CPU monitor, data recorder and live bridge health; its own
  row uses container state and its existing diagnostics stay in `faults`.

**Validation**
- [ ] Watchdog snapshot includes the selected health publishers
- [ ] A stopped app turns stale or disconnected in the watchdog

## Prerequisite for milestone 3: live Docker status

**Work items**
- [x] Carry run-owned container starts and exits from the existing event stream to Console status.
- [x] Retain facts by container ID and expose incomplete observations.
- [x] Deliver the last lifecycle fact with completion; keep live facts out of saved History.

**Validation**
- [x] Event and native tests
- [x] Backend status and History
- [x] Docker start and exit delivery

## Milestone 3: live health panel

**Work items**
- [x] Console shows watchdog app names, source identities, conditions, freshness and connection state.
- [x] Docker service rows show running, exited and unknown facts alongside reported health.
- [x] Last-report timestamps and observation gaps remain visible when reporting stops.

**Validation**
- [x] Frontend tests: healthy, active fault, stale, disconnected, exited
- [x] Browser check in light and dark
