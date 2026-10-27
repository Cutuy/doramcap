![Architecture](102626-function-steps.svg)

# Procedural Rust steps

Created: 2026-10-26. Status: factory injection revision. Target: 4 GB machines.

## Milestones and validation

Update this checklist in place as work proceeds. Check an item only after its
validation evidence is recorded; deferred work remains unchecked.

- [x] Implement transient struct steps, shared config, serialized boundaries and events.
  Evidence: `src/` and local `cargo test -p orchestrator --offline`, 2026-10-26.
- [x] Validate dependencies, nested/concurrent calls, cancellation and observer isolation.
  Evidence: seven integration tests in `tests/steps.rs` passed on x86_64.
- [x] Provide bounded external-script I/O and error exposure.
  Evidence: two integration tests in `tests/process.rs` passed on x86_64.
- [x] Complete local lint, documentation, example and formatting validation.
  Evidence: Clippy with `-D warnings`, `cargo doc`, example execution and workspace
  formatting check passed on 2026-10-26. All nine integration tests passed.
- [x] Complete ARM64 worker validation.
  Evidence: Pueue job 133 succeeded on 2026-10-26 at 15:25 PDT; workspace
  default/all-feature checks, tests and native validation passed. Snapshot:
  `2c4253fb2ee743ae85efe220c0dcf6da`. Subsequent edits only record this evidence.
- [x] Dogfood native CPU monitoring through real Docker lifecycle steps.
  Evidence: [Docker E2E milestones and results](102626-docker-monitor-e2e.md),
  two successful attempts plus injected-failure cleanup on 2026-10-26.
- [x] Emit shared workflow telemetry and demonstrate live Docker observability.
  Evidence: [live demo milestone record](../../../apps/orchestrator-demo/docs/102626-live-workflow.md);
  local live delivery, replay and failure-cleanup tests passed on 2026-10-26.
- [ ] Qualify production memory on a 4 GB target.
  Gate: record peak RSS and bounded event/log buffering during representative
  experiment start, observation and cleanup on the target hardware.
