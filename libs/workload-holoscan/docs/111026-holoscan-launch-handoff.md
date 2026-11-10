![Planned Console–Holoscan phase workflow](111026-holoscan-launch-workflow.svg)

# Running Holoscan through Console

Created November 10, 2026. Status: proposed integration; production implementation pending.
[Adoption overview](110926-holoscan-adoption.md).
[Alternative launch designs](111026-holoscan-launch-handoff.svg).

## Does it work today?

Console does not yet implement this Holoscan workflow.

The isolated test worked: the real Holoscan CLI called a temporary helper in
place of Docker. That helper received the Docker commands, saved them, waited for
permission to continue and changed selected mount paths.

The test used fake Docker, toolkit-version and display-authentication commands.
It launched no real Holoscan container and mounted no OverlayFS views.
There is no production `workload-holoscan` runner or helper yet.

## How the CLI would connect to Console

Holoscan normally runs Docker as a child program. Its existing
`HOLOSCAN_CLI_DOCKER_EXE` setting lets us point those calls at our helper.

The helper would have two modes:

| Mode | What happens |
| --- | --- |
| Prepare / Review | Run the CLI with builds disabled. The helper forwards image inspections, saves the generated `docker run` command and sends its settings back to Console. It then finishes the capture. |
| Start | Run the CLI again. The helper sends the current command to the backend and waits. The backend checks Review and prepares OverlayFS views. The helper applies those paths and ownership labels, then runs Docker. |

The backend supplies a container-identity file through the CLI's native Docker
options. The CLI and Console use that identity for stopping and cleanup.
The helper forwards output and exit status.

Console owns filesystem preparation, stop verification and cleanup. The shared
filesystem steps perform the same automatic change collection and write-back
work already used by Compose.

## Where the phases fit

Console's current Prepare request creates Review. Its Start request performs the
runtime setup and application launch.

Build remains a separate step before Prepare. A build that changes the image must
finish before the Review snapshot is saved.

The diagram shows the proposed additions inside those existing phases.
`PROJECT`, `APP`, `MODE`, `LANG` and `RUN` stand for the selected project,
application, mode, language and run directory.

Both calls to `holoscan run` disable image and application builds. During Prepare,
the helper saves the Docker command. During Start, it eventually executes that
command after Console has prepared the filesystem.

`--persistent` keeps the stopped container available for filesystem collection.
Console removes its owned resources after finalization. Before another run,
required cleanup stops any lingering owned runs; output salvage is best effort.

## What the test established

- The CLI supplied the complete Docker command to the helper.
- The helper could wait, change project/data mount sources and add a run label.
  The remaining arguments and container paths were preserved.
- Output and the injected exit status reached the caller.
- Cancellation worked before launch and triggered the native stop path after a
  simulated container identity was created.
- The Docker-mode override worked for a mode configured for local execution.
- Display authentication files existed during the call and were removed on CLI
  exit. Their paths changed between Prepare and Start.

Those temporary paths need separate handling when comparing Review with Start.
Image identity and the supported Docker argument forms also need production checks.

## Work still needed

- Implement the Holoscan runner and its helper in the Console backend.
- Share the existing filesystem operations while retaining each backend's mount
  and launch handling.
- Connect review settings, run events and results to Console's existing contracts.
- Test a real Docker application with OverlayFS, then GPU and sensor workloads.
- Test controller failure and verify that the next run cleans up leftovers.

The helper design remains under discussion. Production use of
`HOLOSCAN_CLI_DOCKER_EXE` and `HOLOSCAN_CLI_BUILD_LOCAL=0` requires explicit
configuration approval. This investigation added no Console configuration input.

## Evidence

Existing Console behavior:
[review and Start](../../../console/server/src/service.ts),
[native execution](../../../console/server/native/src/execution.rs),
[shared runtime sequence](../../workload-lifecycle/src/run.rs) and
[Compose filesystem preparation](../../workload-docker/src/workflows/recorded_compose/mod.rs).

Upstream behavior:
[Docker calls and temporary files][launcher],
[run flags][run] and [Docker/local selection][mode].

Applicable rules: D-01, D-03, D-04, D-05, D-06, D-07, D-09, D-12, D-13 and D-14.

[launcher]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/container/core.py
[run]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/run.py
[mode]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/utils/holohub.py
