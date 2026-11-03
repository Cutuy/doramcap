![System app health](110226-system-app-health.svg)

# System app health

Created: 2026-11-02. Status: proposed. Reworks [PR 96](https://github.com/Cutuy/doramcap/pull/96), which should wait for Milestone 3.

- **Publishing:** each system app reports its own health on a Dora output named `health`, with or without the watchdog.
- **Wiring:** Console links every selected `health` output to the watchdog when it is selected.
- **Projects:** using the watchdog requires nothing from a project.
- **Policy:** producers keep owning deadlines and recovery; the watchdog keeps owning aggregation and staleness.

## Confirmed constraints

Read from `dora-node-api` 1.1.0 and the code at `b78c671`.

| Fact | Effect |
|---|---|
| Dora inputs are fixed `node/output` links, with no topic subscription | Console generates the links, as `bindBridge` does for the live bridge |
| `node_config().inputs` lists a node's inputs at runtime | The watchdog can stop listing them in pbtxt |
| An undeclared output is ignored, with a one-time warning | A graph missing `health` still runs, so flows should declare it to avoid the warning |
| Only `fault-demo` and `fault-watchdog` use `fault-reporter` | System apps need new reporting code |
| `mcap-recorder` and `live-bridge` discard their Dora node handle | Publishing changes their event loops |

## Decisions to confirm

| # | Decision | Recommendation | Alternative |
|---|---|---|---|
| 1 | Finding health outputs | Output name convention: any selected node with an output named `health` | Declare `health_outputs` per app in `console.pbtxt` |
| 2 | Queue policy | `drop_oldest`, since health must never stall a producer and periodic snapshots repair losses | `backpressure`, as PR 96 uses today |
| 3 | Live bridge health | Defer: the watchdog would read the bridge while the bridge reads the watchdog, and that cycle is unverified | Include it and test the cycle |
| 4 | Missing-source blocking | Remove from PR 96, since selecting the watchdog can no longer block; reintroduce separately for `cpu-recorder` if wanted | Keep it for `cpu-recorder` |

**Scope limits**
- Project Dora nodes and the `flow.yml` merge leave this work; a project that needs them gets its own plan.
- The watchdog stays optional and unselected by default.

## Milestone 1: Watchdog inputs from Dora

**Work items**
- [ ] Delete `inputs` from `WatchdogConfig` and every watchdog pbtxt; reserve its protobuf field number and name.
- [ ] Connect to Dora before building the engine, and read inputs from `node_config()`.
- [ ] Apply the source and frame budget checks to those inputs.
- [ ] Allow zero inputs, so the watchdog can report only its local diagnostics.

**Validation**
- [ ] Config tests pass without `inputs`, and with and without `mcap,otel`.
- [ ] `gallery_dora_deployments` passes the `fault-demo` and `fault-watchdog` cases.
- [ ] A watchdog with no inputs stays running and publishes its local diagnostics.
- [ ] A node declaring more inputs than the budget allows fails at startup with a clear error.

## Milestone 2: System apps publish health

**Work items**
- [ ] Move the reporter draining loop from `fault-demo` into one shared helper.
- [ ] Add a `health` output to `system-monitor`, reporting sampling stalls and recording write failures.
- [ ] Add a `health` output to `mcap-recorder`, reporting input loss and write failures.
- [ ] Declare `health` in the release, gallery and test graphs for both apps.

**Validation**
- [ ] Each condition's failure and recovery appear in a real run, using a deliberately induced fault.
- [ ] With no subscriber, both apps run normally and memory stays bounded.
- [ ] Existing `system-monitor` and `mcap-recorder` tests, and the Dora recording equality check, pass.
- [ ] Boundary checks pass with the new `fault-reporter` dependencies.

## Milestone 3: Console wiring and release

**Work items**
- [ ] Add a `health_inputs` flag to `SystemAppDefinition`, set for the release watchdog.
- [ ] Generate that app's inputs in `graphs.ts` from selected nodes with a `health` output.
- [ ] Remove `workload/faults` from the watchdog graph, and "(needs workload/faults)" from its label.
- [ ] Remove the project `flow.yml` merge, `requireSources`, `MissingSource`, `selection_blocked` and their docs and tests.
- [ ] Rebase PR 96 onto Milestones 1 and 2, and rewrite its description.

**Validation**
- [ ] Graph tests cover zero, one and several health outputs, and deselected apps.
- [ ] Console server and frontend tests pass.
- [ ] On the `cpu-burn` project with no `flow.yml`, the watchdog can start with each profile, and health reaches the panel, `faults.mcap` and an OTLP collector.
- [ ] Stopping `cpu-recorder` mid-run marks its source stale in the Faults panel.
- [ ] The watchdog selected alone starts and shows its local diagnostics.
- [ ] Cloud CI passes; full local CI and the ARM64 worker run at this integration point, once the worker's out-of-memory failures are resolved or waived.
