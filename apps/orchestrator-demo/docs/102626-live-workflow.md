![Architecture](102626-live-workflow.svg)

# Live workflow observability

Created: 2026-10-26. Scope: reusable workflow telemetry and a real Docker demo.

Current workflow contract owner: [workflow-protocol](../../../libs/telemetry/workflow-protocol/).
See [contract ownership and compatibility](../../../libs/telemetry/docs/102626-contract-ownership.md).

## Milestones and validation

- [x] Add a separate workflow Protobuf contract to `telemetry-protocol` and an
  orchestrator observer with bounded encoding and visible delivery-loss counters.
  Evidence: four observer integration tests passed on 2026-10-26, covering
  codec metadata, all states, admission/delivery failure, panic isolation and gaps.
- [x] Stream a bounded replay/live feed from the real Docker workflow.
  Evidence: [live validation](102626-live-validation.json), 40 Protobuf events
  across two real Docker runs, live delivery before completion, conflict rejection,
  viewer-disconnect cleanup, reconnect replay and zero dropped events.
- [x] Build an interactive live timeline, dependency links and step inspector.
  Evidence: frontend production build and two model tests passed; failure and
  cleanup events delivered through the live endpoint. Visual inspection is pending.
- [x] Run local Rust/frontend checks and real Docker validation.
  Evidence: Rust tests, strict Clippy, formatting, frontend model tests/build and
  `npm run test:live` passed on 2026-10-26. Native recordings contain 24 snapshots
  per attempt, zero drops and no cleanup errors.
- [x] Complete ARM64 regression validation.
  Evidence: job 135 succeeded at 16:04 PDT; verified on 2026-10-26. The factory
  injection revision also passed job 136 at 16:40 PDT, snapshot
  `5151eb912c03454e874f94bfecbfde84`.
- [ ] Inspect the rendered browser UI.
  Blocker: the computer-use tool reports no available browsers in this session.
  The live app is served at http://127.0.0.1:8092; no screenshot is claimed.

Factory injection revalidation, 2026-10-26: the rebuilt worker passed the same
live checks with 40 events, zero drops and finalized recordings in both attempts.
See [revision evidence](../../../libs/orchestrator/docs/102626-factory-injection-validation.json).

## Components

The worker registers typed step factories on `LocalExecutor` and injects its
telemetry observer. The library emits versioned Protobuf events through that sink.
Transport, replay and presentation belong to the demo. The existing CPU packet
schema stays unchanged. The demo reuses Axum, Tokio, React and protobufjs.

The authorized worker runs only the fixed monitor workflow. A private Unix socket
carries run requests and encoded events to the unprivileged HTTP server. WebSocket
clients receive bounded replay followed by live events. Payload capture is explicit;
the demo's inspector contains only this fixture's resource metadata.

## Delivery and lifetime bounds

- Observer events default to metadata; payload capture is explicit. A sink must
  enqueue without blocking. Admission/sink failures increment visible counters.
- Worker output queue: 128 frames, each at most 1 MiB. Server replay: 512 events /
  4 MiB. Broadcast: 128 frames; stalled viewers time out after two seconds.
- Browser retention: 512 events / 4 MiB of encoded input and up to 16 sessions.
  Decoded object overhead is additional. Replay eviction and missing starts are
  explicit; this is observation history rather than a recovery journal.
- The worker and web server persist as managers. Each experiment's monitor and
  recording process is created fresh and terminated before another run is admitted.
- The worker uses the shared `workload-docker` Compose workflow. The original
  standalone Docker example and native launcher have been retired.
- Experiment artifacts remain under the chosen output directory.

## Validation commands

```sh
cargo test -p orchestrator -p orchestrator-demo -p telemetry-protocol
cargo clippy -p orchestrator-demo -p orchestrator --all-targets --all-features -- -D warnings
npm test --prefix apps/orchestrator-demo/frontend
npm run build --prefix apps/orchestrator-demo/frontend
# Runs a normal and injected-failure experiment against an already-running demo:
npm run test:live --prefix apps/orchestrator-demo/frontend
```
