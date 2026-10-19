# Activity demo milestones

- [x] M1: Detached same-runtime message context and four worker pub/sub simulator.
  Gate: closed publisher identity survives a cross-thread handoff; Rust tests pass.
- [x] M2: Bounded live replay endpoint and embedded MCAP output.
  Gate: real WebSocket frames decode; all four worker TIDs appear; causal parents
  and message IDs match; controls and replay pass without drops.
- [x] M3: React/TypeScript timeline, inspector, and message exchange feed.
  Gate: decoder tests and TypeScript production build pass. Browser visual review
  remains a follow-up because this session exposes no browser automation surfaces.
- [x] M4: ARM64 workspace/native regression and final evidence.
  Gate: inspect the submitted Pueue job result and record it here.

Local validation: 17 default-feature library tests, four frontend tests, production
frontend build, all-target/all-feature Clippy with warnings denied, and the real
HTTP/WebSocket integration check passed. The latter verified 802 records across
four worker threads, pub/sub causality, pause/manual greeting, reconnect replay,
and zero capture/mailbox/field loss.

ARM64 Pueue job 32 passed default/all-feature workspace tests, builds and native
capability-required accounting/allocation checks. Snapshot:
`/home/doramcap-ci/jobs/6415ce0660b243ce9c9df446dda2a69a`.
Job 30 exposed zero-timestamp source archives reusing stale Cargo outputs;
`scripts/ci/submit.py` now timestamps archived files to invalidate those outputs.
The local all-feature test hit disk capacity; its ARM64 equivalent passed.

The allocation gate additionally passed 1,000 copied-context handoffs with zero
producer allocations. A two-second demo run wrote 102 batches to each sink,
reported zero losses/errors, and produced a finalized 27,628-byte MCAP file.
The footer record and terminal magic were verified. Frontend formatting was
followed by another successful decoder test and production build.

Remaining manual gate: inspect the rendered timeline and exercise span selection
in a browser. No browser surfaces were available to the agent for this check.


## Gaussian sleep controls

- [x] Atomic μ/σ settings, validated API, bounded Gaussian sampling.
  Gate: seeded mean/standard-deviation check, σ=0, bounds, and packing pass.
- [x] Frontend sliders, distribution preview, 300 ms debounce, ordered requests.
  Gate: TypeScript build passes; browser interaction review remains manual.
- [x] Runtime integration.
  Gate: live API rejects invalid settings and new sleep spans carry zero-duration
  samples when μ=σ=0; original settings restored after the check.

ARM64 Pueue 34 passed full workspace default/all-feature tests and native checks.
Snapshot: `/home/doramcap-ci/jobs/89a057eb35884498bb9e04be846c4955`.
Local live Gaussian validation observed 14 sleep spans using the requested fixed
zero duration; sampling tests, Clippy, frontend tests/build also passed.

## Memory and buffer visualization

- [x] Read-only capture/encoded pool snapshots and status resource counters.
  Gate: held/queued/free capture blocks and registration counts track transitions;
  Clippy passes with warnings denied. Snapshot reads add no producer callbacks.
- [x] Resource panel with RSS/encoded-throughput trends, configured pool payload,
  replay/broadcast gauges, per-worker mailboxes and browser retention counts.
  Gate: six frontend tests and production build pass; zoom controls preserved.
- [x] Live pressure validation on a separate demo instance.
  Gate: μ=2,000 ms creates mailbox backlog; byte counters increase, capacities
  remain bounded, RSS/high-water data are present. Observed configured pool payload
  was 22,908,928 bytes and server RSS was 14,065,664 bytes.

Disk exhaustion during implementation was recovered from the prior verified CI
snapshot; generated artifacts for activity-demo/activity-tracing were cleared.
Visual browser review remains manual because browser automation is unavailable.

ARM64 Pueue 37 passed full workspace default/all-feature tests and native checks.
Snapshot: `/home/doramcap-ci/jobs/ca8e72c008ce4b4d9504f76fd6afd79c`.


## Publishing frequency slider

- [x] 0.1–20 Hz slider shares the existing 300 ms debounce with μ/σ.
- [x] Validated playground endpoint and watch-based scheduler rescheduling.
- [x] Live check: slow timer wakes promptly at 20 Hz, invalid settings are rejected
  without mutation, pause suppresses scheduled starts and manual greeting works.

Local Rust unit tests, six frontend tests, Clippy and production build passed.
The demo was restarted with existing sleep/pause settings restored.
ARM64 Pueue 38 passed full workspace default/all-feature tests and native checks.
Snapshot: `/home/doramcap-ci/jobs/e41efb5e94df4f3592db6b0881f487c9`.
