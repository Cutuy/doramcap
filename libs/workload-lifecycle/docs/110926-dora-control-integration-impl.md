# Dora control integration implementation

Created: 2026-11-09. Updated: 2026-11-10. [Architecture](110926-dora-control-integration.md).
Parent: [standalone client PR #251](https://github.com/Cutuy/doramcap/pull/251).

This PR records the integration work. Compose and systemd changes ship as separate
adoption PRs, each with its own qualification. Implementation checkboxes below
remain open until that backend is changed and tested.

## Milestone 0: Capture the integration

**Work items**

- [x] Map current control process launches to the existing client API.
- [x] Record retained ownership, preparation parity and bounded cleanup.
- [x] Separate Compose and systemd adoption and their acceptance checks.
- [x] Map every workflow phase to retained client use, recovery and native cleanup in the workflow/recovery SVG.

**Validation**

- [x] Trace both backends against parent source `cb45f12b` and review applicable guide rules.
- [x] Check relative links, SVG XML/rendering and whitespace; visually inspect the rendered architecture.

Planning validation: source mapping, 17 relative links/anchors, SVG XML, rendered
layout and whitespace passed. Backend implementation and runtime qualification
remain pending in the two adoption PRs.

## Follow-up PR 1: Compose adoption

**Work items**

- [ ] Add a path dependency on the existing `dora-control` crate to `workload-docker`.
- [ ] Retain one client in [`ExperimentOwner`](../../workload-docker/src/workflows/experiment.rs); keep cleanup access through `ComposeRunning.resources` and existing [`register_experiment`](../../workload-docker/src/workflows/steps.rs) factories.
- [ ] Preserve [`shared_dora::prepare/graph`](../../workload-docker/src/workflows/shared_dora.rs): frozen hash, expanded dynamic descriptor and source/target bindings. Prepare one typed Start with the run name, session ID, runtime-visible working directory and current options; record required session artifacts before Start.
- [ ] Replace readiness and force shutdown in [`workflows/dora.rs`](../../workload-docker/src/workflows/dora.rs), plus registration, inspection and system-node stops in `experiment.rs`. Use retained client identity instead of duplicate `attempted`/`identity` fields.
- [ ] Establish the absolute startup deadline before native Dora startup; pass its remaining budget to native start and the same deadline to client calls. Preserve observation cadence and bound each observation using existing budgets.
- [ ] Preserve system-node grace and application drain order. Give cleanup a stop deadline, including a fresh budget after startup cancellation; continue container quiescence and independent exit verification after control failure.
- [ ] Delete `ExperimentOwner::cli`, CLI list/status JSON parsing and the backend readiness loop once the direct path is covered. Keep native service launch and endpoint inspection.

**Validation**

- [ ] Extend [`tests/docker.rs`](../../workload-docker/tests/docker.rs) for real Dora readiness, registration, observation, node-stop grace, recording completion and native runtime exit.
- [ ] Compare descriptor hashes, runtime-visible paths, Dora session identity/artifacts and reports with the current supported dynamic-graph path; reject changed frozen input before Start.
- [ ] Cancel before Start, after remote acceptance and after UUID receipt; hang/disconnect control. Verify bounded return, retained diagnostics and container teardown/removal rules.
- [ ] Run concurrent experiments, a subsequent run and reset after controller exit; verify independent endpoints/UUIDs, retained artifacts and released resources.
- [ ] Record Docker exec events for the full run and assert zero Dora control exec launches (`status`, `start`, `list`, `node stop`, `down`).
- [ ] Run focused locked release-profile checks, then required workspace/hosted CI and ARM64 qualification on the adoption revision; record native Dora results separately.

## Follow-up PR 2: systemd adoption

**Work items**

- [ ] Add a path dependency on the existing `dora-control` crate to `workload-systemd`.
- [ ] Retain the client in [`dora::Dora`](../../workload-systemd/src/dora.rs), owned by [`SystemdRunning.dora`](../../workload-systemd/src/running.rs), before the first Start await. Retain native unit handles independently through partial startup.
- [ ] Preserve generated-program and supplied-unit endpoint discovery, users, binds and runtime directories. Validate direct endpoint/auth access from the executor; remove CLI-only executable discovery when unused.
- [ ] Prepare typed Start from the captured supported dynamic graph. Preserve graph copy/hash, source/target bindings, session artifacts and the coordinator-visible working directory; use `build_id: None` for this supported graph form.
- [ ] Replace `await_daemon`, `register` and `down` CLI calls with client methods. Use one startup deadline across native Dora startup and control, and a stop deadline during reverse-phase teardown. Preserve systemd's existing stop-request monitoring policy.
- [ ] Continue native unit stops after timeout, disconnect or unresolved graph identity; retain control diagnostics and verify unit/process exit before removal.
- [ ] Delete `Dora::cli`, command counters, `cli-*.out` handling, `CLI_PREFIX` and its CLI-only name reservation/tests when unused. Retain `Manager::run_oneshot` for independent consumers.

**Validation**

- [ ] Extend [`tests/native.rs`](../../workload-systemd/tests/native.rs): `dora_runs_as_units_and_registers_the_graph` and `dora_app_units_resolve_their_native_executable_and_register_the_graph`.
- [ ] Exercise generated programs and supplied native units with real Dora; compare descriptor/session artifacts, paths, shared-memory access and reports.
- [ ] Cancel at the registration boundaries and hang/disconnect control; verify bounded cleanup, native unit exit and evidence/removal outcomes.
- [ ] Run concurrent scopes, a subsequent run and reset after controller exit; verify independent graph identity, unit ownership and released resources.
- [ ] Capture transient-unit creation throughout the run and assert zero `dora-cli-*` control units and no generated `cli-*.out` files.
- [ ] Run focused locked release-profile checks and required workspace/hosted CI/ARM64 qualification; build unprivileged and elevate only native test executables as documented in AGENTS.md.

Each adoption PR records completed/deferred checks here and updates its backend
documentation. Full local CI and worker/native runtime tests are deferred for this
documentation-only PR. The polling-removal TODO remains in the client plan.
