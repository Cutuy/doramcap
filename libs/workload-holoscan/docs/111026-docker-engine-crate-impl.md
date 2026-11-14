# Shared `docker-engine` crate: implementation plan

Created November 10, 2026. Status: complete; its three milestones are merged.
Architecture: [owners](110926-holoscan-outputs.md#owners).

This is the record of that move and keeps the words used at the time ("view",
"baseline"). The overlay code moves on to `host-filesystem` in
[Cutuy/doramcap#318](https://github.com/Cutuy/doramcap/pull/318); current terms are in the
[overlay plan](https://github.com/Cutuy/doramcap/blob/codex/console-file-change-design/console/docs/111026-file-change-report.md#terms).
[Adoption overview](110926-holoscan-adoption.md).

`libs/docker-engine` will hold the Docker operations that the Compose and Holoscan
backends share. `workload-docker` keeps the Compose parts and depends on it.

The work lands in three pull requests, one per milestone, in order. Each one
leaves Compose runs behaving as they do today.

## What moves

| Today in `workload-docker` | After |
| --- | --- |
| Docker client and socket inside `DockerAdapter` | `docker-engine`. `DockerAdapter` keeps its Compose startup state and wraps the client. |
| `compose/adapter/scope.rs`: stop, verify and remove by ownership label | `docker-engine` |
| `compose/overlay.rs`: namespace check, view creation, finalize, discard | `docker-engine` |
| `compose/overlay.rs`: manifest rewrite and Compose validation | Stays, inside the existing prepare step |
| `compose/filesystem.rs`: baseline, change collection, record readers | `docker-engine` |
| `proto/filesystem.proto` record schema | `docker-engine`, unchanged |
| `compose/adapter/project.rs`, `configuration.rs`, `command.rs` | Stay |
| Repository lock and reset step in `recorded_compose/reset.rs` | Stay |

## New input contract

The filesystem steps take these messages in place of `ResolvedComposeProject`.
They live in a new `proto/docker.proto`, package `doramcap.docker.v1`, with
generated Rust types. The messages use plain and repeated fields only.

| Message | Fields | Replaces |
| --- | --- | --- |
| `BindMount` | consumer, source, target, read_only, propagation, selinux | Binds read from the Compose manifest (`ResolvedBind`) |
| `VolumeMount` | consumer, name, target, read_only | Named volumes read from the Compose manifest |
| `OwnedContainer` | consumer, id | Containers found through the Compose project |
| `HostView` | source, view | The source-to-view map passed to change collection |

`consumer` is a display name chosen by the backend. Compose supplies
`project/service`, so saved change records keep their current text.

The step input structs keep their current serde form and embed the generated types:

| Step | Input today | Input after |
| --- | --- | --- |
| Baseline | artifacts, projects | artifacts, bind mounts, volume mounts |
| View creation | artifacts, workspace, projects, policy | artifacts, workspace, bind mounts, policy |
| Change collection | host views, baseline, projects | host views, baseline, containers |
| Finalize views | view set | view set |

## Compatibility

| Item | Decision |
| --- | --- |
| Step ID `compose.filesystem.prepare` | Kept. The step stays in `workload-docker`. |
| `doramcap.filesystem.v1` record files | Unchanged. |
| Consumer text in change records | Unchanged. |
| `overlays/prepared.json` | Shape changes: Compose projects leave it and mappings carry `consumer`. Only tests read it. |
| Leftover views from a run before the change | Discard reads the directory layout only, so cleanup still works. |
| `workload_docker::compose::DockerAdapter` | Kept. `project-explorer` and Console use it. |
| Public paths `workload_docker::compose::{overlay, filesystem}` and `filesystem_proto` | Removed in milestone 3. Callers move to `docker_engine`; no aliases. |

## Milestone 1 — Mount-based inputs in place

All changes stay inside `workload-docker`.

**Work items**

- [ ] Add `proto/docker.proto` with the four messages and generate its types.
- [ ] Split `overlay::prepare` into view creation, which takes bind mounts and
      returns trees and mappings, and a Compose function that rewrites and saves
      the execution manifests.
- [ ] Keep `PrepareOverlays` as the Compose step around both halves and Compose
      validation. A failure after view creation unmounts the new views, as today.
- [ ] Return the rewritten Compose projects from the step beside the view set, and
      update `execution_plan` and the finalize input.
- [ ] Replace `project` and `service` in `MountMapping` with `consumer`.
- [ ] Change baseline capture to take bind and volume mounts. Move manifest
      reading, its digest check and volume-name resolution to Compose preparation.
- [ ] Change collection to take containers. Compose lists its project's
      containers and keeps its ownership check; the stopped-container check stays
      with collection.
- [ ] Check what `overlays/plan.json` records and keep it free of Compose types.
- [ ] Update doc comments on every changed input and function.

**Validation**

- [ ] Existing unit tests in `compose/overlay.rs` and `compose/filesystem.rs` pass with the new inputs.
- [ ] `tests/overlay.rs` passes: isolation, shared aliases, deletion, write-back, full disk and interruption cases.
- [ ] `tests/docker.rs` filesystem cases pass, with `prepared.json` assertions updated.
- [ ] A new test covers a failed Compose validation after view creation: no view stays mounted.
- [ ] Change records from a Compose run carry the same consumer text as before.
- [ ] Formatting and Clippy pass for `workload-docker`.

## Milestone 2 — Separate Docker operations from Compose in place

Still inside `workload-docker`. The module layout matches the final crate, so
milestone 3 is a move.

**Work items**

- [ ] Add an `engine` module holding the Docker client, socket path and `connect`.
      `DockerAdapter` wraps it and keeps `connect`, `socket_path` and `project`.
- [ ] Add an engine error type for invalid input, Engine failures and I/O.
      `compose::Error` converts from it and keeps its current messages.
- [ ] Move `digest`, `private_write` and `create_private_dir` beside the code that uses them.
- [ ] Move the ownership labels and `scope.rs` into `engine`. The container
      removed last becomes an input in place of the Dora owner label.
- [ ] Move view creation, finalize, discard, baseline, collection and the record
      readers under `engine`. Their steps take the engine handle.
- [ ] Confirm that nothing under `engine` imports a Compose module.

**Validation**

- [ ] `recorded_compose/reset` tests pass, including a foreign reservation and leftover views.
- [ ] `compose/adapter` tests pass.
- [ ] All milestone 1 tests pass unchanged.
- [ ] Error text in a failing reset and a failing collection matches the text before the change.

## Milestone 3 — Move to `libs/docker-engine`

**Work items**

- [ ] Create `libs/docker-engine` with the `engine` module, `docker.proto`,
      `filesystem.proto` and a build script. Add it to the workspace.
- [ ] Carry over only the dependencies the moved code uses. `bollard` keeps its pinned version.
- [ ] Move `tests/overlay.rs` and the unit tests with the code.
- [ ] Make `workload-docker` depend on `docker-engine` and update its imports,
      step registration and tests.
- [ ] Update outside callers of the removed public paths.
- [ ] Update `scripts/ci/boundaries.json`: `docker-engine` depends on no
      `workload-*` crate, and every crate that forbids `bollard` also forbids `docker-engine`.
- [ ] Update CI steps that name the moved capability-required tests.
- [ ] Give the crate a minimal README and link it from `libs/README.md`.

**Validation**

- [ ] Full local CI on the final revision: library boundaries, formatting,
      default and all-feature Clippy and workspace tests, storage tests without
      default features, and snapshot synchronization tests.
- [ ] Capability-required overlay tests pass from the new crate.
- [ ] ARM64 worker run for the default and all-feature test sets.
- [ ] The move commit shows as renames with import changes only.

## Out of scope

- The Holoscan runner and its use of `docker-engine`. It gets its own plan.
- The repository lock. The Holoscan runner plan decides whether both backends share it.
- Renaming `workload-docker`.

Applicable rules: D-02, D-03, D-05, D-06, D-12, D-13 and D-14 in the
[design guide](../../../docs/design-guide.md); I-03, I-04, I-07 and I-09 in the
[implementation guide](../../../docs/impl-guide.md).

Source reviewed at revision `87ba9e2d`:
[overlay](../../workload-docker/src/compose/overlay.rs),
[filesystem](../../workload-docker/src/compose/filesystem.rs),
[adapter](../../workload-docker/src/compose/adapter.rs),
[ownership cleanup](../../workload-docker/src/compose/adapter/scope.rs),
[Compose run sequence](../../workload-docker/src/workflows/recorded_compose/mod.rs) and
[boundary rules](../../../scripts/ci/boundaries.json).
