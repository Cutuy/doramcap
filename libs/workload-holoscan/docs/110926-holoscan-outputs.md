![Shared automatic filesystem capture](110926-holoscan-outputs.svg)

# Holoscan observations and automatic filesystem capture

Created November 9, 2026. Stage 5 architecture: automatic capture and reuse direction.
[Adoption overview](110926-holoscan-adoption.md).

Adopt the existing automatic filesystem capture workflow for Holoscan. The backend
supplies the application's resolved mounts and uses the prepared filesystem views
when launching its containers.

## Reuse boundary

| Owner | Responsibility |
| --- | --- |
| Holoscan backend | Resolve its container mounts and apply the prepared host-source mappings to its own launch. |
| Compose backend | Preserve its manifest resolution and mount rewriting. |
| Shared filesystem operations | Prepare private views, collect file changes, finalize writes and report coverage. |
| Holoscan application | Continue its native sensor recording and file formats. |

Current filesystem steps accept Compose project inputs. Extract the common
operations behind mount-based inputs owned outside Compose orchestration.
Preserve their existing behavior and acceptance cases. The implementation design
will choose their shared location and concrete contracts.

## Run sequence

1. Clean up lingering owned runs.
2. Build through the existing Holoscan workflow.
3. During Prepare, resolve mounts and create the automatic capture views.
4. Start the application with those views at its expected container paths.
5. Stop writers, collect changes and finalize through the existing filesystem policy.

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

The [launch handoff discussion](111026-holoscan-launch-handoff.md) compares the
two ownership choices and the required Prepare/Start behavior.

The adapter must obtain the actual mount set and apply the prepared source paths
to the CLI's container launch. Catalog and mode responses cover discovery; mount
integration needs its own implementation design and qualification.

Qualify the resulting launch against native behavior, including project mounts,
additional data mounts, shared aliases and device/shared-memory access. Reuse the
current local Docker/OverlayFS requirements and existing failure behavior.

## Evidence and acceptance

Source inspected at repository revision `935212e1`:

- [Filesystem views and finalization](../../workload-docker/src/compose/overlay.rs).
- [Filesystem change collection](../../workload-docker/src/compose/filesystem.rs).
- [Output finalization ordering](../../workload-docker/src/workflows/recorded_compose/finalization.rs).
- [Existing overlay acceptance cases](../../workload-docker/tests/overlay.rs).

The [upstream container launcher][launcher] generates the native mounts and offers
a configurable Docker invocation point. The implementation discussion will select
and qualify the launch integration mechanism.

Carry the existing isolation, shared-alias, deletion, write-back, full-disk and
interruption cases into acceptance for both consumers. Extraction must preserve
Compose behavior. No implementation or runtime qualification has been performed.

Applicable rules: D-02, D-03, D-04, D-06, D-07, D-09, D-11, D-12, D-13 and D-14.

[launcher]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/container/core.py
