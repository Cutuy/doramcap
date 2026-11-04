![System app health](110226-system-app-health.svg)

# System app health and protobuf launches

Created: 2026-11-02. Status: complete. [PR 96](https://github.com/Cutuy/doramcap/pull/96).

CPU monitor records itself. Watchdog aggregates health. The generic recorder stores
user or system messages. Apps receive complete protobuf TextFormat configs through
read-only mounts.

## Milestone 0: Standard protobuf configuration

**Work items**

- [x] Delete `pbtxt-config`; load generated watchdog and resource-policy messages directly.
- [x] Remove includes, overlays, injected defaults and config-relative path rewriting.
- [x] Make watchdog settings and recorder report paths explicit.

**Validation**

- [x] Default and feature-enabled configuration tests pass.
- [x] Clippy passes; API docs and catalog links match the configuration contracts.

## Milestone 1: Watchdog inputs from Dora

**Work items**

- [x] Reserve the removed `inputs` field; read inputs from `node_config()`.
- [x] Validate source/frame budgets and allow zero inputs.

**Validation**

- [x] Profile and budget tests pass.
- [x] Dora and Compose runs cover zero inputs and incoming health.

## Milestone 2: System apps publish health

**Work items**

- [x] Share bounded reporter draining with the demo.
- [x] Report monitor sampling/publication failures and recorder input/recording failures.
- [x] Declare health outputs in release and integration graphs.

**Validation**

- [x] Failure, recovery and staleness survive transport and MCAP readback.
- [x] No-subscriber runs, recording equality and BPF/MCAP rotation pass.
- [x] Demo compatibility and dependency boundary checks pass.

## Milestone 3: Console wiring and release

**Work items**

- [x] Wire selected health outputs through bounded `drop_oldest` queues.
- [x] Remove fixed `workload/faults`, project `flow.yml` discovery and Review blocking.
- [x] Pair recorder profiles with native Dora graphs and pbtxt configs.
- [x] Launch the live bridge from pbtxt and update PR 96.

**Validation**

- [x] Graph tests cover health fan-in and user-message inputs.
- [x] Console server/frontend checks pass.
- [x] Release scenarios pass: alone, live, MCAP, OTLP, CPU topics and watchdog recording.
- [x] Full local CI, hosted x86/ARM64 CI and ARM64 native checks pass.
