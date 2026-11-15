# App health without probes: implementation

Created: 2026-11-15. [Architecture](111526-exits-from-docker-events.md).

Applicable rules: D-02/04/10/11/13/15; I-03/04/05/07.

## Milestone 1: remove probes and ready files

Console readiness logic stays: no health status counts as started, and a customer
healthcheck is honored. Exits keep coming from Docker `die` events.

**Work items**
- [ ] Delete `READY_FILE` and `ready()` from `dora_app::environment` and their callers.
- [ ] Remove `healthcheck` and `DORAMCAP_READY_FILE` from Console system apps and app deploy files.
- [ ] Remove the demos' HTTP and file healthchecks; demo validation scripts check output instead.
- [ ] Change demo `depends_on` conditions from `service_healthy` to `service_started`.
- [ ] Rework tests that ordered services through ready files.

**Validation**
- [ ] Focused crate tests and Clippy
- [ ] Docker workload tests
- [ ] Demo Compose stacks start
- [ ] ARM64 host CPU before and after

## Milestone 2: system apps report health

**Work items**
- [ ] live-bridge publishes a `health` output through `fault-dora`.
- [ ] Apps without a `health` output gain one; the watchdog already consumes selected apps' outputs.

**Validation**
- [ ] Watchdog snapshot lists each selected system app as a connected source
- [ ] A stopped app turns stale or disconnected in the watchdog

## Milestone 3: live health panel

**Work items**
- [ ] Console shows one row per system app: watchdog health for Dora apps, running or exited
  from Docker events for the Dora coordinator and daemon.

**Validation**
- [ ] Frontend tests: healthy, active fault, stale, disconnected, exited
- [ ] Browser check in light and dark
