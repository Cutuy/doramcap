![System app health](110226-system-app-health.svg)

# System app health and protobuf launches

Created: 2026-11-02. Updated: 2026-11-03. Status: complete; PR 96 is open for user review.
Updates [PR 96](https://github.com/Cutuy/doramcap/pull/96); leave it open for user review.

CPU monitor records itself. Watchdog aggregates selected health outputs. The generic
recorder stores selected user or system messages. App configs are complete standard
protobuf TextFormat messages, mounted read-only at stable container paths.

## Milestone 0: Standard protobuf configuration

**Work items**
- [x] Delete `pbtxt-config`; migrate watchdog and resource-policy to their generated domain types.
- [x] Remove includes, overlays, injected config defaults and config-relative path rewriting.
- [x] Supply explicit watchdog settings and recorder report paths in shipped pbtxt files.

**Validation**
- [x] Focused release-profile tests pass for watchdog, monitor, recorder and resource-policy without optional features.
- [x] Final feature-enabled tests and both Clippy configurations pass.
- [x] Reference docs and catalog links match the direct configuration contracts.

Evidence: initial focused Rust run passed 28 tests plus the recorder fixture; the
BPF-only monitor integration test was explicitly ignored in that build.

## Milestone 1: Watchdog inputs from Dora

**Work items**
- [x] Reserve the removed protobuf `inputs` field; read inputs from `node_config()`.
- [x] Validate source/frame budgets against actual Dora inputs and allow zero inputs.

**Validation**
- [x] Initial standalone and host tests pass with direct pbtxt.
- [x] Feature-enabled and default profile/budget tests pass; all 15 external source slots fit each enabled release profile.
- [x] Real Dora and Compose runs cover zero inputs and incoming health publications.

## Milestone 2: System apps publish health

**Work items**
- [x] Add one bounded reporter drain helper under the fault domain.
- [x] Monitor reports sampling deadlines and publication failures.
- [x] Recorder reports input closures/recovery and recording failures.
- [x] Declare health outputs in release and integration graphs and reuse helper in demo.

**Validation**
- [x] Failure/recovery/staleness publications survive real transport; independent MCAP readback validates 265 watchdog publications.
- [x] No-subscriber execution passes; official Dora CLI recording equality preserves all 24 CPU samples.
- [x] Dependency boundary checks pass.

## Milestone 3: Console wiring and three-app release

**Work items**
- [x] Wire selected `health` outputs using bounded `drop_oldest` queues.
- [x] Remove fixed `workload/faults`, project `flow.yml` discovery and associated Review blocking.
- [x] Pair generic recorder profiles with complete native Dora graphs and pbtxt configs.
- [x] Update PR 96 and leave it open for review.

**Validation**
- [x] Graph tests cover zero/one/multiple health sources and arbitrary user-message recording inputs.
- [x] Console server/frontend checks pass (77 server tests, one root-only skip; frontend tests/build pass).
- [x] All six real release-profile scenarios pass: alone, live, MCAP, OTLP, CPU topics and watchdog publications.
- [x] Full local CI passed: boundaries, formatting, default/all-feature Clippy and workspace tests, storage without defaults, examples/binaries and snapshot synchronization.
- [x] Hosted Node and Rust CI pass on x86_64 and ARM64 for implementation revision `bbd53c7`; final follow-up local checks pass.
- [x] ARM64 job 573 passes Node, lint, default/all-feature Rust and native accounting/allocation checks.
- [x] Final budget/UTF-8 snapshot job 574 passes; its Rust/protobuf/config hashes match the implementation.

Worker observation: started the unprivileged Pueue daemon after finding no socket.
ARM64 job 572 exhausted linker memory. The runner now builds with one Cargo job; retry 573 passed, including native tests. Final snapshot job 574 passed. Full CI and end-to-end checks passed.

Acceptance command (build the release binaries and examples unprivileged first):

```sh
sudo node --import ./console/server/node_modules/tsx/dist/loader.mjs \
  console/scripts/validate-system-apps.mts --dora /absolute/dora --bin-dir /absolute/target/release
```

This uses isolated test image tags and temporary projects; it retains acceptance
artifacts under the printed `/tmp/console-system-apps-*` directory for inspection.

Validation update: the native Dora equality check preserved all 24 CPU samples.
Direct zero-input watchdog execution stayed alive until explicit shutdown. Container
acceptance passed with the exact release-pinned Dora CLI archive and isolated image tags.

The first-party live bridge also receives a complete `BridgeConfig` pbtxt via a
read-only bind mount. Dora's third-party coordinator/daemon retain native CLI arguments.

Acceptance evidence: `/tmp/console-system-apps-dsGPy7` on x86_64. Every scenario
finished with clean boundaries and finalized recordings. User payload readback
preserved 92–93 byte-exact messages per scenario; the system-output profile read
343 watchdog messages. Live frames contained monitor, recorder and workload health;
the OTLP collector received records. The health/recording graph cycle ran cleanly.

The bundled `cpu-recorder` app is now `data-recorder`. Operator profiles pair a
complete recorder pbtxt with a complete native Dora graph; the `user/messages`
acceptance workload demonstrates this integration without a special message codec.

Final evidence for Milestone 3: [hosted Rust CI](https://github.com/Cutuy/doramcap/actions/runs/41762914364)
and [hosted Node CI](https://github.com/Cutuy/doramcap/actions/runs/41762914378) pass
on both architectures at `bbd53c7`. The subsequent documentation commit changes
no application or test source. PR 96 remains open and unmerged.

Additional compatibility evidence for Milestone 2: the existing demo reporter
produced 140 independently decoded watchdog MCAP publications through the direct
configuration loader. The native BPF/MCAP rotation test passed with all 24 samples
and zero loss. GPU qualification is outside this change.
