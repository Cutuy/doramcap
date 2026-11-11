![Shared crate for apps as containers](111126-compose-apps-crate.svg)

# Shared crate for apps as containers

Created November 11, 2026. Status: boundary agreed; implementation not started.
[Implementation plan](111126-compose-apps-crate-impl.md).

Running Dora apps and Console's system apps as containers moves from
`workload-docker` into a shared crate, `compose-apps`. The Compose workflow and
the Holoscan workflow both use it. Each keeps only its own run order.

## Why

- The Holoscan workflow starts the same Dora and system apps as containers, then
  launches its own application through the Holoscan CLI. It needs the pieces and
  has its own order.
- The Compose workflow gains a named boundary: a run order on top of pieces with
  their own contracts.

## Boundary

| Moves to `compose-apps` | Contents | State today |
| --- | --- | --- |
| Project steps | Resolve, start, inspect, stop, quiesce and remove services in a Compose project. | A separate module with no references into the workflow. |
| Dora in containers | Start the Dora runtime, register the graph, stop it, and the Dora wiring. | Separate from the Compose workflow. |
| App preparation | Turn app groups into one configured project: merge, shared memory, cgroup placement, Dora wiring, pinned images and snapshot. | Inside the workflow module, taking the Compose run's whole input. |
| System apps: start, watch, stop | The ordered sequences for the system part of a run, and the rule for which container is removed last. | Inline in the Compose workflow. |

| Stays in each workflow | Compose | Holoscan |
| --- | --- | --- |
| Run order | Yes | Yes |
| User application | Start and stop the user's services. | CLI dry run, launch, supervision and stop. |
| Report and finalization | Yes | Yes |

## Rules of the boundary

- `compose-apps` depends on `docker-engine` and `workload-lifecycle`. It depends on
  no workflow crate.
- App preparation takes app groups: Dora apps, system apps and an optional extra
  group. Compose passes its user projects. Holoscan passes none.
- Watching takes the list of services to check.
- The start and stop order of the system part exists once, in the shared crate.

## Expected behavior

This is a restructuring. A Compose run behaves exactly as before: the same steps
with the same identities, the same Review snapshot for the same inputs, the same
saved records and the same report.

## Order

It follows the [admission lock](https://github.com/Cutuy/doramcap/pull/315) and
the [capture move](https://github.com/Cutuy/doramcap/pull/316), since all three
reshape `workload-docker`. It comes before the
[Holoscan runner](https://github.com/Cutuy/doramcap/pull/264) starts system apps.

Applicable rules: D-01, D-02, D-03, D-06, D-09, D-12, D-13 and D-14 in the
[design guide](../../../docs/design-guide.md).
