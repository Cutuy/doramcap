![System Monitor lifecycle MVP](102926-supervisor-monitor-mvp.svg)

# System Monitor through Supervisor

Created: 2026-10-29. Implementation complete. Latest follow-up: ARM64 validated; full local CI blocked by Linux disk space.

Run the existing `apps/system-monitor` binary through a first-party Supervisor
template. Exercise preparation, readiness, observation, cooperative stop,
recording finalization and verified cleanup using the same lifecycle driver as
Compose. One system app is the complete workload for this MVP.

## Scope

- Add `apps/system-monitor/deploy/supervisor/recording.conf`; keep usage in the existing app deployment README.
- Keep the existing Compose recipe at `deploy/recording.yaml`.
- Run one dedicated Supervisor daemon and one monitor program in an owned cgroup.
- Use a prebuilt monitor binary with BPF support and explicit host prerequisites.
- Defer foreign-config adoption, gallery integration, multiple apps, Dora,
  mixed backends, automatic environment installation and native filesystem overlays.
- Preserve ownership evidence on interruption. Automatic restart recovery is deferred;
  an unfinished owned run must block reuse until its cleanup is verified.

## Reuse boundary

| Component | Shared implementation | Backend implementation |
| --- | --- | --- |
| Execution | Existing `orchestrator` steps, events and dependency lineage | Step registration and adapter state |
| Recording | `Recording`, readiness probe, MCAP completion check | Monitor deployment recipe and output-path binding |
| Workflow | Prepare/launch/observe/stop/finalize/remove sequencing, cancellation and cleanup continuation | Concrete phase operations and evidence |
| Outcome | Execution error, writers stopped, recording finalized, cleanup verified | Docker diagnostics or Supervisor process diagnostics |
| Ownership | Admission rules and attempted-resource ledger contract | Docker resources or daemon/cgroup identities |

Extract recording code from `workload-docker::workflows::system_apps` into
`workload-lifecycle::recording`, retaining re-exports for existing callers.
Compose-specific preparation and `SystemAppsConfig` remain in `workload-docker`
unless a field genuinely becomes shared. Neutral error types must not depend on Docker.

Add one ordinary async recorded-run driver in `workload-lifecycle`. A small typed
phase interface binds each backend's steps to that driver. It owns phase order,
readiness notification, observation/cancellation, and best-effort completion.
Concrete wrappers retain their own run specifications and reports. Derive the
interface from these two callers; do not add a workflow DSL or universal service schema.

Compose hooks retain OverlayFS, Dora, Docker event capture and resource-removal
gates. A mandatory final cleanup hook closes background observers even when an
earlier phase fails. Failed finalization must not skip safe cleanup, and cleanup
must not discard evidence required by the backend. Preserve Compose's conservative
admission behavior and existing report fields.

## Proposed files

| Location | Purpose |
| --- | --- |
| `apps/system-monitor/deploy/supervisor/recording.conf` | Fixed first-party program recipe |
| `libs/workload-lifecycle/src/recording.rs` | Shared recording steps |
| `libs/workload-lifecycle/src/recorded_run.rs` | Shared lifecycle sequencing and outcomes |
| `libs/workload-supervisor/src/` | Native ownership, adapter and typed steps |
| `libs/workload-supervisor/proto/` | MVP run configuration and durable ownership evidence |
| `libs/workload-supervisor/examples/supervisor_monitor.rs` | End-to-end monitor run using generated protobuf settings |

The fixed template uses environment substitutions for the absolute binary and
run directory. Preparation creates a private daemon configuration with its own
Unix socket, PID file and log paths. Validate/escape generated values or reject
unsupported characters. No arbitrary INI importer is needed.

Program policy: `autostart=false`, `autorestart=false`, graceful TERM, bounded
stop wait, and process-group stop/kill. The runner explicitly starts the program
after establishing ownership. A monitor restart ends this recording attempt.
Supervisor priority does not replace the shared readiness barrier.

## Dependencies for evaluation

Reuse existing Tokio, prost, rustix/libc and orchestrator facilities where suitable.
Supervisor itself is the requested external runtime; orchestration remains Rust.

