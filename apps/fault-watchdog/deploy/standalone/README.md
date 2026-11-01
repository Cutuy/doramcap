# Standalone Fault Watchdog

Adds the existing watchdog backend to an experiment with shared Dora and a
protocol-compatible workload. Merge the [graph fragment](flow.yml) with workload
nodes and supply the [Compose input](compose.yaml) as a system app input.

Build for the target architecture with a toolchain compatible with Ubuntu 24.04,
then stage the binary beside the Dockerfile:

```sh
cargo build --release --locked -p fault-watchdog --features mcap,otel
cp target/release/fault-watchdog apps/fault-watchdog/deploy/standalone/fault-watchdog
docker compose -f apps/fault-watchdog/deploy/standalone/compose.yaml build
```

Supply explicit Compose overrides for the user, configuration directory and fresh
results directory. Set `user` to `DoraRuntime.participant_uid:participant_gid` so
the process can read the runtime's private transport configuration. The image's
`65532:65532` default requires this override when participants use another identity.
Provision writable results for that same user.

| Contract | Configuration / behavior |
| --- | --- |
| Identity | Compose service and graph node are `fault-watchdog`; change both together |
| Dora runtime | Workflow supplies approved native connection settings, read-only transport configuration and shared IPC; use host networking |
| Producer | Replace `workload/faults` with a declared output of raw reporter-protocol bytes; begin with a snapshot and repeat snapshots periodically |
| Input | Graph name `workload_faults` matches `WatchdogConfig.inputs`; Dora queue_size is 32 with backpressure |
| Configuration | Read-only `/etc/fault-watchdog`; override `command` to select `watchdog.pbtxt`, `watchdog-mcap.pbtxt` or `watchdog-otel.pbtxt` |
| Live output | `fault-watchdog/faults` carries aggregation-protocol updates, snapshots and source/sink diagnostics |
| MCAP | Enable the compiled feature and provision io_uring permissions; create-new `/results/faults.mcap`; enable rotation only when the caller collects every part |
| OTLP | Enable the compiled feature; configure a reachable full logs URL and service name; the image supplies system CA certificates for HTTPS |
| Readiness | Marker confirms node attachment and worker creation; observe sink delivery and source health through aggregation publications and stderr |
| Shutdown | Workflow stop drains the Dora event stream; the watchdog publishes final state, closes all sink queues and awaits finalization within `shutdown_ms` |

The example allows 15 seconds for container shutdown and 5 seconds for sink drain;
keep those budgets aligned when overriding configuration. Preserve incomplete
recordings after forced termination. Ordinary Compose startup requires prior graph
assembly and a prepared shared runtime.

See the [runtime contract](../../docs/102026-runtime.md) for liveness, bounded sink
queues, loss reporting and finalization, and the [fault protocol](../../../../libs/fault/protocol/README.md)
for producer/publication schemas. Existing gallery MCAP deployment uses
`seccomp=unconfined`; qualify the chosen io_uring policy and limits on the target host.
