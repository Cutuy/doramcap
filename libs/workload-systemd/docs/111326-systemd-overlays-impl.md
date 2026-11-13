# systemd overlays: implementation plan

Created November 13, 2026. Status: implementation in progress.
[Architecture](111326-systemd-overlays.md). Terms follow the overlay design's
[term list](../../../console/docs/111026-file-change-report.md#terms).

## Starting point

| Piece | Where |
| --- | --- |
| Overlays, change list from the uppers, write policy | `host_filesystem::overlay`: `resolve`, `prepare`, `changes`, `finalize` |
| Shared run-end steps | `workload_lifecycle::capture` (`FinalizeOverlays` and the change-list steps), used by `compose-apps` |
| System apps | `manager::transient_properties` and `manager::private_root` |
| User apps | `manager::adopt` links a captured copy; `preparation` captures unit and drop-ins |
| Run report | `runner::SystemdRunReport`; Console's `systemd.ts` shows file changes as unavailable |

## Milestone 1: Real-host probe

**Work items**

- [x] On a real systemd host, mount an overlay from Console's process and start a
  transient service and an adopted unit that bind its overlay directory. Confirm both
  see the overlay.
- [x] Confirm an owned drop-in can reset `BindPaths=` and list every entry again, keeping
  the others and their order, for a unit whose own drop-ins also set `BindPaths=`.
- [x] Stop and report to the user if either fails.

**Validation**

- [x] Overlay visible to transient and adopted units.
- [x] Drop-in reset preserves entry order.
- [x] Drop-in reset preserves read-only binds.

The [host probe](../tests/overlay-probe.mts) uses runtime-only units and one owned
overlay. An empty `BindPaths=` also clears `BindReadOnlyPaths=` on systemd 255. Restoring
both lists preserves their entries, flags and order. Seventeen other exposed mount
properties remain unchanged, including path restrictions, tmpfs and sandbox settings.
The orchestrator approved this single drop-in path; no captured-copy fallback is needed.

## Milestone 2: System apps

**Work items**

- [x] Collect writable binds from `Program::binds`; resolve one
  overlay plan through `host_filesystem::overlay::resolve` at Review and Start.
- [x] Mount overlays before any unit starts; pass overlay directories to
  `private_root` in place of host folders. Keep targets and access flags.
- [x] Delete the extra writable mount `private_root` adds for an undeclared working directory.
- [x] Check declared working directories at Review and before mounting. For a system app,
  check inside its private root. Fail naming the app and directory.
- [ ] Test that the Dora coordinator and daemon keep `/runtime` under their declared output folder.

**Validation**

- [x] Writes stay out of the host folder during the run.
- [x] Missing working directory fails at Review and Start.
- [ ] Dora system apps start unchanged.

## Milestone 3: User apps

**Work items**

- [x] Read each unit's `BindPaths=` from the service manager after linking the captured copy.
- [x] Write the owned drop-in from milestone 1 with overlay sources.
- [x] Skip the working directory check for `WorkingDirectory=-/path`.
- [x] At Review, list a writable working directory outside every writable mount, and the
  unit's other writes, as not tracked.

**Validation**

- [x] User app sees overlay directories at its declared paths.
- [x] `WorkingDirectory=-/path` starts.
- [x] Not-tracked notes at Review.

## Milestone 4: Run end and report

**Work items**

- [ ] Agree the shared step IDs with the user (see the architecture's decisions).
- [ ] Stop all started units and wait until none holds an overlay, then run the shared
  read, save, unmount and write-policy steps.
- [ ] Add the shared file-change result to `SystemdRunReport` and carry it to Console's
  run result. Remove the "unavailable" case for systemd in `systemd.ts`.
- [ ] Record overlay ownership so interrupted-run reset stops units, then discards overlays.

**Validation**

- [ ] Equivalent Compose and systemd fixtures give equivalent change records.
- [ ] Write back reaches the host folder after the run.
- [ ] Rollback, cancellation, busy overlay, report-save failure and Console restart.
- [ ] Owned mounts and units gone after success.
- [ ] Real systemd run shows File changes in Console.

## Tests

Capability-required tests build unprivileged and elevate only the test executable.
Scripted end-to-end checks are TypeScript.
