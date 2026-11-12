# Shared crate for apps as containers: implementation plan

Created November 11, 2026. Status: milestones 1–3 complete; milestone 4 in progress.
[Architecture](111126-compose-apps-crate.md).

Create `libs/compose-apps` and move four pieces into it from `workload-docker`,
one milestone each. Every milestone is a move with no behavior change. Milestones 1 to 3
can start now.

## Starting code

| Piece | Original location in `workload-docker` | Notes |
| --- | --- | --- |
| Project steps | `src/compose/` | Public. No references into `src/workflows/`. |
| Dora in containers | `src/workflows/dora.rs`, `shared_dora.rs`, `experiment.rs` | Public. One helper registers workflow steps alongside them. |
| App preparation | `src/workflows/recorded_compose/preparation.rs` and its `model` and `snapshot` modules | `resolve` takes `ComposeRun`. Uses the cgroup placement helper in `recorded_compose/cgroup.rs`. |
| System apps sequences | `launch_apps` and `monitor` in `recorded_compose/execution.rs`, `stop_apps` in `running.rs`, the owner rule in `reset.rs` | Inline, with user services handled in the same functions. |

## Compatibility

| Area | Rule |
| --- | --- |
| Step identities | Unchanged, including the `compose.*` names and recorded lineage. |
| Review snapshot | The same inputs give the same fingerprint before and after. |
| Protobuf | Packages and tags unchanged. Files move with their code. |
| Saved records | File names, layouts and formats unchanged. |
| Public names | Callers move to the new crate paths. No forwarding aliases. |
| Dependencies | No new third-party crate. |

Boundary rule to add in `scripts/ci/boundaries.json`. `compose-apps` depends on
`workload-lifecycle`, which is shared code, so the rule names each forbidden
crate.

| `compose-apps` must not depend on | Reason |
| --- | --- |
| `workload-docker`, `workload-systemd`, `workload-supervisor`, `workload-holoscan` | The concrete workflow crates. |
| `console-native` | Console's backend sits above it. |
| `mcap`, `storage-persistence`, `mcap-recorder` | `workload-docker` is held to these today, and the moved code keeps that limit. |

## Milestone 1: project steps

**Work items**

- [x] Create `libs/compose-apps` with its boundary rule.
- [x] Move `src/compose/` and its protobuf files. Update callers in `workload-docker` and Console's backend.

**Validation**

- [x] Existing project-step and adapter tests pass from the new crate.
- [x] Boundary check, formatting and Clippy.

## Milestone 2: Dora in containers

**Work items**

- [x] Move the Dora runtime, graph registration and wiring modules.
- [x] Split step registration so the workflow's own steps register in `workload-docker`.

**Validation**

- [x] Existing Dora wiring and runtime tests pass from the new crate.
- [x] A Compose run with Dora apps registers and removes its graph as before.

## Milestone 3: app preparation

**Work items**

- [x] Give preparation its own input: run identity, Dora apps, system apps, an
  optional extra group, shared-memory policy, recording writers and budgets.
- [x] Move `resolve`, the model and snapshot modules and cgroup placement.
- [x] The Compose workflow builds that input from `ComposeRun`.

**Validation**

- [x] Snapshot fingerprints for the existing fixtures are unchanged.
- [x] Preparation with no extra group yields a project of Dora and system apps only.
- [x] Cross-group checks still reject a system app that depends on a user service.

## Milestone 4: system apps start, watch and stop

Status: implementation in progress in [PR #324](https://github.com/Cutuy/doramcap/pull/324).
The capture move in [PR #318](https://github.com/Cutuy/doramcap/pull/318) is merged.

**Work items**

- [ ] Extract the start sequence: shared memory, Dora runtime, graph, system apps,
  wait for recordings.
- [ ] Extract watching, taking the services to check. Reuse the existing narrowing of an
  inspected project to a list of services.
- [ ] Extract the stop sequence: system apps, then the Dora runtime.
- [ ] Move the rule for which container is removed last. Each workflow keeps the reset
  order: stop the containers, discard the views, then remove the containers.
- [ ] The Compose workflow calls these, then starts and stops its user services around them.

**Validation**

- [ ] Start and stop order in a Compose run is unchanged, shown by recorded step order.
- [ ] A run with no user services starts, becomes ready, stops and cleans up.
- [ ] Reset after an interrupted run removes the shared-memory holder last.
- [ ] Full local CI and an ARM64 worker run at integration.

## Outside this plan

- The Holoscan workflow's use of these pieces.
- Any change to what a Compose run does.

Applicable rules: I-03, I-04, I-05, I-07 and I-09 in the
[implementation guide](../../../docs/impl-guide.md).

## Validation record

- Local x86_64: 84 unit/protobuf contract tests and four focused native Docker tests
  passed. Native checks cover Dora registration/cleanup under both shared-memory
  policies, cancellation, independent snapshots and preparation with no extra group.
- All targets compile; default/all-feature Clippy passes for `compose-apps`,
  `workload-docker`, Console, Project Explorer and Orchestrator Demo.
- Formatting, dependency boundaries, five boundary-validator tests and Console
  protobuf generation pass; generated Console output is unchanged.
- Snapshot hashes captured from `431d63bd` match. Native validation corrected stale
  IPC assertions and retained the launch descriptor used by Dora test fixtures.
- Full workspace CI and ARM64 were not run; they remain milestone 4's integration
  checks. Milestone 4 is outside this implementation.
