# Event-driven waits

![Event-driven workers](110326-event-driven-waits.svg)

Replace Rust sleep, timed-park and yield loops used to poll for state changes outside tests with explicit event-driven signaling. Timers may implement sampling schedules, pacing and real deadlines.

## Plan

- One implementation PR per milestone. Milestones 2–4 are independent of milestone 1.
- Scope: Rust code. Preserve sampling schedules, simulated work and stability deadlines; record each retained timer's purpose in its milestone.
- Present third-party crates for evaluation before adding any.
- Bring any wait without an event source to the user before keeping a timer.
- ARM64 validation is waived by the user for this implementation. Keep local and cloud results separate.

## Milestone 1: Activity tracing

![Sink block batching and MCAP serialization](../libs/activity-tracing/docs/batch-blocks-flow.svg)

**Work items**

- [x] Follow up on merged #122 and #123: replace sink worker `park_timeout` in `libs/activity-tracing/src/pipeline.rs` with bounded channel signaling and `select!` for work, shutdown and active deadlines.
- [x] Remove the 10 ms merge window. Merge already queued blocks and use the existing 100 ms writer flush deadline only while data is buffered.
- [x] Reuse `McapWriter::flush_written()` at the flush deadline; add no public wait API.

**Validation**

Evidence: release locked tests passed with default features (24) and all features (30), including zero-allocation capture, startup failure, sink error/panic, idle wake and deadline regressions. Strict Clippy passed for both configurations; formatting, library boundaries and whitespace checks passed. `strace -ff -e futex,clock_nanosleep` showed two indefinite worker waits spanning the idle intervals, with work and shutdown wakes. Paced throughput (100,000 events, 1,000 us/64) delivered 1,564 batches with no drops; three unpaced runs delivered 930–1,329 and counted 235–634 sink drops. These are post-change measurements, not a before/after comparison.

Named-constant follow-up: timing call sites in the output tests use named durations; the writer uses `MAX_BATCH_AGE` for its flush deadline. Release locked output tests passed with default features (9) and all features (12); MCAP roundtrip tests passed (6). Capability-required executables ran with sudo after unprivileged builds. Formatting and whitespace checks passed.

Approved design: bounded block channels and a shared shutdown channel; no 10 ms merge window. Buffered data uses the existing 100 ms flush deadline and `flush_written()`.

- [x] Idle workers make no periodic wake-ups, verified by kernel wait tracing with `strace`.
- [x] A new block and a stop request each wake an idle worker. Shutdown drains accepted blocks.
- [x] Sink build failure, write failure and panic release blocks and allow shutdown to finish.
- [x] Record zero-allocation capture and `throughput` results in this milestone's implementation PR.

- [ ] Cloud CI finishes successfully. Local full-workspace CI is deferred to cloud CI; ARM64 validation is waived by the user. Dora/CUDA checks belong to later milestones.

## Milestone 2: System monitor

**Work items**

- [x] Add a blocking receive to the `monitor-core` handoff, and use it at `apps/system-monitor/src/main.rs:431`.
- [x] Forward Dora terminal events from a cancelable receiver thread into the control channel, replacing `stop_requested` polling.
- [x] Keep the sampling deadline at `main.rs:262`, waiting on a stop channel so stopping is immediate.
- [x] Signal collector completion and errors to the publishing loop; drain queued snapshots before exit and join the Dora receiver on shutdown.

Approved design: retain synchronous collection/publication with `crossbeam-channel` snapshot and stop/error waits. The Dora receiver uses `futures` cancellation and is joined on finish/drop. Both dependencies reuse locked versions already in the repository. Recording uses its existing buffered-data flush deadline.

**Validation**

- Release locked default tests pass (17); Dora-feature tests plus the adapter pass (20). Strict Clippy passes for default and Dora-feature configurations.
- Dora in-process tests verify stop notification and cancellation before a scheduled 30-second event. The sampling test verifies stop before a 30-second tick.
- Zero-allocation handoff, overload recovery, selected receive and producer-disconnect drain tests pass. `strace` shows an indefinite selected wait until the control signal.
- All-feature compilation and strict Clippy pass with `CLANG=/usr/bin/clang-18`. The privileged native recording/rotation test passes: 24 samples, zero drops, valid dictionary checkpoints. ARM64 is waived; full-workspace cloud CI is pending.

- [x] Ctrl-C and Dora stop wake waits without waiting for the next sampling tick.
- [x] Collector completion, failure and producer disconnect wake the publisher; queued snapshots drain before exit.
- [x] The idle publishing loop makes no periodic wake-ups.
- [x] System Monitor tests and its recording checks pass.
- [ ] Full-workspace cloud CI finishes successfully.

## Milestone 3: Demo apps

**Work items**

- [ ] `apps/dataflow-tracing/server/src/pipeline.rs:303`: block on Dora `recv_timeout` until the next real deadline.
- [ ] `pipeline.rs:121`, `:319`: replace `control_idle` and `drained` flags with a channel or condition variable.
- [ ] `apps/storage-demo/src/main.rs:407`, `:409`: wait with `recv_deadline` on commands until the next publish, and pace bursts on writer backpressure.
- [ ] `apps/storage-demo/src/observation.rs:159`: notify from the collector.
- [ ] `apps/cuda-shm-demo/src/main.rs:325`: block the consumer on Dora `recv`.
- [ ] `apps/cuda-shm-demo/src/main.rs:182`: signal the producer when a frame slot frees.

**Validation**

- [ ] Preserve dataflow simulated-work delays and storage publish pacing.
- [ ] Each app's existing tests pass; stop and disconnect wake blocked workers.
- [ ] CUDA SHM Demo passes on a CUDA-capable host.
- [ ] Storage Demo bursts still reach their previous throughput.

## Milestone 4: Orchestration

**Work items**

- [ ] `libs/workload-supervisor/src/steps.rs:430`: wait on `cgroup.events` with `inotify`.
- [ ] `steps.rs:289`: wait for `control.sock` and Supervisor exit.
- [ ] `libs/workload-lifecycle/src/recording.rs:87`: wait on recording file changes with `inotify`.
- [ ] `libs/workload-docker/src/compose/adapter/project.rs:220`, `workflows/recorded_compose/execution.rs:62`, `:135`: use Docker's event stream.
- [ ] `libs/workload-docker/src/workflows/dora.rs:121`: find a Dora daemon connection signal.
- [ ] `apps/project-explorer/server/src/runtime.rs:427`: wait on status with `tokio::sync::watch`.

**Validation**

- [ ] Subscribe before inspecting state; recheck readiness after notifications and handle event-stream closure.
- [ ] Readiness and stop timeouts still fail on their deadlines.
- [ ] Preserve or explicitly revise the recording footer stability deadline and system-app readiness stabilization interval.
- [ ] Dora end-to-end and Console tests pass.
- [ ] A final audit of Rust code outside tests finds no timer-driven state polling; retained timers have documented sampling, pacing or deadline behavior.

## Open decisions

- Crates for `inotify` and Docker events.
- Whether CUDA offers a blocking wait or callback for frame completion.
- Whether the Dora CLI offers any connection event.
