# Dora control integration implementation

Created: 2026-11-09. Updated: 2026-11-10. [Architecture](110926-dora-control-integration.md).
Parent: [standalone client PR #251](https://github.com/Cutuy/doramcap/pull/251).

This PR records the design. Shared preparation ships with Compose adoption;
systemd adoption reuses it. Runtime work and qualification checkboxes stay open.

## Shared preparation and identity

[`workload_lifecycle::dora`](../src/dora.rs) owns dynamic descriptor validation and
the typed `ControlRequest::Start` mapping. Move the common semantics currently in
[`shared_dora::graph`](../../workload-docker/src/workflows/shared_dora.rs) there;
inspection and execution use that same preparation. Backend code owns filesystem
copies, hash verification, source/target binds and session-artifact writes. Pass
resolved values into the shared mapping; introduce no generic backend framework.
Use the existing workspace-pinned Dora types and dependencies.

| Start field | Shared mapping |
| --- | --- |
| `dataflow` | Captured, expanded, validated dynamic descriptor; verify its frozen hash before sending. |
| `name` | Existing unique experiment run name, supplied explicitly. |
| `session_id` | One generated Dora session ID, shared with required session artifacts written before Start. |
| `local_working_dir` | Coordinator/daemon-visible working directory supplied by native path resolution. |
| `build_id` | `None`; supported graphs contain externally launched dynamic nodes. |
| `uv`, `write_events_to` | Preserve the current CLI path's effective defaults and artifacts; do not infer values from executor ambient state. |

The shared mapping does not parse Compose or systemd definitions. Backend adapters
resolve paths once and preserve the reviewed graph. Confirm CLI session-file and
option parity against the qualified Dora executable in the shared acceptance
fixture before deleting either CLI path.

The retained `DoraCoordinatorClient::graph()` is the only live start-intent/UUID
store. Compose removes `ExperimentOwner::attempted` and `identity` in the same
change that adopts the client. systemd removes `Dora::registered` in its adoption
change. Persisted reports hold evidence only; neither backend reconstructs a live
client from report fields.

## Endpoint and credential access

Construct `DoraCoordinatorClient` with an explicit socket address and optional
bearer token. Use host loopback and the resolved native coordinator port. Validate
that its native listen interface includes that address. Compose preserves its
required host networking. Generated systemd programs and supplied units must
share the executor's host network; reject incompatible `PrivateNetwork` or
`NetworkNamespacePath` settings before graph Start. Keep native users, binds and
root directories unchanged.

For an authenticated coordinator, read `.dora-token` from its resolved run-local
working directory after native startup. Compose maps it through the coordinator's
binds; systemd uses its resolved binds/root directory. Require a credential readable
by the executor under existing ownership and private-file permissions. Retain it
only in the client; exclude secrets from captured payloads, reports and logs.
Never fall back to the executor's home/config directory or ambient token discovery.
Existing native credential overrides must agree with this run-local credential or
fail access validation; this migration introduces no environment contract.

Token availability and authenticated connection share the startup deadline. Use
filesystem notification with subscribe-before-check if startup must await token
creation. Missing/unreadable credentials or failed authentication stop startup
before Start and still trigger native cleanup. Pass `None` only for a coordinator
whose supported native contract explicitly disables authentication; a missing token
file does not establish that mode. Qualify both modes against the actual executable.
Different users or isolated networks that prevent direct access need a separate
approved design; do not change permissions, impersonate users or launch a CLI proxy.

## Workflow mapping and deadlines

| Phase | Direct client use | Native responsibility |
| --- | --- | --- |
| 1. Prepare | Shared typed Start mapping; no control I/O. | Freeze graph, resolve paths/endpoints, write required session artifacts. |
| 2. Start Dora | `new(endpoint, token)`, `wait_for_daemon(startup_deadline)`. | Launch owned coordinator/daemon and validate direct access. |
| 3. Register graph | `start_graph(Start, startup_deadline)`. | Retain the client before awaiting; verify frozen input. |
| 4. Start applications | Keep the client retained. | Preserve system/user launch ordering and shared-memory attachment. |
| 5. Observe | Compose uses `inspect_graph(observation_deadline)`. | Preserve Compose cadence and systemd's stop-request monitoring policy. |
| 6. Drain applications | Compose uses `stop_node(node, grace, stop_deadline)`. | Preserve grace and drain order; systemd stops application units natively. |
| 7. Stop Dora | `shutdown(stop_deadline)`. | Always attempt native teardown and independently verify exit. |
| 8. Finalize and remove | Retain control diagnostics until reporting. | Apply recording/evidence and removal policy; retain unverified resources. |

Establish one absolute startup deadline before native startup; pass its remaining
budget to native launch and the same deadline to token access, readiness and
registration. Bound observation calls with existing budgets. Cleanup receives a
fresh stop budget after cancelled startup; node stops and control shutdown share
that stop deadline. Native teardown follows the backend's existing bounded stop
rules even when control has exhausted its budget.

Cancellation drops the in-flight socket while the retained client keeps intent
and any received UUID. Live cleanup uses `reconcile_graph` to recover one exact
run-name match. Zero or multiple matches remain unknown. `shutdown` sends Stop
before Destroy; a matching `DataflowStopped` permits Destroy even with node errors.
Uncertain Stop prevents Destroy. Preserve node/destruction errors together and
continue native teardown after every control failure. Force shutdown requires
exclusive ownership of coordinator and daemon.

The approved bounded 100 ms daemon-registration polling remains in the client.
Its [event-driven replacement TODO](../../dora-control/docs/110926-coordinator-client-impl.md#todo-replace-startup-readiness-polling)
is separate. The [migration roadmap](https://github.com/Cutuy/doramcap/pull/210)
records the existing lifetime and deadline decisions.

## Controller-exit recovery

Client state is process-local. A controller crash can leave an accepted graph
without a recoverable UUID; its execution outcome remains unknown. Recovery uses
native ownership, even if no graph identity was ever recorded:

- Compose acquires repository admission and runs [`ResetExperiment`](../../workload-docker/src/workflows/recorded_compose/reset.rs), which quiesces label-owned containers before removing runtime resources.
- systemd refuses a new run while prior resources remain. Explicit [`SystemdRunner::cleanup`](../../workload-systemd/src/runner.rs) acquires the scope lock and runs native removal, stopping owned services and verifying exit first.

Stop the owned coordinator/daemon and applications; never replay Start or connect
to a newly assigned endpoint to recover an old graph. Preserve artifacts, leave
unknown execution/recording outcomes unknown, and block admission/removal when
native exit is unverified. No new recovery journal, persistent UUID store or graph
resumption is introduced. Reconnect/reconcile applies only while the live owner
still retains its client.

## Milestone 0: Capture the integration

**Work items**

- [x] Assign shared Start preparation and live graph identity one owner each.
- [x] Specify direct endpoint/credential access and native reset after controller exit.
- [x] Separate architecture diagrams from the phase/API mapping and backend work.

**Validation**

- [x] Rebase onto `origin/main` source `b11bc398` and trace the client, both backends and native reset paths.
- [x] Check 21 relative links/anchors, SVG XML/rendering and whitespace; visually inspect both diagrams.
- [x] Recheck applicable implementation/design rules with Scanner and record coverage.

Planning checks pass. The architecture has 49 visible words; cleanup has 20.
Runtime behavior remains unimplemented here. Local full CI, ARM64 worker and real
backend qualification are deferred to the adoption PRs below.

## Follow-up PR 1: Compose adoption

**Work items**

- [x] Implement shared preparation in `workload-lifecycle::dora`, using the existing Dora dependency versions; preserve native graph/session semantics.
- [x] Add the existing `dora-control` dependency to `workload-docker`; retain one client in [`ExperimentOwner`](../../workload-docker/src/workflows/experiment.rs), with cleanup access through `ComposeRunning.resources` and [`register_experiment`](../../workload-docker/src/workflows/steps.rs).
- [x] Preserve [`shared_dora::prepare`](../../workload-docker/src/workflows/shared_dora.rs) hashes and binds; use shared graph/Start preparation. Resolve the run-local credential and direct endpoint before Start.
- [x] Replace readiness/shutdown in [`workflows/dora.rs`](../../workload-docker/src/workflows/dora.rs) and registration/inspection/node stops in `experiment.rs`; remove `attempted`/`identity` with those callers.
- [x] Apply shared deadlines and cancellation ownership; preserve application drain, grace, native quiescence and exit/removal verification.
- [x] Delete `ExperimentOwner::cli`, CLI list/status parsing and backend readiness polling after direct-path coverage. Keep native launch and endpoint inspection.

**Validation**

- [ ] Cover shared descriptor validation and every Start field with native graph/session inputs; verify the session ID/artifacts and CLI defaults agree across both backend fixtures.
- [ ] Extend [`tests/docker.rs`](../../workload-docker/tests/docker.rs) for real Dora readiness, registration, observation, node-stop grace, recording completion and verified native exit.
- [ ] Compare hashes, source/target paths, session artifacts and reports; reject changed frozen inputs before Start.
- [ ] Exercise run-local authentication, supported unauthenticated mode, missing/wrong/unreadable tokens and unsupported listen/network settings; assert no Start on access failure, preserved permissions and native cleanup.
- [ ] Cancel before Start, after remote acceptance and after UUID receipt; hang/disconnect control. Verify bounded cleanup, retained diagnostics and removal rules.
- [ ] Kill the controller after remote acceptance but before UUID receipt. Run repository reset; verify owned runtime exit, artifacts retained, no graph resumption and a subsequent clean run. Inject exit-verification failure and assert admission stays blocked.
- [ ] Run concurrent repositories and a subsequent run; verify independent endpoints, credentials/UUIDs and cleanup scope.
- [ ] Capture Docker exec events for the full run and assert zero Dora control exec launches (`status`, `start`, `list`, `node stop`, `down`).
- [ ] Run focused locked release-profile checks, then required workspace/hosted CI and ARM64 qualification on the adoption revision; record native Dora results separately.

Compose implementation: focused shared preparation/access tests (5) and locked release compilation pass. Real Docker/Dora, cancellation/crash, full CI and ARM64 qualification are in progress.

## Follow-up PR 2: systemd adoption

**Work items**

- [ ] Add the existing `dora-control` dependency to `workload-systemd`; retain the client in [`dora::Dora`](../../workload-systemd/src/dora.rs), owned by [`SystemdRunning.dora`](../../workload-systemd/src/running.rs), before the first Start await. Retain native unit handles through partial startup.
- [ ] Reuse shared descriptor/Start preparation; preserve graph copy/hash, binds, session artifacts and runtime-visible paths. Remove `Dora::registered` when adopting client identity.
- [ ] Resolve generated-program/supplied-unit endpoints and run-local credentials. Validate host network and executor access; preserve native users/binds/roots. Remove CLI-only executable discovery when unused.
- [ ] Replace `await_daemon`, `register` and `down` with client calls and shared deadlines. Preserve stop-request monitoring and reverse-phase teardown.
- [ ] Continue native unit stops after timeout, disconnect or unresolved graph identity; retain diagnostics and independently verify exit before removal.
- [ ] Delete `Dora::cli`, counters, `cli-*.out`, `CLI_PREFIX` and unused CLI-only name reservations/tests. Retain `Manager::run_oneshot` for independent consumers.

**Validation**

- [ ] Extend [`tests/native.rs`](../../workload-systemd/tests/native.rs): `dora_runs_as_units_and_registers_the_graph` and `dora_app_units_resolve_their_native_executable_and_register_the_graph`.
- [ ] Qualify generated programs and supplied units with real Dora; compare shared Start fields, graph/session artifacts, paths, shared-memory access and reports.
- [ ] Exercise credential and endpoint failures, including incompatible native users/network namespaces. Assert no Start, no permission changes and verified native cleanup.
- [ ] Cancel at registration boundaries and hang/disconnect control; verify bounded cleanup, exit, evidence and removal outcomes.
- [ ] Kill the controller after graph acceptance and before UUID receipt. Assert new-run refusal, explicit scope cleanup, preserved artifacts and a subsequent clean run; unverified exit must keep admission blocked.
- [ ] Run concurrent repository scopes; verify independent endpoints, credentials/UUIDs and native cleanup ownership.
- [ ] Capture transient-unit creation and assert zero `dora-cli-*` control units and no generated `cli-*.out` files.
- [ ] Run focused locked release-profile checks and required workspace/hosted CI/ARM64 qualification; build unprivileged and elevate only native test executables as documented in AGENTS.md.

Each adoption PR records completed/deferred checks here and updates its backend
documentation. GPU/IPC qualification requires its own capable host.
