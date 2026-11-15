# Compose app exits from Docker events: implementation

Created: 2026-11-15. [Architecture](111526-exits-from-docker-events.md).

Applicable rules: D-02/04/10/11/13; I-03/04/05/07.

## Code changes

- Delete `READY_FILE` and `ready()` from `dora_app::environment` and their callers.
- Remove `healthcheck` and `DORAMCAP_READY_FILE` from Console system apps and app deploy files.
- Change demo `depends_on` conditions from `service_healthy` to `service_started`.
- Console readiness logic stays: no health status counts as started; a customer healthcheck is honored.
- Tests that used ready files to order services synchronize on observable outputs instead.

## Milestone 1: remove probes and ready files

**Work items**
- [ ] Delete the ready-file contract from `dora-app` and the apps.
- [ ] Remove healthchecks and ready-file settings from Compose files.
- [ ] Rework tests that relied on ready files.

**Validation**
- [ ] Focused crate tests and Clippy
- [ ] Docker workload tests
- [ ] Demo Compose stacks start
- [ ] ARM64 host CPU before and after
