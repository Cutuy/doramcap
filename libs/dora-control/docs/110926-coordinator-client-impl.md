# Dora coordinator client implementation

Created: 2026-11-09. [Architecture](110926-coordinator-client.md).
Migration roadmap: [PR #210](https://github.com/Cutuy/doramcap/pull/210).

[PR #251](https://github.com/Cutuy/doramcap/pull/251) delivers the standalone
`dora-control` library. Compose and systemd adoption belong to separate follow-up
PRs. Callers supply Dora's typed Start request, validate endpoint ownership and
retain responsibility for native process teardown and persisted run evidence.

Use locked Dora 1.1.0 messages and tokio-tungstenite 0.29.0 on the caller's Tokio
runtime. A request temporarily owns the socket; cancellation drops it while the
client retains graph identity. Calls use absolute deadlines and never replay
uncertain mutations. Force shutdown requires exclusive coordinator ownership and
stops the graph before destroying its coordinator and daemons.

## Milestone 1: Standalone client

Evidence: final-source focused WebSocket tests, strict Clippy, formatting and
dependency boundaries passed on x86. Real Dora 1.1.0 passed registration,
inspection, force shutdown and verified coordinator/daemon exit.

**Work items**

- [x] Add typed control operations with explicit endpoint/token inputs.
- [x] Reuse the connection and retain Start intent/UUID through cancellation.
- [x] Validate reply IDs/types and protocol versions without replaying mutations.
- [x] Resolve uncertain graph identity conservatively and force-stop before Destroy.
- [x] Keep node-stop grace explicit; document exclusive-runtime shutdown ownership.
- [x] Add approved bounded startup readiness sampling and its replacement TODO.

**Validation**

- [x] Exercise real WebSocket frames, explicit authentication and Ping/Pong.
- [x] Cover node stop, invalid input/replies, disconnect, silent peers and deadlines.
- [x] Cover cancelled Start/Stop, missing/ambiguous identities and UUID retention.
- [x] Run the real Dora 1.1.0 lifecycle and verify native process exit on final source.

## Milestone 2: Ready for independent adoption

Validation is tracked in [PR #251 checks](https://github.com/Cutuy/doramcap/pull/251/checks).

**Work items**

- [x] Publish the standalone library separately from its migration roadmap.
- [ ] Complete Scanner review against the implementation/design guides.
- [ ] Record final validation evidence, mark the PR ready and release its lease.

**Validation**

- [ ] Pass local locked client tests, default/all-feature Clippy, formatting, documentation and dependency-boundary checks.
- [ ] Pass cloud workspace checks on x86_64 and ARM64.
- [ ] Verify ARM64 worker completion; record native Dora results separately.

## Follow-up PR: Compose adoption

**Work items**

- [ ] Inject the shared client through existing factories and retain cleanup ownership.
- [ ] Preserve prepared dynamic descriptors, runtime-visible paths, session identity and artifacts.
- [ ] Replace control exec calls, then remove their unused helpers and output parsing.
- [ ] Share the startup deadline and give cancelled startup a fresh shutdown budget.

**Validation**

- [ ] Run registration, observation, node stop, graceful teardown and failed startup with real Dora.
- [ ] Cancel during startup and hang/disconnect control; verify native exit and evidence retention.
- [ ] Verify concurrent repositories stay isolated and control launches no transient Dora CLI processes.

## Follow-up PR: systemd adoption

**Work items**

- [ ] Inject and retain the client while preserving unit ownership and execution context.
- [ ] Preserve prepared dynamic descriptors, runtime-visible paths, session identity and artifacts.
- [ ] Replace control oneshots, then remove unused unit helpers and output files.
- [ ] Enforce control deadlines while continuing native teardown after failures.

**Validation**

- [ ] Run startup, registration, inspection, stop and failed startup with real Dora units.
- [ ] Cancel during startup and hang/disconnect control; verify units stop and reports preserve outcomes.
- [ ] Compare descriptor/session artifacts and verify repository isolation and absence of control oneshots.

## TODO: Replace startup readiness polling

Owner: `dora-control` integration. This approved follow-up does not gate PR #251.

**Work items**

- [ ] Add/adopt an upstream wait-for-daemon operation or readiness subscription.
- [ ] Replace timed registration checks and remove `DAEMON_REGISTRATION_CHECK_INTERVAL` and its polling exception.

**Validation**

- [ ] Cover registration before/after subscription, disconnect, cancellation and deadline expiry.
- [ ] Verify adopted backends preserve startup ordering and teardown without readiness polling.