- [ ] Implement generic Compose adoption in `workload-lifecycle`.
  Gate: complete the [lifecycle milestones](../../workload-lifecycle/docs/102626-foreign-workload-adoption.md#milestones-and-validation),
  including unchanged pipeline code when Compose services are added or removed.

## Factory injection revision

Approved API: a fresh step comes from a caller-supplied factory; shared services
are ordinary typed fields. Remove `Context`, generic configuration and `Limits`.
Keep mandatory JSON input/output round trips and automatic lifecycle telemetry.

- [x] Implement `LocalExecutor`, typed factories and context-free `Step`.
- [x] Migrate external steps, Docker fixture and live worker.
- [x] Validate singleton sharing, fresh instances, serialization and lifecycle regressions.
- [x] Update API documentation and architecture SVGs.
- [x] Re-run real Docker and live telemetry validation.
- [x] Submit and verify the revised source on the ARM64 worker.
  Evidence: Pueue job 136 succeeded at 16:40 PDT on 2026-10-26; snapshot
  `5151eb912c03454e874f94bfecbfde84`. Default/all-feature tests and native
  validation passed. Subsequent edits only record validation evidence.

Revision evidence, 2026-10-26: [validation record](102626-factory-injection-validation.json).
Local checks passed: 16 orchestrator tests, one demo test, three protocol tests,
two frontend tests, strict Clippy, Rustdoc, formatting, simulated workflow and
frontend production build. The updated SVG was rendered and visually inspected.
Real Docker: two normal runs plus injected-failure cleanup; finalized recordings
with 23/24/24 snapshots. Live demo: 40 events across two runs, zero drops,
reconnect replay and cleanup after viewer disconnect. Native Docker runs used
the existing trusted-test capability/seccomp policy documented in the E2E plan.

## Decision and ownership

The workflow is an ordinary async Rust function. Steps implement `Step` with
associated input/output types and one `run(&mut self, input)` method.
`executor.register::<MyStep>(factory)` supplies construction;
`executor.run::<MyStep>(input).await` creates and drops a fresh instance.
Factories capture dependencies and inject them into typed fields. Callers choose
`Rc`, `Arc`, ownership and lifetimes; shared services never cross serialization
boundaries. Configuration is application data supplied through inputs or injected
fields. The core has no experiment model or `Context` wrapper.

Reuse Tokio, Serde/serde_json, tracing and thiserror. Factory registration uses
standard-library `TypeId` and closures; it requires no handwritten step names or
DI framework. Re-registration replaces a factory; missing registration returns
`UnregisteredStep` before emitting events. Factories run after input validation.
Rust type names are diagnostic identities; a rename changes the name. They are
not a stable remote dispatch ABI.

Rust owns the initial workflow and execution library. Python and Node scripts can
implement individual steps through `ProcessStep`. A future frontend consumes events
and does not require Node on the target. Language bindings could eventually expose
the core without rewriting it, but are outside this implementation.

## Execution and observation

Use one current-thread Tokio runtime. Native calls run in place and permit non-Send
state. `tokio::join!` provides explicit concurrency; steps must yield and avoid
blocking/CPU-heavy work. Runtime-internal helper threads may still exist.

Each invocation records a caller-defined telemetry label (`session` on the wire),
ID, type name, version, local executor, parent,
explicit dependencies, elapsed time and terminal outcome. The synchronous observer
receives events as they occur; the core retains no history. Applications provide a
bounded sink and decide whether observation loss invalidates experiment evidence.
Observer panics are counted and suppressed to avoid repeating side effects.

`LocalExecutor::new(observer)` injects the event consumer. Payload capture is
opt-in via `capture_payloads(true)`; redact at the application boundary. Typed
inputs and outputs always round-trip through JSON, including local execution.
There is no core payload-size limit. Transport queues, telemetry encoding and
external-process streams retain their own bounds. Large recordings stay outside
orchestration memory; steps pass paths or identifiers.

The executor owns factories, ID allocation, clock and observer. It retains no
application configuration or step outputs. Factory captures determine singleton
lifetimes: sharing one service across factories shares that instance across steps.
Dropping the executor drops its factories; caller-held references can keep services
alive. Use weak executor references when injecting recursive orchestration to avoid
ownership cycles. Nested calls use a coroutine-local scope; concurrent siblings
remain independent. Spawned tasks do not automatically inherit parent metadata.

## Dependencies

Passing `Output<T>` directly into another step transfers its value and records the
producer. For transformed or combined inputs, use
`Input::new(value).after(&first).after(&second)`. Parent IDs record nested calls;
dependency IDs record data/control prerequisites. Cross-executor outputs are rejected.
Extracting a raw value discards automatic lineage; equal paths do not imply an edge.
File-producing steps return explicit paths, with digests when the domain needs them.

## Failure and resource lifetime

Errors propagate without retries. Cancellation drops the step future and instance
and emits a terminal event; panics unwind normally and emit a panic event.
Rust drop cannot await cleanup. Steps own partial-failure cleanup, and procedural
workflows explicitly await stop/drain/restore after normal errors. Do not cancel a
whole workflow and assume its resources have stopped.

Resource-owning operations return identifiers that later cleanup steps can consume.
Setup checks existing state and ownership; stop tolerates already-absent owned
resources. The runner cannot enforce idempotence of arbitrary step code.

## External steps

`ProcessStep` launches an executable with literal arguments and sends one JSON object
on stdin: `protocol: 2`, `input`. Application metadata belongs explicitly in
`input`; the previous implicit context fields have been removed.
The executable emits one JSON result on stdout and diagnostics on stderr.
Python example body:

```python
import json, sys
request = json.load(sys.stdin)
json.dump({"path": request["input"]["path"]}, sys.stdout)
```

Read/write/wait proceed concurrently. Timeout and stream limits kill and await the
direct child; dropped calls use Tokio kill-on-drop. Process trees and persistent
services require domain containment and explicit shutdown. This adapter supplies
no PTY or interactive Docker terminal bridge. Those belong in lifecycle adapters.

## Experiment integration

`workload-lifecycle` owns Docker/system configuration and monitor/recorder steps.
A generic step consumes the resolved Compose project; adding a supported service
requires zero pipeline or step-registry edits. This crate does not implement Compose
adoption yet. Customer pbtxt supplies experiment configuration.

Only the manager and its self-monitor persist. Each experiment starts fresh native
services and foreign workloads, stops foreign workloads, drains/stops native services,
restores/verifies configuration, and seals evidence before the next experiment.

## Deferred capabilities

Remote transport, multiple executors, remote discovery, artifact transfer, resume and
crash detection are deferred. Event metadata reserves executor identity (`local`).
Future remote dispatch needs versioned discovery and authorization; serialized
values alone do not make arbitrary Rust code remotely invocable. One/all executor
calls will need per-executor outcomes and explicit path-transfer semantics.

## Runtime memory evidence

2026-10-26 exploratory measurement, x86_64 Linux 7.0.0, Node 22.23.3,
Rust 1.99.1, Tokio 1.53.3 (release, current-thread, timer support). Five fresh
processes per case; sampled `/proc/PID/smaps_rollup` 250 ms after readiness.
Each coroutine waits on a 60-second timer; the probe terminates after sampling.
No forced GC, payloads, Docker, serialization, tracing or application modules.

| Waiting coroutines | Rust RSS / PSS MiB | Node RSS / PSS MiB |
|---|---|---|
| 1 | 2.08 / 0.55 | 42.90 / 28.58 |
| 1,000 | 2.44 / 0.91 | 45.24 / 30.89 |

Values are medians. PSS apportions shared pages and depends on other processes.
Observed thread counts were 1 for this Tokio probe and 7 for Node; both used one
thread for the user coroutine event loop. Node's internal threads are not an
application-managed step pool. RSS includes resident shared pages; the roughly
41–43 MiB difference is about 1% of 4 GiB. These idle timer measurements establish
a baseline, not production footprint, peak usage or ARM64 qualification.

[Raw measurements](102626-runtime-memory.json). Disposable probe sources were
kept under `/tmp/doramcap-runtime-memory`; no runtime dependency was added here.

The existing project-explorer frontend builds to self-contained static assets
(see its hosting documentation). Current browser UIs therefore do not establish
a requirement for Node on the target device. Future agent or JavaScript services
could introduce one, but that is a product choice, not an assumed dependency.
