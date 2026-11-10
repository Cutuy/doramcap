![Planned Console–Holoscan phase workflow](111026-holoscan-launch-workflow.svg)

# Running Holoscan through Console

Created November 10, 2026. Status: launch design agreed; CLI probe and production implementation pending.
[Adoption overview](110926-holoscan-adoption.md).
[Launch designs compared](111026-holoscan-launch-handoff.svg).

## Does it work today?

Console does not yet implement this Holoscan workflow. There is no production
`workload-holoscan` runner.

The agreed design below has not run. It depends on two CLI behaviors listed under
[Pending probe](#pending-probe). An earlier isolated test covered the fallback
design only.

## How Console steers the CLI

Console prepares the filesystem views before it calls the CLI. It then passes the
view to the CLI as the project root. The CLI builds its usual Docker command, and
the project mount points at the view.

| Need | How it is met |
| --- | --- |
| Captured project files | The CLI receives the OverlayFS view as its project root. |
| Ownership labels | Passed through the CLI's native Docker options. |
| Container identity | A container-identity file, passed through the same options. |
| Stopped container kept for collection | The CLI's native `--persistent` flag. |

Console sets no environment variable for the CLI and places no program between
the CLI and Docker.

Mounts that a project declares in its own Docker options keep their original
source. Console reports them as outside capture coverage.

## Who owns the container

| Owner | Responsibility |
| --- | --- |
| Holoscan CLI | Discovery, build, resolving the launch and starting the container. |
| Console, through the Docker Engine API | Finding owned containers by label, inspection, events, stop, file changes and removal. |

Console stops the container itself with its own deadline, as the Compose backend
does. The CLI process ends when the container ends. The runner follows the shared
run sequence in `workload-lifecycle`: start, observe, stop, check recordings, remove.

The Engine API client is the one `workload-docker` already uses.

## Review and Start

Review saves one snapshot of the expected launch. Start requires the same
snapshot, as Compose and systemd do.

The snapshot covers:

- The Docker command from the CLI's dry run.
- The image ID read through the Engine API.
- The project's configured Docker options, which the dry run hides.

Run-specific values are replaced with fixed names before hashing: view paths, the
ownership label and the container-identity file.

After the CLI creates the container, Console checks the facts it owns: the image
ID, the view mounts and its labels. A mismatch stops the container.

Known limits:

- The CLI lists devices at each call. A device added after Review requires a new Review.
- The CLI forwards some environment variables by name. A changed value is not detected.

## Where the phases fit

Console's current Prepare request creates Review. Its Start request performs the
runtime setup and application launch.

Build remains a separate step before Prepare. A build that changes the image must
finish before the Review snapshot is saved.

The diagram shows the proposed additions inside those existing phases. `PROJECT`,
`VIEW`, `APP`, `MODE`, `LANG`, `OWNER` and `RUN` stand for the selected project, its
filesystem view, application, mode, language, ownership label and run directory.

Every call to `holoscan run` disables image and application builds. Prepare uses
the dry run. Start launches with the view as the project root.

Console removes its owned resources after finalization. Before another run,
required cleanup stops any lingering owned runs; output salvage is best effort.

## Designs compared

| Design | Outcome |
| --- | --- |
| Console steers the native CLI | Chosen. Fewest new parts and no environment setting. |
| Docker stand-in between the CLI and Docker | Fallback if the probe fails. Sees the exact command before launch. Needs a parser for Docker arguments and approval for `HOLOSCAN_CLI_DOCKER_EXE`. |
| Console launches from an exported plan | Set aside. Console would repeat the CLI's device, display and GPU setup and track it across releases. |

A machine-readable launch plan from the CLI would make the third design cheap.
CLI 5.0.0 offers none.

## Pending probe

The probe uses the real CLI and a fake Docker. It answers:

- Does the application see the same container path when the project root is a view?
- Does the dry run give the same command on two calls for an unchanged host?

A failed first check moves the design to the fallback. A failed second check
reopens the Review rule.

## What the earlier test established

The earlier test covered the fallback. The real CLI called a temporary helper in
place of Docker, with fake Docker, toolkit-version and display-authentication
commands. No real container ran and no OverlayFS view was mounted.

- The CLI supplied the complete Docker command to the helper.
- The helper could wait, change project and data mount sources and add a label.
- Output and exit status reached the caller.
- Cancellation worked before launch and triggered the native stop path afterward.
- Display authentication paths changed between two calls.

## Work still needed

- Run the probe and record its result here.
- Implement the Holoscan runner in the Console backend.
- Move the shared Docker operations to their own crate, as the
  [filesystem capture document](110926-holoscan-outputs.md) describes.
- Connect review settings, run events and results to Console's existing contracts.
- Test a real Docker application with OverlayFS, then GPU and sensor workloads.
- Test controller failure and verify that the next run cleans up leftovers.

## Evidence

Existing Console behavior:
[review and Start](../../../console/server/src/service.ts),
[snapshot check at Start](../../../console/server/native/src/execution.rs),
[shared run sequence](../../workload-lifecycle/src/run.rs),
[Compose snapshot](../../workload-docker/src/workflows/recorded_compose/preparation/snapshot.rs) and
[Compose filesystem preparation](../../workload-docker/src/workflows/recorded_compose/mod.rs).

Upstream behavior:
[Docker calls, dry run and temporary files][launcher],
[run flags][run] and [Docker/local selection][mode].

Applicable rules: D-01, D-02, D-03, D-04, D-05, D-06, D-07, D-09, D-12, D-13 and D-14.

[launcher]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/container/core.py
[run]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/run.py
[mode]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/utils/holohub.py
