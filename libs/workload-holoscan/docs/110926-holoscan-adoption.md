![Holoscan adoption architecture](110926-holoscan-adoption.svg)

# Holoscan adoption in Console

Created November 9, 2026. Updated November 10, 2026. Status: draft architecture; initial adoption scope agreed.

Console opens a Holoscan project, discovers its applications, runs a selected
application and retains the results. `workload-holoscan` owns this adoption
workflow. Holoscan remains responsible for the application's graph and sensor
processing.

## Component ownership

| Owner | Responsibility |
| --- | --- |
| Console | Project selection, review, run controls, results and comparisons. |
| `workload-holoscan` | Holoscan discovery, build, preparation, execution, recovery and output collection. |
| Holoscan project and tools | Application definitions, build rules, launch settings and graph execution. |
| Shared steps and subworkflows | Reusable host observation, file collection and other operations with matching contracts. |
| `workload-docker` | The existing Compose adoption workflow. |

`workload-holoscan` has its own domain model and lifecycle. It has no dependency
on Compose orchestration. Shared operations have an owner outside the Compose
workflow when both consumers need the same purpose, inputs, results and failure
behavior. Extract shared operations after identifying that concrete overlap.

## User workflow

1. Select a project and its configured Holoscan tools.
2. Choose an application, language variant, run mode and execution environment.
3. Review requirements, effective launch settings, resource controls and outputs.
4. Clean up lingering owned runs and verify the cleanup result.
5. Build through the existing Holoscan workflow as a separate operation.
6. Prepare the selected run using the built outputs.
7. Start the application with the selected Console observations.
8. Stop, collect results and inspect execution, recordings and cleanup outcomes.

Review and execution use the same resolved inputs. A changed project or environment
requires renewed preparation when it affects the reviewed run.

## Discovery and execution

A project is the source checkout and upstream configuration used to describe and
launch its applications. An installed SDK supplies development/runtime libraries;
the application catalog comes from the selected project.

Console delegates catalog discovery to the project's Holoscan tools. The catalog
identifies applications, implementations and named modes. Libraries and other
build-only entries remain distinguishable from runnable choices. Preserve the
project's native metadata and search configuration.

Discovery preserves empty catalogs, unavailable CLI capabilities and failed queries
as different outcomes. Dependency installation and application builds belong to the
explicit Build operation before Prepare.

**Docker is optional.** Holoscan supports both execution environments:

| Environment | Workload ownership |
| --- | --- |
| Docker | Application containers, associated owned resources and output files. |
| Host native | Application processes, associated owned resources and output files. |

The application and execution environment are separate choices. Initial adoption
targets Docker to simplify testing and reuse proven operations. Host-native
execution means building and launching application processes in the selected
environment; support belongs to a later stage.
[Upstream launch behavior][cli-run] and [catalog discovery][cli-discovery] establish
these boundaries.

## Existing placement and recording

First adoption preserves the application's Holoscan placement and recording controls.

| Capability | Holoscan responsibility | Console responsibility |
| --- | --- | --- |
| CPU and GPU placement | Internal worker placement, scheduling and GPU execution settings. | Observe the run and manage explicitly selected outer resource budgets. |
| Host allowance | Request resources within the available environment. | Show effective limits and known conflicts before execution. |
| Sensor recording | Existing application recorders, formats and capture settings. | Retain native files and report available recording outcomes. |
| Console observations | Application outputs available for observation. | Own its host telemetry, workflow logs and supporting services. |

One owner controls each placement setting and recorded output. An override or an
additional recorder is an explicit choice shown during review.

Compare declared Holoscan and Console settings where a reliable check is practical.
Warn about overlapping controls and fail preparation when the settings cannot work
together. Keep deeper inspection of application-internal placement and recording
as future work. Each supported check needs a clear boundary so users can see which
conflicts Console can detect.

Declared launch settings and application-provided execution descriptions explain
internal controls. Recording availability and recording validity are reported
separately.

## Run lifetime and recovery

`workload-holoscan` tracks resources from their first creation, including partial
startup. Stop and recovery operate on the resources owned by that run. Cancellation
and a controller interruption preserve ownership information for the next run's
repeatable cleanup. Lingering owned runs must stop before Build/Prepare; output
retention during recovery is best effort.

Stopping the application, finishing recorders, collecting files and confirming
cleanup have independent outcomes. Useful evidence remains available when another
part of the run fails. A repeated run receives a new identity.

