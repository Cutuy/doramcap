# systemd overlays: implementation plan

Created November 13, 2026. Status: implemented.
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
- [x] Test that the Dora coordinator and daemon keep `/runtime` under their declared output folder.

**Validation**

- [x] Writes stay out of the host folder during the run.
- [x] Missing working directory fails at Review and Start.
- [x] Dora system apps start unchanged.

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

- [x] Agree the shared step IDs with the user (see the architecture's decisions).
- [x] Stop all started units and wait until none holds an overlay, then run the shared
  read, save, unmount and write-policy steps.
- [x] Add the shared file-change result to `SystemdRunReport` and carry it to Console's
  run result. Remove the "unavailable" case for systemd in `systemd.ts`.
- [x] Record overlay ownership so interrupted-run reset stops units, then discards overlays.

**Validation**

- [x] Equivalent Compose and systemd fixtures give equivalent change records.
- [x] Write back reaches the host folder after the run.
- [x] Rollback, cancellation, busy overlay, report-save failure and Console restart.
- [x] Owned mounts and units gone after success.
- [x] Real systemd run shows File changes in Console.

## Shared write-policy integration

**Work items**

- [x] Consume the filesystem-owned enum through `RunPlan.write_policy`; retire
  `SystemdPlan.discard_writes`, leaving tag 5 and its name unreserved.
- [x] Remove the temporary Console rejection; use shared plan validation for invalid policies.
- [x] Reuse saved checkpoints, saved positions and the confined opener for downloads.

**Validation**

- [x] Keep and discard for system apps and user apps.
- [x] Keep-copy failure preserves uppers.
- [x] Console Review, Result and downloads after restart.
- [x] Headless Chromium policy and download actions.

## Milestone 5: Explicit live host folders

**Work items**

- [x] Carry `passthrough_folders` from Console through the shared run plan and filesystem policy.
- [x] Use the shared source classifier for Review, overlays and saved coverage notes.
- [x] Preserve passed-through user binds in the reset drop-in, with source, flags and order.

**Validation**

- [ ] Equal, inside, containing and unrelated folder matches.
- [ ] Compose and systemd live sockets and direct writes beside an overlaid folder.
- [ ] Review and Result in Chromium.

## Tests

Capability-required tests build unprivileged and elevate only the test executable.
Scripted end-to-end checks use TypeScript host and Console fixtures. Compare raw
relative-path bytes, change kinds, modes and regular-file sizes. Download positions
retain saved manifest order. Timestamps and directory sizes depend on the backing
filesystem. The manager exposes missing-ok working directories with a `!` prefix.
