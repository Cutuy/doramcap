# Holoscan runner: implementation plan

Created November 11, 2026. Status: draft; open items listed below need answers before milestone 2.
[Adoption overview](110926-holoscan-adoption.md) ·
[Launch design](111026-holoscan-launch-handoff.md).

Build `workload-holoscan` and connect it to Console's backend. A user can then
discover a project's applications, build one, review its launch, run it and get
its results. The web app journey is a later stage.

## Starting point

| Piece | State |
| --- | --- |
| Shared Docker crate, `docker-engine` | On `main`: Engine client, cleanup by label, filesystem views, baseline and change collection. |
| Shared run sequence | On `main`: `workload_lifecycle::run` with the `Running` trait. |
| Admission lock | Planned in [Cutuy/doramcap#315](https://github.com/Cutuy/doramcap/pull/315). Milestone 4 depends on it. |
| Console backend | Two operations today, inspect and start, each with a Compose and a systemd form. |
| Holoscan | Design documents only. Probed with the real CLI 5.0.0 and a fake Docker. |

## Agreed design this plan implements

| Area | Decision | Document |
| --- | --- | --- |
| Discovery | The CLI lists applications, languages and modes. | [Discovery](110926-holoscan-discovery.md) |
| Build | Its own Console request. Source commit and clean state recorded and compared later. | [Preparation](110926-holoscan-preparation.md) |
| Launch | Console prepares views and passes the view as the CLI's project root. Labels, limits and the log driver travel as Docker options. | [Launch](111026-holoscan-launch-handoff.md) |
| Review | One snapshot of the expected launch must match at Start. | [Launch](111026-holoscan-launch-handoff.md#review-and-start) |
| Supervision | The container is the source of truth, read through Engine events. | [Execution](110926-holoscan-execution.md#ready-and-ended) |
| Results | No recording check. Changed files and mounts outside capture are reported. | [Outputs](110926-holoscan-outputs.md) |
| Limits | Selected limits as launch options; conflicts found at Prepare. | [Controls](110926-holoscan-controls.md) |
| Logs | Host journal; a project's own log driver wins. | [Outputs](110926-holoscan-outputs.md#logs) |

## Crate

A new crate, `libs/workload-holoscan`.

| Depends on | For |
| --- | --- |
| `workload-lifecycle` | Run sequence, admission lock, shared run plan. |
| `docker-engine` | Engine client, cleanup by label, views, baseline, change collection, child process groups. |
| `orchestrator` | Steps and progress events. |

Boundary rule to add in `scripts/ci/boundaries.json`: `workload-holoscan` must
not depend on `workload-docker`, `workload-systemd` or `workload-supervisor`.
No new third-party crate is expected. Any that turns out to be needed is
presented for review first.

## Contract

A new package, `doramcap.holoscan.v1`, following the existing plan contracts.
Messages use plain fields and repeated messages.

| Message | Carries |
| --- | --- |
| `Selection` | Application, language and mode. |
| `HoloscanPlan` | Selection, extra data folders, selected limits and the filesystem policy. |
| `Catalog` | Applications with their languages and modes, plus diagnostics from the CLI. |
| `BuildRecord` | Selection, image ID, source commit and whether the tree was clean. |
| `ConfigurationSnapshot` | The reviewed launch: normalized command, image ID, project Docker options, mounts outside capture, limits and log setting. |
| `HoloscanRunReport` | Execution error, container exit, stop and cleanup facts, file-change counts and coverage, mounts outside capture. |

Console's execution contract gains a Holoscan form beside Compose and systemd in
`RunRequest`, `Inspection` and `StartRequest`, and two operations: catalog and build.

## Steps

Each is an orchestrator step with its own progress and failure.

| Step | Does |
| --- | --- |
| `DiscoverApplications` | Runs the CLI's list and modes queries in the project. Returns the catalog and diagnostics. |
| `BuildApplication` | Runs the CLI build under the admission lock. Streams output. Writes the build record. |
| `PrepareLaunch` | Checks the selection and the build record, runs the CLI dry run, reads the image ID and project Docker options, and produces the snapshot. |
| `ResetProject` | Stops and removes leftover containers by label through `docker-engine` and discards old views. |
| `LaunchApplication` | Starts the CLI against the views and waits on Engine events for the labelled container. Checks it against the snapshot. |

Baseline capture, view preparation, change collection and view finalization are
the existing `docker-engine` steps.

## Run sequence

The runner implements `Running` and calls `workload_lifecycle::run`.

| Phase | Work |
| --- | --- |
| Before `run` | Take the admission lock. Reset. Prepare and compare with the reviewed snapshot. Capture the baseline. Create views. |
| `start` | Subscribe to Engine events for the run label, then start the CLI. Wait for the container. Check image ID, view mounts and labels. |
| `ready` | Publish readiness once the container is running and checked. |
| `ended` | Resolve when the Engine reports the container stopped, with its exit code. |
| `stop` | Stop the container through the Engine with the run's deadline. Verify it stopped. Collect file changes. |
| `remove` | Finalize views by the filesystem policy. Remove the container when permitted. |

The run passes an empty recording list.

## Milestone 1: prove the assumptions on a real container

Nothing so far has run a real Holoscan container. This milestone is a set of
probes on a machine with an NVIDIA GPU and no production code.

**Work items**

- [ ] Build one Python and one C++ application from the Holoscan applications repository with the real CLI.
- [ ] Launch each with a real OverlayFS view as the project root.
- [ ] Launch with a label, a container-identity file, one CPU limit and the journal log driver as Docker options.
- [ ] Record the Engine events seen for the labelled container from create to stop.

**Validation**

- [ ] Build output is visible through the view at run time.
- [ ] The application runs correctly from a view, including display forwarding where the application needs it.
- [ ] Labels, the limit and the log driver are present on the created container.
- [ ] Container output appears in the host journal, and the Engine still returns the last lines.
- [ ] The Engine reports the application's own exit code.
- [ ] Stopping through the Engine ends the CLI process.
- [ ] Results are written into the [launch document](111026-holoscan-launch-handoff.md#probe-result). A failed assumption stops the plan for discussion.

## Milestone 2: crate, contract and discovery

**Work items**

- [ ] Create the crate, its protobuf package and the boundary rule.
- [ ] Implement `DiscoverApplications` with the five outcomes in the discovery document.
- [ ] Add the catalog operation to Console's backend.

**Validation**

- [ ] Catalog from the real CLI against the Holoscan applications repository.
- [ ] Empty folder, missing CLI and failed query each give their own outcome.
- [ ] Boundary check, formatting and Clippy.

## Milestone 3: Prepare and Review

**Work items**

- [ ] Implement `PrepareLaunch`: dry run, normalization of run-specific values, image ID, project Docker options.
- [ ] List mounts outside capture from the dry run and project options.
- [ ] Detect conflicts between selected limits or the log setting and the project's own options.
- [ ] Report "not built" when the image or build record is missing, and "rebuild needed" when the source commit or clean state differs.
- [ ] Add the Holoscan form to Console's inspect operation.

**Validation**

- [ ] Two Prepares with unchanged inputs give the same snapshot.
- [ ] A changed mode, image, project option or limit changes the snapshot.
- [ ] Each conflict and each "not built" case is reported with its cause.

## Milestone 4: Build request

Depends on the admission lock.

**Work items**

- [ ] Implement `BuildApplication` and the build record.
- [ ] Add the build operation to Console's backend, with progress and its own result.

**Validation**

- [ ] A real build succeeds and writes a record that Prepare accepts.
- [ ] A failed build keeps its output and blocks Prepare with a clear cause.
- [ ] A build is refused while a run holds the repository, and the reverse.
- [ ] An edit committed after Build makes Prepare report "rebuild needed".

## Milestone 5: run

Depends on the admission lock.

**Work items**

- [ ] Implement `ResetProject`, `LaunchApplication` and the `Running` handle.
- [ ] Wire baseline, views, change collection and finalization from `docker-engine`.
- [ ] Add the Holoscan form to Console's start operation, with the snapshot check before admission completes.
- [ ] Write the run report into the run's folder and map it to Console's execution result.

**Validation**

- [ ] A real application runs to its own exit and reports its exit code.
- [ ] Stop from Console ends the container within the deadline and collects file changes.
- [ ] A container that differs from the snapshot is stopped and Start fails.
- [ ] The CLI exiting before a container exists fails Start with the CLI's output.
- [ ] Killing the CLI mid-run leaves the run going and records a note.
- [ ] Files the application writes appear in the change report; the source tree is unchanged under the discard policy.
- [ ] Mounts outside capture appear in the result.

## Milestone 6: recovery and acceptance

**Work items**

- [ ] Cover controller loss during Build, during start and mid-run. The next run cleans up.
- [ ] Cover cancellation during start, failed stop and failed removal.
- [ ] Agree the acceptance projects and machines in stage 6 and run them.

**Validation**

- [ ] After each interruption, the next run on the repository starts cleanly.
- [ ] Unrelated containers and a live competing run keep their resources.
- [ ] Full local CI on the final revision and an ARM64 worker run at integration.
- [ ] GPU and sensor qualification on a CUDA-capable host, reported separately.

## Open items

These need an answer before the milestone that uses them.

| # | Question | Needed by | Proposal |
| --- | --- | --- | --- |
| 1 | How Console learns where the Holoscan CLI is. This is a new configuration input and needs explicit approval. | 2 | A field in the project's Console configuration naming the CLI executable. |
| 2 | Whether Console's own system apps (monitors, recorders) run alongside a Holoscan application. Compose and systemd start them inside their own workflow. | 5 | First version runs none and keeps the machine state capture, workflow events and file changes. |
| 3 | Where the build record is kept, since a build belongs to no run. | 4 | In Console's data folder, keyed by project and selection. |
| 4 | Which limits the first version offers. | 3 | The CPU set, matching what Compose offers per application. |
| 5 | How a tree with uncommitted changes at Build is treated. | 4 | Allowed and shown at Review as unverifiable. |

## Outside this plan

- The web app journey (stage 7).
- Showing file changes in the web app ([Cutuy/doramcap#312](https://github.com/Cutuy/doramcap/pull/312)).
- Collecting a run's log from the journal ([Cutuy/doramcap#314](https://github.com/Cutuy/doramcap/pull/314)).
- Host-native execution without Docker.
- Launching the container through the Engine API in place of the CLI.

## Tests

Scripted end-to-end checks are TypeScript. Rust tests stay small and cover
observable contracts. The Holoscan CLI is an upstream Python tool that the user
installs; Console runs it as an external program and adds no Python of its own.

Applicable rules: D-01, D-02, D-03, D-04, D-06, D-07, D-08, D-09, D-10, D-12,
D-13 and D-14 in the [design guide](../../../docs/design-guide.md); I-03, I-04,
I-05, I-06, I-07 and I-09 in the [implementation guide](../../../docs/impl-guide.md).
