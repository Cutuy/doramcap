# Per-app cgroups: implementation

Created: 2026-11-12. [Architecture](111226-per-app-cgroups.md).

Applicable rules: D-02/03/04/06/12/13; I-03/04/05/07/09.

## Milestone 1: Compose per-service groups

`compose_apps::cgroup::placement` keeps returning the `system` group as
`CgroupPlacement`. `service_parent(placement, role, service)` derives each service's
Docker `cgroup_parent`:

| Driver | Run group | Service parent |
| --- | --- | --- |
| systemd | `doramcap-compose-‹run›.slice` | `doramcap-compose-‹run›-‹role›-‹escaped service›.slice` |
| cgroupfs | `/doramcap-compose-‹run›` | `/doramcap-compose-‹run›/‹role›/‹service›` |

**Work items**
- [x] Generate the placement whenever a system service or a user service without its own `cgroup_parent` exists.
- [x] Assign per-service parents to system, Dora, shm and user services; keep user-set `cgroup_parent`.
- [x] Snapshot generated parents as `@run-cgroup/‹role›/‹service›`.
- [x] Verify each system container against its own service parent.
- [x] Register the Docker group for every placement; create the system identity only with system apps.
- [x] Clean up by removing the whole run group.

**Validation**
- [x] Preparation and snapshot unit tests.
- [x] Native per-service placement, user-set parent kept, cleanup for both drivers.

## Milestone 2: Native systemd user slice

**Work items**
- [x] Place generated user programs in `‹repo›-user.slice`.
- [x] Add the slice drop-in to adopted user units only when they set no `Slice=`; system units keep the override.
- [x] Remove the system, user and run slices.

**Validation**
- [x] Native run places user units in the user slice and leaves no slices.
- [x] Native suite, Compose and Docker cgroup tests for both drivers.
- [x] Adopted user unit with its own `Slice=` keeps it.
