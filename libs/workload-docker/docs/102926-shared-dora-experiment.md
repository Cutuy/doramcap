![Current and planned containers and shared IPC](102926-shared-dora-containers.svg)

# Shared Dora daemon per experiment

Created: 2026-10-29. Status: M1–M4 complete; M5 retests pending; M6 planned. ARM64 deferred.
Acceptance requires **strictly fewer production-code LOC than main**.

## Contract

- Keep user-supplied `compose.yml` inputs. Assemble **one Compose project per
  repository**, with identity derived from its explicit canonical root; reject naming conflicts.
- Every experiment supplies coordinator/daemon; apps may ignore Dora.
- Steps own startup: **coordinator/daemon ready → system apps ready/stable →
  adopted user apps**. Control selected services within the shared project.
- System Monitor is an ordinary system app supplied through its deployment
  Compose file. Remove its special configuration, preparation and launch path.
- Every container joins one IPC namespace and the same `/dev/shm` backing store.
  Enable Zenoh SHM. Give all apps the same daemon connection settings; use is optional.
- Applications retain their Compose commands, mounts, dependencies and health checks.
  The Dora setup step owns communication services; workload steps own all app services.
- Stop adopted apps, stop system producers, await consumer drain, stop Dora, then
  finalize filesystems, remove resources and release machine-wide admission. Only
  one experiment runs at a time. Forced stops leave recording status incomplete.
- Preserve protobuf/pbtxt settings, native Dora graphs, immutable inputs, embedded
  recording and the MCAP writer's `seccomp=unconfined` setting.

## M1 — One project and shared IPC test: complete

Evidence (2026-10-30): `tests/shared_ipc.rs::one_project_qualifies_zenoh_shm_and_adopted_ipc`
passed locally in 13.05 s. Five containers shared their IPC namespace and shm mount;
20 payloads ≥256 KiB used `/dev/shm` and finalized MCAP in order (0–19).
An adopted app opened the same segment. Endpoint ownership rejection, interrupted
startup and verified cleanup passed. Dora 1.1 uses its daemon/TCP path for backpressure
routes; the SHM test uses `drop_oldest`. Recording routes retain backpressure.

**Work items**

- [x] Test one native Compose project with coordinator, daemon, a system
  producer/consumer pair and an adopted container. Start each group through steps.
- [x] Use a shareable IPC owner, join every app to it, size `/dev/shm` once and
  enable Zenoh SHM. Preserve matched permissions, memlock and backpressure.
- [x] Test this placement before replacing application launch paths.

**Validation**

- [x] All containers have the same IPC namespace and shared-memory backing;
  an adopted container sees and opens a segment created by a system container.
- [x] Prove actual Zenoh shared-memory payload delivery, retain the full accepted
  sequence and finalize MCAP. A successful TCP fallback does not satisfy this check.
- [x] Verify setup ordering, endpoint collision rejection, cancellation and cleanup.

## M2 — Ordinary apps and one resolved experiment: complete

Evidence (2026-10-30): native-input, concurrent-directory startup/recovery,
private input snapshots, image pins and conflict rejection passed. Mixed volume
removal and evidence preservation passed in 5.36 s. Gallery tests use System
Monitor through ordinary app inputs.

**Work items**

- [x] Resolve supplied Compose inputs with their own paths/environment, assemble a
  single experiment model and reject service/resource naming conflicts before launch.
- [x] Remove `CpuMonitor`, special `PrepareSystemApps` handling and hidden monitor
  injection. Supply System Monitor through the ordinary system app list.
- [x] Remove the extra participant roles/phases/bindings, host process manager and
  duplicated validation/recovery. Use native graph and Compose service identities.
- [x] Preserve private input snapshots and existing filesystem/removal gates.

**Validation**

- [x] Native Compose files, overrides, relative paths, environment, networks and
  volumes preserve their declared behavior; conflicts fail without side effects.
- [x] Inputs are immutable and images pinned. System Monitor follows the same
  preparation, launch, readiness and shutdown operations as another system app.

## M3 — Explicit startup, drain and recovery: complete

