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

Evidence for runtime source `375b0878`:

- Local CI-profile client tests, strict Clippy/Rustdoc, workspace formatting, boundaries and snapshot tooling passed; real Dora 1.1.0 exited cleanly.
- Hosted [Rust CI](https://github.com/Cutuy/doramcap/actions/runs/42773353944) passed on x86_64/ARM64; [Node](https://github.com/Cutuy/doramcap/actions/runs/42773354030) and [MCAP](https://github.com/Cutuy/doramcap/actions/runs/42773354068) passed.
- ARM64 worker jobs 635 (full suites/native checks) and 636 (real Dora 1.1.0 client lifecycle) passed from snapshot `32d5237a24d041e78e301f6ce7343533`.

Local checks focus on this library; hosted CI and the ARM64 worker qualify workspace integration. Compose/systemd adoption and GPU/IPC qualification are separate work.

**Work items**

- [x] Publish the standalone library separately from its migration roadmap.
- [x] Complete Scanner review against the implementation/design guides.
- [x] Record standalone validation evidence and separate adoption follow-ups.

**Validation**

- [x] Pass local locked client tests, default/all-feature Clippy, formatting, documentation and dependency-boundary checks.
- [x] Pass cloud workspace checks on x86_64 and ARM64.
- [x] Verify ARM64 worker completion and the separate real-Dora lifecycle result.

## Review fix: Destroy after confirmed stop with node errors

**Work items**

- [x] Continue runtime destruction after a matching `DataflowStopped` reply with node errors.
- [x] Preserve node failures and any destruction failure together; retain graph identity.

**Validation**

- [x] Reproduce the skipped Destroy with real WebSocket frames, then pass all 19 protocol tests, including destruction rejection and uncertain-stop protection.
- [x] Run strict Clippy/Rustdoc, formatting, boundaries and the real Dora lifecycle on the fix.
- [x] Verify final hosted checks and ARM64 worker qualification.

Evidence for runtime source `2028e137`: 19 WebSocket tests, strict local checks and
real Dora 1.1.0 passed; hosted [Rust CI](https://github.com/Cutuy/doramcap/actions/runs/42784702353),
Node/MCAP CI and Bugbot passed. ARM64 worker 640 passed from snapshot
`4ee42bec33b84f5581970078911d2e6f`. Full local workspace CI was not duplicated.

## Follow-up PR: Compose adoption

Work items and acceptance checks are tracked in the
[Compose integration milestone](../../workload-lifecycle/docs/110926-dora-control-integration-impl.md#follow-up-pr-1-compose-adoption).
The [integration architecture](../../workload-lifecycle/docs/110926-dora-control-integration.md)
maps retained ownership and direct control calls.

## Follow-up PR: systemd adoption

Work items and acceptance checks are tracked in the
[systemd integration milestone](../../workload-lifecycle/docs/110926-dora-control-integration-impl.md#follow-up-pr-2-systemd-adoption).

## TODO: Replace startup readiness polling

Owner: `dora-control` integration. This approved follow-up does not gate PR #251.

**Work items**

- [ ] Add/adopt an upstream wait-for-daemon operation or readiness subscription.
- [ ] Replace timed registration checks and remove `DAEMON_REGISTRATION_CHECK_INTERVAL` and its polling exception.

**Validation**

- [ ] Cover registration before/after subscription, disconnect, cancellation and deadline expiry.
- [ ] Verify adopted backends preserve startup ordering and teardown without readiness polling.
