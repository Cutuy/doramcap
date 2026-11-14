![Overlays and the change list for Compose and Holoscan](110926-holoscan-file-changes.svg)

# Holoscan file changes and logs

Created November 9, 2026. Updated November 14, 2026: renamed from outputs; journal
export is merged. Status: agreed.
[Adoption overview](110926-holoscan-adoption.md). Terms follow the
[overlay term list](../../../console/docs/111026-file-change-report.md#terms).

A Holoscan run gets the same overlays, change list and write policy as Compose and
systemd runs. `workload-holoscan` names the host folders to overlay and passes their
overlay directories to the CLI; `host-filesystem` does the rest through the shared steps.

## What a launch mounts

| Storage | Overlay | Change list |
| --- | --- | --- |
| Project folder | Yes | From the upper |
| Extra data folder | Yes | From the upper |
| Mount set in the project's own Docker options | No | Not tracked |
| Docker named volume | No | Not tracked |
| The container's own filesystem | No | Docker's diff |
| Devices, host IPC, host networking | No | Not tracked |

Review and the result list every mount that is not tracked; Start is allowed.

The CLI places the project folder at `/workspace/holohub` and an extra data folder
at `/workspace/volumes/` plus its host folder's name, so each overlay directory
carries that name.

## Run sequence

1. Clean up earlier owned runs; leftover overlays are discarded.
2. Build. Its output lands in the lower before overlays are mounted.
3. Prepare saves the Review snapshot.
4. Start mounts the overlays and launches with the overlay directories.
5. Stop the application and read the uppers into the change list.
6. Save the report, unmount and apply the write policy: write back (default), keep
   with the run, or discard.

There is no recording check; the change list is the record of what the application
wrote. A recording cut short is not detected.

## Logs

| Case | Behavior |
| --- | --- |
| Project sets no log driver | Console sets Docker's journal driver as a launch option. |
| Project sets its own log driver | Kept; Review says so and Console collects nothing for that container. |
| Application writes its own log files | They appear in the change list when they land in an overlaid folder. |

At the end of a run, journal export saves each program's output with the run's
files, as for Compose and systemd. On failure, Console reads the container's last
lines through the Engine for the error message. The CLI's own output goes to a
launch log file.

The journal driver and reading back last lines are qualified in runner milestone 1.

Applicable rules: D-02 to D-04, D-06, D-07, D-09, D-11 to D-14. Upstream:
[container launcher][launcher].

[launcher]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/container/core.py
