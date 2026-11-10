![Shared automatic filesystem capture](110926-holoscan-outputs.svg)

# Holoscan observations and automatic filesystem capture

Created November 9, 2026. Updated November 10, 2026. Stage 5 architecture: automatic capture agreed; shared owner agreed.
[Adoption overview](110926-holoscan-adoption.md).

Adopt the existing automatic filesystem capture workflow for Holoscan. The backend
prepares the filesystem views first and passes the view to the Holoscan CLI as its
project root.

## Reuse boundary

| Owner | Responsibility |
| --- | --- |
| Holoscan backend | Name the folders to capture, pass their views to its own launch and supply its container IDs. |
| Compose backend | Preserve its manifest resolution, mount rewriting and Compose validation. |
| Shared Docker crate | Docker client, cleanup of owned resources by label, private views, file-change collection, write finalization and coverage. |
| Holoscan application | Continue its native sensor recording and file formats. |

## Shared Docker crate

A new crate, `docker-engine`, holds the Docker operations both backends need.
`workload-docker` keeps the Compose parts and depends on it, as
`workload-holoscan` does. The [implementation plan](111026-docker-engine-crate-impl.md)
lists the milestones.

Current filesystem steps accept Compose project inputs, so the work has two stages:

1. Change the step inputs in place to mounts and container IDs. Compose keeps
   reading its manifest and rewriting it. Existing acceptance cases keep passing.
2. Move the code to the new crate with no behavior change.

| Compose phase | Stays in `workload-docker` | Shared Docker crate |
| --- | --- | --- |
| Reset | Repository lock and the reset step. | Stop, verify and remove resources by ownership label; discard leftover views. |
| Prepare | Resolve and pin the manifest; read its binds and volumes. | Record the file baseline. |
| Views | Rewrite manifest sources; Compose validation. | Create views; return each source and its view. |
| Start, stop, remove | Compose project operations. | None. |
| Finalize | List the project's containers. | Collect file changes; write back or discard; close views. |

`workload-docker` keeps its own prepare step around view creation and manifest
rewriting. A failed Compose validation discards the new views, as today, and the
step keeps its current identity.

Details settled in the implementation plan:

- The new step inputs are protobuf messages.
- Mount records name a Compose project and service today. They become a plain consumer name.
- Reset removes the container holding the shared-memory reservation last. The shared operation takes that as an input.
- The saved view record drops its Compose projects. Only tests read it.

## Run sequence

1. Clean up lingering owned runs.
2. Build through the existing Holoscan workflow.
3. Prepare saves the Review snapshot.
4. Start creates the automatic capture views.
5. Start runs the application with the view as its project root.
6. Stop writers, collect changes and finalize through the existing filesystem policy.

Build completes before the application's capture views are created. The native
application writes its files through the prepared mounts. Console retains its
host observations, workflow logs and the filesystem results under the same run.

## Existing coverage to preserve

| Storage | Existing behavior |
| --- | --- |
| Supported host directory mounts | Per-run OverlayFS views carry writes; finalization applies the selected write-back or discard policy. |
| Container writable layers | Docker supplies added, changed and deleted path metadata. |
| Supported named volumes | Before/after metadata reports identify changes; volume preservation follows the current lifecycle policy. |
| Device paths and live shared memory | Native access and the existing coverage rules remain explicit in the launch plan. |

Results identify storage categories, retained paths and capture coverage. Native
recording validity and filesystem change reports keep separate outcomes.

Normal finalization owns write-back. Cleanup of an interrupted prior run stops
its writers and discards temporary views after any best-effort salvage.

## Holoscan integration work

The [launch document](111026-holoscan-launch-handoff.md) records the agreed
launch design and the designs it was compared with.

Captured folders reach the container through inputs Console controls: the
project root and extra data folders. The view of an extra data folder keeps the
folder's name, because the CLI derives the container path from it. Mounts declared
in a project's own Docker options keep their original source and are reported as
outside capture coverage.

The default launch also shares host networking, host IPC and `/dev`. These pass
through to the host unchanged.

Qualify the resulting launch against native behavior, including project mounts,
additional data mounts, shared aliases and device/shared-memory access. Reuse the
current local Docker/OverlayFS requirements and existing failure behavior.

## Evidence and acceptance

Source inspected at repository revision `935212e1`:

- [Filesystem views and finalization](../../workload-docker/src/compose/overlay.rs).
- [Filesystem change collection](../../workload-docker/src/compose/filesystem.rs).
- [Output finalization ordering](../../workload-docker/src/workflows/recorded_compose/finalization.rs).
- [Existing overlay acceptance cases](../../workload-docker/tests/overlay.rs).

The [upstream container launcher][launcher] generates the native mounts from the
project root it receives.

Shared-crate boundary reviewed against
[ownership cleanup](../../workload-docker/src/compose/adapter/scope.rs),
[Compose project operations](../../workload-docker/src/compose/adapter/project.rs) and the
[Compose run sequence](../../workload-docker/src/workflows/recorded_compose/mod.rs).

Carry the existing isolation, shared-alias, deletion, write-back, full-disk and
interruption cases into acceptance for both consumers. Extraction must preserve
Compose behavior. No implementation or runtime qualification has been performed.

Applicable rules: D-02, D-03, D-04, D-06, D-07, D-09, D-11, D-12, D-13 and D-14.

[launcher]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/container/core.py
