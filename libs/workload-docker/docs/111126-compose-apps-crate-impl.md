# Shared crate for apps as containers: implementation plan

Created November 11, 2026. Status: design agreed; not started.
[Architecture](111126-compose-apps-crate.md).

Create `libs/compose-apps` and move four pieces into it from `workload-docker`,
one milestone each. Every milestone is a move with no behavior change. Milestones 1 to 3
can start now.

## Current code

| Piece | Location in `workload-docker` | Notes |
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

Boundary rules to add in `scripts/ci/boundaries.json`: `compose-apps` must not
depend on any `workload-*` crate or on `console-native`.

## Milestone 1: project steps

**Work items**

- [ ] Create `libs/compose-apps` with its boundary rule.
- [ ] Move `src/compose/` and its protobuf files. Update callers in `workload-docker` and Console's backend.

**Validation**

- [ ] Existing project-step and adapter tests pass from the new crate.
- [ ] Boundary check, formatting and Clippy.

## Milestone 2: Dora in containers

**Work items**

- [ ] Move the Dora runtime, graph registration and wiring modules.
- [ ] Split step registration so the workflow's own steps register in `workload-docker`.

**Validation**

- [ ] Existing Dora wiring and runtime tests pass from the new crate.
- [ ] A Compose run with Dora apps registers and removes its graph as before.

## Milestone 3: app preparation

**Work items**

- [ ] Give preparation its own input: run identity, Dora apps, system apps, an
  optional extra group, shared-memory policy, recording writers and budgets.
- [ ] Move `resolve`, the model and snapshot modules and cgroup placement.
- [ ] The Compose workflow builds that input from `ComposeRun`.

**Validation**

- [ ] Snapshot fingerprints for the existing fixtures are unchanged.
- [ ] Preparation with no extra group yields a project of Dora and system apps only.
- [ ] Cross-group checks still reject a system app that depends on a user service.

## Milestone 4: system apps start, watch and stop

Waits for the capture move in [Cutuy/doramcap#316](https://github.com/Cutuy/doramcap/pull/316) milestone 1.

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