Evidence (2026-10-30): the public workflow suite passed in 101.35 s: slow startup,
20-message drain, cancellation, failed app start, daemon death, forced writer stop
and repeated crash recovery. Three additional cancellation runs passed after
joining pending Compose starts before cleanup. Partial-removal recovery reused
saved evidence; its stopped daemon reserved the directory until cleanup completed.
Forced stops retain recording files with incomplete status.

**Work items**

- [x] Launch selected Dora services, register the native graph, launch/stabilize
  system services, then launch user services. Await attempted starts on failure.
- [x] Supply daemon settings, default node IDs and IPC/SHM to every adopted app.
  Preserve declared IDs, commands, health checks and dependencies; Dora use is optional.
- [x] Apply the contract's shutdown order and reuse saved experiment identity for
  crash recovery; keep the IPC owner until its users have stopped.

**Validation**

- [x] Events prove daemon ready → every system app stable → user starts, including
  slow startup. Earlier-stage failure/cancellation prevents user startup.
- [x] Plain Compose apps and apps that ignore Dora run and stop normally.
- [x] Final accepted messages survive drain. Forced workload stop and daemon death
  report incomplete recording and await cleanup.
- [x] Repeated crash/partial-removal recovery preserves ownership and filesystem
  evidence; admission releases only after verified cleanup.

## M4 — Deployments and final acceptance: complete

Evidence (2026-10-30): all six gallery registrations passed in 58.11 s.
The local harness passed public startup/drain/failure tests, concurrent-directory
runs, stopped-owner exclusion, crash/partial-removal recovery, immutable inputs,
conflict rejection and mixed volume policies. Full local CI passed after rebasing
onto main `4f1bb84`: boundaries,
formatting, default/all-feature Clippy and workspace tests (86 executables each),
doctests, storage without defaults, examples/binaries, snapshot tests and Rustdoc.
ARM64 is deferred by request.
For the directory-discovery test edit, registry tests and Clippy passed;
full CI was deferred.

Full feature deltas versus pre-stack main `7966a1e`; inherited changes are excluded:

| Category | LOC delta |
| --- | ---: |
| Production Rust | −84 |
| Protobuf schema code | +71 |
| Production total | **−13** |
| Tests/tooling | Rust +2,831; Python +24 |
| Deployment/configuration, excluding schemas | −77 |
| Documentation | −675 |

Production counts include Rust and protobuf schemas, excluding comments/blanks,
tests and generated/vendor code. Other categories use nonblank lines.

**Work items**

- [x] Integrate CPU Viewer, MCAP Recorder, Fault Demo and Watchdog variants as
  ordinary app inputs. Preserve embedded recording, controls, viewers and results.
- [x] Remove stale deployment/runtime paths and align code docs and diagrams.
- [x] Replace admission/crash markers with Docker container ownership scoped to
  the canonical output directory. Remove the disabled-Dora branch.
  Create the owner before mounts/startup; retain it through stopped-process checks,
  saved evidence and resource cleanup.
- [x] Replace the stale local test launcher with the shared-Dora suite; retain
  failed-run resources and evidence for recovery.

**Validation**

- [x] All six gallery registrations pass live data, controls, downloads, finalized
  MCAP/OTLP outputs, join/idle-stop/restart and leak-free failed startup.
- [x] Production LOC is **strictly below current main** with identical counting:
  exclude tests, generated/vendor code, docs, blanks and comments; count every
  shipped helper. Moving/compressing code or deleting tests cannot satisfy this.
- [x] Report production, tests, deployment/configuration and docs separately;
  exclude unrelated inherited work.
- [x] Run final public workflow tests and full local CI from `AGENTS.md`, including
  affected Rustdoc. ARM64 is deferred.
- [x] Commit/rebase onto current main; preserve the active lease and inherited
  `Cargo.lock`.

## M5 — Generated Compose contracts

