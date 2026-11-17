![Holoscan adoption architecture](110926-holoscan-adoption.svg)

# Holoscan adoption in Console

Created November 9, 2026. Updated November 17, 2026. Status: all seven backend
milestones implemented in the [PR stack](https://github.com/Cutuy/doramcap/pull/441); none are merged.

Console discovers, builds and runs applications from existing Holoscan Application
and Module projects and keeps the results. `workload-holoscan` owns this workflow.
Holoscan retains native project files and YAML, graph execution, CPU/GPU placement
and sensor recording.

## Documents

| Document | Covers |
| --- | --- |
| [Discovery](110926-holoscan-discovery.md) | The application catalog from the project's Holoscan CLI. |
| [Build](110926-holoscan-build.md) | The Build request and what Prepare checks against it. |
| [Launch](111026-holoscan-launch.md) | Native launch, verified Review facts and partial diagnostics. |
| [Run and recovery](110926-holoscan-run.md) | Admission, cleanup, ready and ended. |
| [File changes and logs](110926-holoscan-file-changes.md) | Overlays, the change list, the write policy and available logs. |
| [Runner plan](111126-holoscan-runner-impl.md) | Code-level design and milestones, including acceptance. |
| [Web app](111126-holoscan-web-app.md) | The journey and screens; planned after the run works. |

## Owners

| Owner | Responsibility |
| --- | --- |
| Console | Project selection, Review, run controls, results and comparisons. |
| `workload-holoscan` | Discovery, Build, Prepare, the run, recovery and its report. |
| Holoscan project and CLI | Application definitions, build rules, launch settings and graph execution. |
| `docker-engine` | Docker client, cleanup of owned resources and the container filesystem diff. |
| `host-filesystem` | Overlays, the change list and the write policy, shared with Compose and systemd. |
| `workload-lifecycle` | Shared run sequence, repository admission and helper ownership. |

`workload-holoscan` has no dependency on `workload-docker`.

## Decisions

| Area | Decision |
| --- | --- |
| Execution | Application Build/Run in Application and Module projects, through Docker first. Module defaults use the existing locked `toml` crate. |
| Workflow | Cleanup, Build, Prepare (Review), Start. |
| Discovery | The project's Holoscan CLI lists applications, languages and modes. |
| Build | Its own Console request. [Source facts and freshness](110926-holoscan-build.md#build-request) follow current Compose/systemd capture; only known mismatches require rebuild. |
| Admission | One lock per repository, shared by Compose, systemd and Holoscan runs and by Holoscan Build. |
| Launch | Console mounts overlays and passes the project folder's overlay directory to the CLI as its project root. |
| Container | The CLI builds and launches; Console stops, inspects and removes containers through the Engine API. Native Docker routing and Console must select the same local daemon. |
| Review | Requested selection, source facts, the recorded Build image verified through Engine and file fingerprints are rechecked at Start. Omitted native defaults and launch settings remain unknown. |
| Supervision | Engine events determine container readiness and completion. The Console Rust execution thread reaps CLI processes it spawns directly. |
| File changes | Shared overlays, change list and write policy. Observed mounts outside those overlays are untracked. |
| Recordings | Validate configured system-app recordings. Holoscan owns its sensor recording. |
| Shared memory | Apply the [existing run contract](../../workload-lifecycle/docs/110826-run-shared-memory.md): native host IPC requires explicit `PASSTHROUGH`; `BLOCKED` rejects it. |
| Logs | Preserve native logging configuration; save CLI output and available runtime log evidence. Initial Review leaves the log driver unknown. |
| System apps | Console's system apps, including Dora, run as containers beside a Docker project. |
| Configuration | `form: HOLOSCAN` requires `holoscan { cli }` with an absolute CLI path; other forms reject the block. Build/Run and their dry-runs set only child `HOLOSCAN_CLI_BUILD_LOCAL=0`. Docker routing and other inherited settings stay unchanged. |
| Acceptance | The unchanged official SDK `first_pipeline` covers Build, Review, Run, system apps and recovery in Application and Module projects. No replay dataset, model or display. |

## Order of work

![Order of work](111126-holoscan-work-order.svg)

Shared admission, apps, filesystem and journal prerequisites are merged.
The backend implementation is in the following open PRs; the
[stack index](https://github.com/Cutuy/doramcap/pull/441) includes their prerequisites.

| Milestone | Pull request |
| --- | --- |
| 1: lightweight lifecycle | [#444](https://github.com/Cutuy/doramcap/pull/444) |
| 2: catalog and configuration | [#443](https://github.com/Cutuy/doramcap/pull/443) |
| 3: limited Prepare/Review | [#448](https://github.com/Cutuy/doramcap/pull/448) |
| 4: Build | [#458](https://github.com/Cutuy/doramcap/pull/458) |
| 5: application Run | [#465](https://github.com/Cutuy/doramcap/pull/465) |
| 6: system apps | [#464](https://github.com/Cutuy/doramcap/pull/464) |
| 7: recovery and acceptance | [#466](https://github.com/Cutuy/doramcap/pull/466) |

The web app keeps its separate plan.

Reuse the existing repository/run hierarchy with system and user application
groups and repeatable owned-resource cleanup. Capture and write-back run once.
Initial Review uses verified facts and partial diagnostics. Existing mandatory
recording and IPC constraints remain in force; unknown evidence cannot satisfy
them. After controller loss, reuse the existing application cgroup and one
repository recovery record to stop surviving commands before resource reset.

Applicable rules: D-01 to D-04, D-06 to D-10, D-12 to D-14 in the
[design guide](../../../docs/design-guide.md). Upstream baseline: Holoscan CLI 5.0.0
([run][cli-run], [discovery][cli-discovery]).

[cli-run]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/run.py
[cli-discovery]: https://github.com/nvidia-holoscan/holoscan-cli/blob/v5.0.0/src/holoscan_cli/commands/info.py