| Control option | Evaluation |
| --- | --- |
| Official `supervisorctl` | Smallest integration, no new Rust protocol dependency; verify timeout, cancellation and status semantics before choosing |
| `quick-xml` + existing HTTP transport | Recommended candidate for structured XML-RPC if CLI semantics are insufficient; requires a narrow protocol adapter and fault handling |
| `xmlrpc` | Dedicated protocol client candidate; evaluate current maintenance, async/cancellation and Unix-socket transport before adoption |

References: [Supervisor API](https://supervisord.org/api.html),
[Supervisor configuration](https://supervisord.org/configuration.html),
[quick-xml](https://github.com/tafia/quick-xml),
[xmlrpc](https://docs.rs/xmlrpc/latest/xmlrpc/).
Selected: official `supervisorctl`; existing Tokio bounds control calls and output,
kills/reaps on deadline, and detects monitor generation changes. No new protocol
adapter or third-party package/version was added. Qualified with Supervisor 4.2.5.

## Milestone 1: Deployment contract

**Work items**

- [x] Add the fixed monitor recipe and usage in the existing deployment README.
- [x] Define generated protobuf inputs: run identity, new output directory,
  supervisor/control executable locations, monitor binary, readiness/stop deadlines,
  and the delegated cgroup location. Use pbtxt for examples.
- [x] Match the Compose command: BPF backend, 32,768 records, `cpu.pb`, `cpu.mcap`,
  buffered recording. Retain logs and config/binary identity beside outputs.
- [x] Record the selected control approach and dependency evaluation.

**Validation**

- [x] Validate the template with the supported Supervisor version, including paths
  containing spaces and unsupported interpolation characters.
- [x] Reject missing binaries, unavailable BPF access or cgroup delegation before
  launching the monitor. No automatic sudo or host configuration mutation.
- [x] Confirm TERM reaches the existing monitor shutdown handler and closes MCAP.

Evidence: Supervisor 4.2.5 accepts paths with spaces; strict pbtxt/path tests reject
unsupported inputs. Real BPF preflight rejects missing privileges before launch,
and TERM produces readable MCAPs. Logs: `/tmp/supervisor-native-final.log`,
`/tmp/supervisor-unprivileged-bpf.log`, `/tmp/supervisor-preflight-denied.log`.

**Status:** complete.

## Milestone 2: Shared recording steps and lifecycle driver

**Work items**

- [x] Move `Recording`, `WaitForSystemApps` and `FinalizeSystemApps` behavior into
  the neutral crate, with compatibility re-exports and backend-neutral errors.
- [x] Extract the sequencing driver and adapt `ComposeRunner` to use it. Keep
  preparation, Docker events, filesystem and Dora work in Compose phase hooks.
- [x] Permit a system-app-only Compose run when a monitor is configured;
  continue rejecting runs with neither projects nor supporting apps.
- [x] Gate finalization on writer quiescence. Record execution, finalization and
  cleanup outcomes independently without weakening existing cleanup gates.

**Validation**

- [x] Contract tests cover successful phase order, readiness failure, cancellation
  during launch, quiescence failure and finalization failure with safe cleanup.
- [x] Verify no readiness notification after cancellation or failed probes.
- [x] Run existing Compose lifecycle/recording tests and a real monitor-only
  Compose run. Confirm existing app/Dora and overlay behavior is preserved.

Evidence: both wrappers use the shared driver and recording steps. Contract tests
cover ordering, failures and cancellation. Native Compose (5 tests), OverlayFS
(12 tests) and Dora parity (24 snapshots/26 messages) passed. Logs:
`/tmp/supervisor-compose-regressions-final2.log`, `/tmp/supervisor-overlay.log`,
`/tmp/supervisor-dora.log`.

**Status:** complete.

## Milestone 3: Supervisor steps on the shared driver

**Work items**

- [x] Add typed prepare, launch, inspect, quiesce and remove steps in
  `workload-supervisor`, registered on the existing executor.
- [x] Persist ownership intent before spawning; establish the daemon's cgroup
  before it can create descendants. Keep runtime handles out of serialized inputs.
- [x] Launch the monitor explicitly, invoke the shared recording readiness step,
  inspect liveness until stop, then quiesce and use shared recording finalization.
- [x] Bound control calls, stop the owned daemon, reap it and verify the cgroup is
  empty. Retain diagnostics and block reuse on unknown ownership or cleanup failure.
- [x] Add a thin Supervisor run wrapper and the System Monitor example using the
  same driver as Compose. Record how to diagnose an abandoned MVP run safely.

**Validation**

- [x] Run a real native monitor and read its completed MCAP with existing readers.
- [x] Inject startup failure, early monitor exit, stop timeout and manager-future
  cancellation. Verify attempted resources remain accountable and teardown runs.
- [x] Verify descendant cleanup, daemon reaping and two concurrent runs with
  separate sockets, cgroups and output directories.
- [x] Confirm forced termination never reports recording success for truncated MCAP.
- [x] Confirm incomplete ownership evidence blocks reuse without signaling unrelated PIDs.

Evidence: native tests cover real BPF/MCAP, concurrent runs, startup/early-exit
failures, forced stop, descendants, cancellation and blocked reuse. Ownership-save
regression verifies failed-save cleanup and preservation of existing evidence.
Terminal tests cover Ctrl-C and test-owned cleanup after manager SIGKILL.
Latest logs: `/tmp/supervisor-trim-native.log`,
`/tmp/supervisor-trim-arm.json`; see milestone 4 for current qualification.

**Status:** complete.

## Milestone 4: Demonstrate backend parity

**Work items**

- [x] Provide runnable Compose and Supervisor monitor-only examples with the same
  recording policy, cooperative stop and outcome summary.
- [x] Add a compact architecture/usage link and record the native host prerequisites.
- [x] Update library-boundary rules for the neutral extraction and new backend.

**Validation**

- [x] Compare semantic outcomes: advancing CPU stream, readable MCAP, graceful
  stop and verified cleanup. CPU sample values need not match across runs.
- [x] Compare emitted lifecycle phase order and shared recording-step events.
- [ ] Run final-revision local CI: boundaries, formatting, default/all-feature
  Clippy and workspace tests, storage without defaults, examples/binaries and
  snapshot synchronization, with CI profile and locked dependencies.
- [x] Before PR readiness, run final-revision ARM64 worker CI and the native
  Supervisor/System Monitor scenario. Report skipped/blocked checks separately.

Evidence: both examples use BPF, 32,768 records and buffered MCAP. Compose produced
35 snapshots/37 messages; Supervisor produced 36/38. Both passed shared phase-order,
recording and cleanup checks. Logs: `/tmp/supervisor-parity-summary.log`,
`/tmp/supervisor-mcap-parity.log`. Commands: [deployment README](../../../apps/system-monitor/deploy/README.md).

| Qualification | Latest result | Evidence |
| --- | --- | --- |
| Linux boundaries, format, both Clippy variants, storage without defaults | pass | `/tmp/supervisor-bugbot-ci-results.json` |
| Linux full workspace tests and examples/binaries | blocked/incomplete: disk exhaustion | `/tmp/supervisor-bugbot-ci-tests-default.log` |
| Linux targeted builds, 5 library tests, native BPF/MCAP and terminal cleanup | pass | `/tmp/supervisor-trim-native.log` |
| Linux snapshot synchronization | pass, 4 tests | `/tmp/supervisor-bugbot-linux-snapshots.log` |
| ARM64 full worker suite and native regressions | pass, job 511 | `/tmp/supervisor-trim-arm.json` |

Cargo checks use the CI profile and locked dependencies; builds run unprivileged.
ARM64 snapshot: `d7517c23d4994c6ea92439e9e1d1c709`. Earlier full local CI passed at
`eb14d07`; the current full local run requires additional Linux disk space.
Architecture review found no further defects in the one-app scope; resource-envelope
and dedicated control-output boundary coverage remain unverified.

**Status:** implementation and native qualification complete; full local CI blocked.