The workflow delegates substantial machine operations to observable steps. Shared
subworkflows retain their lifecycle and failure meaning across callers. Helpers
and ordinary coordination stay within the owning operation.

## Design discussions

Discuss one stage at a time in the proposed order below. The agreed scope remains
the baseline. Later stages stay open until their discussion starts.

| Stage | Discussion | Decision needed to finish |
| --- | --- | --- |
| 1. [Discovery contract](110926-holoscan-discovery.md) | Existing CLI/project context, reported applications and diagnostics. | Adopt existing setup; retain valid entries and diagnostics; validate the selected application. |
| 2. [Build and preparation](110926-holoscan-preparation.md) | Separate Build from Prepare and preserve the selected native settings. | Build before Prepare; an isolated child Build operation is an accepted fallback. |
| 3. [Conflicting controls](110926-holoscan-controls.md) | Preserve native placement/recording and compare practical declared settings. | Warn or fail clearly for supported comparisons; defer deeper application inspection. |
| 4. [Execution and recovery](110926-holoscan-execution.md) | Owned run resources, repeatable pre-run cleanup and interruption recovery. | Stop lingering owned runs and verify cleanup before Build/Prepare; output retention is best effort. |
| 5. [Automatic filesystem capture](110926-holoscan-outputs.md) | Reuse the existing filesystem workflow, host observations and native recording outputs. | Automatic capture direction; qualify Holoscan mount integration and preserve existing coverage. |
| 6. Backend acceptance | Which existing projects and environments establish supported behavior. | Agree on comparison with upstream launches, failure scenarios and release evidence. |
| 7. Console UI | Present discovery, application selection, preparation, controls, diagnostics and results through the web app. | Agree on the complete web-app journey, saved selections, visible failure states and end-to-end checks. |

### Discussion process

- Start with the user workflow, expected outcomes and the unresolved decision.
- Present a recommendation and meaningful alternatives; wait for the user's answer.
- Keep a focused architecture document for each stage when its discussion starts.
  Lead it with an SVG and link it from this overview.
- Record agreed decisions and deferred work in that stage's document before moving on.
- Write the separate implementation plan after the stage architecture is agreed.
  Give each implementation milestone **Work items** and **Validation** checklists.

Source APIs, configuration fields, chosen mechanisms and individual workflow steps
belong in implementation plans. Share operations after establishing matching
purpose, inputs, results and failure behavior.

### Current discussion

Stages 1–4 record the agreed discovery, build, control and cleanup boundaries.
Stage 5 adopts the existing automatic filesystem capture workflow.

**Next: finish stage 5's launch and mount contract.** Establish how Holoscan
supplies its resolved container mounts and receives the prepared filesystem views,
and where the shared filesystem operations belong. The mechanism remains open.

Backend acceptance follows in stage 6. Console UI is explicitly last in stage 7,
including discovery presentation, saved selections, review, run controls and results.

## Agreed scope

| Decision | Initial scope |
| --- | --- |
| Crate boundary | Separate `workload-holoscan`; share proven operations at step/subworkflow level. |
| Execution | Docker first, with host-native support deferred. |
| Discovery | Projects described by the supported upstream Holoscan CLI. |
| Build | Separate operation before Prepare; retain its own outcome when nested in a preparation workflow. |
| Recovery | Repeat cleanup before the new run proceeds; stop lingering owned runs and attempt output retention. |
| Filesystem capture | Reuse automatic capture at shared workflow/step level; retain backend-specific mount integration. |
| Placement and recording | Preserve Holoscan controls; Console manages runs, its own observations and retained outputs. |
| Conflicts | Warn or fail clearly for supported checks of conflicting declared settings. Defer costly inspection of application internals. |

## Design basis

Applicable rules: D-01, D-02, D-03, D-04, D-06, D-07, D-08, D-09, D-10, D-12, D-13 and D-14 in the
[design guide](../../../docs/design-guide.md). Shared operations follow the
[workflow-step guidance](../../orchestrator/docs/102626-function-steps.md#choosing-workflow-steps).

- Upstream baseline: Holoscan CLI 5.0.0 project discovery and Docker/native launch paths.
- Runtime adoption, placement interaction and recording behavior remain to be qualified on the selected Docker projects.

[cli-run]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/run.py
[cli-discovery]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/info.py
