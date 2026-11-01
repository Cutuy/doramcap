# Activity lab usage

## Component boundary

`server/` depends on `libs/activity-tracing` through its public Rust API. It owns
the four-thread simulator, bounded mailboxes, scheduler, Gaussian sleep settings,
HTTP controls, resource sampling, and the `LiveSink` replay/broadcast adapter.
`frontend/` owns the TypeScript decoder, timeline and controls. Its generated
codec comes from the library-owned `proto/activity.proto` schema using
`npm run generate:proto --prefix apps/activity-demo/frontend`.

The library supplies `McapSink` and the capture/encoding pipeline; see its
[architecture](../../../libs/activity-tracing/docs/architecture.svg).

## Controls

Open http://127.0.0.1:8088. Select a span or message to highlight its conversation.
Pause publishing to let queued messages drain; **Send hello** starts one conversation
even while paused. The **Activity playground** adjusts Gaussian μ (0–2,000 ms)
and σ (0–1,000 ms), plus publishing frequency (0.1–20 Hz), after a 300 ms debounce.
Frequency controls new conversations per second; replies follow each conversation.
The default is 0.6 Hz. Changing frequency resets the next scheduler deadline
without emitting catch-up bursts. Pause suppresses scheduled starts while manual
greetings remain available. Each new sleep reads one atomic
settings snapshot; in-progress sleeps finish normally. Samples are rounded to
milliseconds and clamped to 0–5,000 ms; σ = 0 gives a fixed duration. Sleep spans
include `duration_ms`, `mu_ms`, and `sigma_ms`. **Follow live** freezes/resumes the displayed time window.

Scroll over the timeline plot to zoom horizontally around the pointer (250 ms–60 s).
Zooming freezes the view for inspection; **Follow live** resumes tracking at the
chosen scale.

The server prints its recording path under `/tmp`. Ctrl-C stops workers and
finalizes MCAP. Options: `--port 8088`, `--record /path/new-file.mcap`,
`--assets /path/frontend/dist`, and `--duration 30` for a bounded run.
Existing recording files are protected from overwrite. Use `--bind 0.0.0.0`
to listen on container interfaces; the default is loopback. Frontend assets and
API paths support serving the app under a URL prefix.

## Mechanisms and bounds

- Crossbeam mailboxes hold 32 messages per worker. Publishing uses `try_send`;
  full mailboxes increment a visible drop counter.
- Each message carries a copied `ActivityContext`. Its receive span retains the
  publisher's parent ID and trace root after the publish span closes.
- Capture uses the library's preallocated blocks and bounded output queues.
  The demo uses 128 capture blocks, 68 encoded buffers, and sink queues of 32.
- The live output worker copies each encoded batch once into shared `Bytes`.
  Replay retains at most 512 batches / 4 MiB; broadcast holds 256 batches.
  Slow clients receive gap notices or disconnect after a two-second send timeout.
- The browser deduplicates replay by session/sequence, preserves 64-bit IDs,
  and retains at most 2,000 spans and 600 events within 65 seconds of trace time.
- The recorder, encoder, web server, and browser may allocate. Producer capture
  has a separate allocation regression test. This demo has no Thor bandwidth claim.

The app binds to loopback by default. Context tokens support synchronous, same-runtime
handoffs; await points require separate async propagation support. The demo enables
MCAP and live output; the library's optional OpenTelemetry sink remains available
for other hosts.

## Validation

```sh
cargo test --locked -p activity-tracing -p activity-demo
cargo clippy --locked -p activity-tracing -p activity-demo --all-targets --all-features -- -D warnings
npm test --prefix apps/activity-demo/frontend
npm run build --prefix apps/activity-demo/frontend
# With the server running; this temporarily pauses/resumes the demo:
npm run test:live --prefix apps/activity-demo/frontend
npm run test:gaussian --prefix apps/activity-demo/frontend
```

See [milestones and validation gates](101826-my-roadmpa.md).

## Memory and buffer usage

The **Memory & buffers** panel samples `/api/status` once per second:

- Server RSS and process high-water RSS come from Linux `/proc/self/status`.
- Capture/encoded pool gauges show occupied blocks and configured payload capacity.
  `Handle::buffers()` reads queue lengths; it adds no work to producer callbacks.
- Replay shows retained bytes and batches; broadcast shows shared pending slots.
- Each worker mailbox shows pending messages against its capacity of 32.
- Encoded throughput is the byte-counter delta between status samples. Trends retain
  at most 60 samples; observed pool peaks cover that displayed sampling window.
- Browser retention shows span/event counts. Browser heap bytes are unmeasured.

Try μ = 2,000 ms and σ = 0, then **Send hello** several times: mailbox backlog
should grow while pool payload capacity stays fixed. Lower μ to drain the backlog
and watch encoded throughput. Replay grows until its byte or batch limit evicts
old entries. One-second polling can miss brief pool spikes.

RSS measures resident server pages; reserved capacity can exceed RSS while pages remain untouched. Configured pool payload excludes allocator
metadata, queue bookkeeping, stacks, span registry, encoder scratch space and sink
allocations. Replay/broadcast share byte storage and should not be summed.
Encoded throughput measures serialized bytes; memory bandwidth and allocation
rate require separate profiling.

A live pressure check is available with `npm run test:resources --prefix
apps/activity-demo/frontend`. Prefer a separate demo instance: it temporarily
sets slow sleeps and publishes 12 greetings, restoring settings afterward while
queued work drains. `DEMO_URL` selects that instance.

Frequency validation: `npm run test:frequency --prefix apps/activity-demo/frontend`
checks live rescheduling, limits, pause and manual greetings (prefer a separate
instance via `DEMO_URL`).
