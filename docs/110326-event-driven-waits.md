# Event-driven waits

![Event-driven workers](110326-event-driven-waits.svg)

Replace sleep, timed-park and yield loops outside tests with blocking waits on real events. Timeouts remain only for real deadlines.

## Plan

- One PR per milestone, in order. Milestones 2–4 are independent of milestone 1.
- Present third-party crates for evaluation before adding any.
- Bring any wait without an event source to the user before keeping a timer.

## Milestone 1: Activity tracing

**Work items**

- [ ] Replace sink worker `park_timeout` in `libs/activity-tracing/src/pipeline.rs` with a bounded channel and `select!`, in #122.
- [ ] Block the MCAP worker on its channel, using a deadline only for the merge window or batch age, in #123.
- [ ] Add a public `McapWriter` wait that blocks while writes are in flight, for `McapSink` to use.

**Validation**

- [ ] Idle workers make no periodic wake-ups, measured by context switches with `strace` or `perf`.
- [ ] A new block and a stop request each wake an idle worker.
- [ ] Zero-allocation capture and `throughput` results are recorded in #123.

## Milestone 2: System monitor

**Work items**

- [ ] Add a blocking receive to the `monitor-core` handoff, and use it at `apps/system-monitor/src/main.rs:431`.
- [ ] Forward Dora events from a thread blocking on `recv` into one channel, replacing `stop_requested` polling.
- [ ] Keep the sampling deadline at `main.rs:262`, waiting on a stop channel so stopping is immediate.

**Validation**

- [ ] Stop takes effect without waiting for the next sampling tick.
- [ ] The idle publishing loop makes no periodic wake-ups.
- [ ] System Monitor tests and its recording checks pass.

## Milestone 3: Demo apps

**Work items**

- [ ] `apps/dataflow-tracing/server/src/pipeline.rs:303`: block on Dora `recv_timeout` until the next real deadline.
- [ ] `pipeline.rs:121`, `:319`: replace `control_idle` and `drained` flags with a channel or condition variable.
- [ ] `apps/storage-demo/src/main.rs:407`, `:409`: wait with `recv_deadline` on commands until the next publish, and pace bursts on writer backpressure.
- [ ] `apps/storage-demo/src/observation.rs:159`: notify from the collector.
- [ ] `apps/cuda-shm-demo/src/main.rs:325`: block the consumer on Dora `recv`.
- [ ] `apps/cuda-shm-demo/src/main.rs:182`: signal the producer when a frame slot frees.

**Validation**

- [ ] Each app's existing tests pass.
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
- [ ] `console/server/src/service.ts:91`: replace the 100 ms `tick` with notifications from its sources.

**Validation**

- [ ] Readiness and stop timeouts still fail on their deadlines.
- [ ] Dora end-to-end and Console tests pass.
- [ ] A final audit of non-test code finds no timer-driven polling, apart from deadlines listed in this plan.

## Open decisions

- Crates for `inotify` and Docker events.
- Whether CUDA offers a blocking wait or callback for frame completion.
- Whether the Dora CLI offers any connection event.