Evidence (2026-10-30): generated contracts pass protobuf text/binary and local-step
serialization tests, including uint64 counts and present-zero status. Missing
project/plan fields fail before I/O. Full local CI passes (87 executables per
workspace feature set). Startup/drain, concurrent-directory and actual Zenoh SHM
tests pass. Direct/partial-removal recovery retests hit native Compose launch
(10 s) and stop (22 s) deadlines; artifacts are retained. Observer-ID fixes remain
outside this change.
Volume cleanup (2026-10-30): full local CI and two Docker tests pass. Deletion
defaults off; incomplete volume scans retain data for every run. Serialization
tests cover protobuf text/binary and local-step defaults. ARM64 remains deferred.
Review regressions (2026-10-30): three ordinary tests fail before the #30 fix
and pass afterward: produced identity recovery, independent startup progress and
pinned-image command rejection. Cancellation and shared-waiter coverage also passes.
Compose merge (2026-10-30): omitted top-level fields warn and are saved per app.
Merge regression coverage and full local CI pass; ARM64 remains deferred.
Explicit launch (2026-10-30): `system_apps` readiness/stability precedes adopted apps.
Cancellation unit test, Docker startup/cancellation/barrier tests and full local CI pass.
Preparation rules (2026-10-30): twelve ordinary tests cover app policies, shared IPC,
graph/stage validation and frozen files. Full local CI and native input/startup tests pass.
Runtime setup (2026-10-31): default workers and no exact CLI-version gate. Build pins remain.
Full local CI and startup/drain pass; SHM passed before gate removal; ARM64 deferred.

**Work items**

- [x] Define the Compose model types in protobuf and use generated Rust directly.
- [x] Preserve launch, readiness, recovery and cleanup behavior; remove handwritten models.
- [x] Default volume deletion off; require complete saved volume-change reports before workflow deletion.

**Validation**

- [x] Test protobuf/step serialization and missing/invalid input rejection.
- [x] Test deletion permission defaults, per-volume policy and incomplete-scan retention without a recovery anchor.
- [x] Run startup/drain, concurrent-directory and shared IPC tests and full local CI.
- [ ] Recheck direct/partial-removal recovery after native Compose deadline failures.
  ARM64 remains deferred.

## M6 — Idempotent pre-run cleanup: planned

![Pre-run discard and normal teardown](103126-crash-handling-comparison.svg)

Design (2026-10-31): pre-run cleanup always discards previous uppers. It never
replays write-back or reconstructs old reports. Users salvage old data before starting.

**Work items**

- [ ] Use an explicit repository root for a stable Compose project; keep per-run outputs.
- [ ] Hold exclusive controller ownership across cleanup, execution and teardown;
  reserve the actual daemon under one machine-wide Docker container name.
- [ ] Add `ResetPreviousProject` before preparation: stop/verify owned writers,
  unmount owned overlays, delete old upper/work directories, remove disposable Docker
  resources and release the old reservation. Already absent resources are done.
- [ ] Reuse normal cleanup primitives. Keep copy-back for compatibility; pre-run
  cleanup selects discard regardless of the previous policy. Remove full-tree `Retain`.
  Future per-mount preservation selects copy-back; no new diff-export format.
- [ ] Reset leaves original host files, output directories and retained volumes alone.
  Interrupted copy-back may have changed host files; reset neither finishes nor undoes it.
- [ ] Use exact saved project/ownership labels and owned overlay paths. Delete the
  separate recovery engine/command, anchors, report reconstruction and naming inference.
- [ ] Keep Console hosting outside the experiment slot; inner runs and gallery
  experiments use the same workflow. Interrupted runs never become successful runs.

**Validation**

- [ ] Test concurrent starts within/across repos; only one experiment starts.
- [ ] Kill the launcher during startup, copy-back and removal. The next start discards
  remaining uppers without replaying host writes. Repeated reset is harmless.
- [ ] Test absent/partly removed resources, stopped writers and strict unmounts;
  failed cleanup blocks launch. Preserve host files, outputs and unrelated resources.
- [ ] Retest normal copy-back, volume policies, sequential Console and gallery runs.
- [ ] Show production LOC reduction; run focused checks first and full local CI on
  final source. ARM64 deferred. Implementation checks remain pending.
