# Standalone Fault Watchdog

Adds one watchdog to an experiment with an existing Dora runtime and workload.
The [Compose input](compose.yaml) runs the existing backend; the
[graph fragment](flow.yml) connects `workload/faults` to `workload_faults`.

## Build and configure

Stage the binary for the deployment host's architecture beside the Dockerfile:

```sh
cargo build --release --locked -p fault-watchdog --features mcap,otel
cp target/release/fault-watchdog apps/fault-watchdog/deploy/standalone/fault-watchdog
docker compose -f apps/fault-watchdog/deploy/standalone/compose.yaml build
```

The image uses the repository's existing Ubuntu 24.04 base. Build without optional
features when unused; configuration rejects sinks missing from the binary.
Select `watchdog.pbtxt`, `watchdog-mcap.pbtxt` or `watchdog-otel.pbtxt` by overriding
Compose `command`. Includes resolve within the mounted `config` directory; both
sinks can be enabled in one private pbtxt file using the existing schema.

Supply explicit Compose overrides for the image, user, configuration directory
and fresh writable results directory. The example runs as `65532:65532`; provision
results for the selected user. No producer, dashboard or coordinator is started.
For MCAP, allow io_uring using the deployment's reviewed seccomp policy; the
existing gallery MCAP example uses `seccomp=unconfined`. Qualify storage resource
limits on the target host before enabling recording.

## Experiment assembly contract

| Input | Requirement |
| --- | --- |
| Service / node | Example identity `fault-watchdog`; keep graph ID and Compose service aligned |
| Dora connection | Workflow supplies native settings through `dora_app::environment` and mounts its transport configuration/runtime at the resolved paths |
| Graph | Merge with workload nodes; replace `workload/faults` with a producer's declared output |
| Input mapping | Each graph input name must appear in `WatchdogConfig.inputs`; one source incarnation per input |
| Runtime mounts | Workflow supplies its shared-memory/IPC access and working directory; host networking matches the incoming shared-runtime deployment |
| Configuration | Read-only directory at `/etc/fault-watchdog`; command names the pbtxt file |
| Recording | Writable fresh directory at `/results`; declare `faults.mcap` and `faults.mcap.*` as artifacts |
| Live consumers | Subscribe to `fault-watchdog/faults`; receive aggregation publications |

The Compose input requires the shared runtime assembly; `docker compose up` alone
does not construct a Dora graph. It uses no gallery registration or old system-app
recipe. Override service/node identities and output paths explicitly when adding
multiple instances.

## Producer and output protocol

Publish raw bytes encoded by `fault_protocol::encode_reporter`, beginning with a
reporter snapshot and repeating snapshots periodically. Batches refer to that
registered source incarnation. Inputs require the existing fault protocol;
ordinary workload payloads are rejected. The graph uses a bounded 32-item
backpressure queue; tune it explicitly for the producer's publication budget.

`source_timeout_ms` detects silence. Producer definitions separately control
condition deadlines/recovery. Closed or timed-out sources retain faults as stale;
a new incarnation replaces the old source on that input.

The `faults` output carries `AggregationPublication` protobuf bytes with epoch,
revision, updates and periodic snapshots. Consume the shared protocol descriptor
and retain snapshot reconciliation. Sink queue loss/errors become local diagnostic
conditions in other outputs; each sink filters its own condition. Stderr records
sink errors and finalization failures. There is no additional HTTP health service.

## Readiness, stop and artifacts

- The approved `DORAMCAP_READY_FILE` bridge writes a marker in container-private
  tmpfs after node attachment and output-worker creation. This is startup readiness;
  sink initialization/delivery can still fail asynchronously. It does not certify
  producer availability, healthy faults or successful disk/OTLP delivery.
- Observe ongoing source/sink health through aggregation publications and process
  exit. The readiness marker persists until container removal; do not treat it as
  a continuously updated recording-health signal.
- Stop producers first and let Dora close/drain their inputs. The watchdog keeps
  observing disconnects until explicitly stopped with Dora Stop or SIGINT.
- On stop, it publishes final disconnected state/snapshot, closes every sink queue,
  and waits up to `shutdown_ms` for drain/finalization. Compose allows 15 seconds
  versus the example's 5-second application deadline; keep those budgets aligned.
- MCAP creates new files, records topic `faults`, embeds the aggregation schema,
  and rotates to `faults.mcap.000001` and later parts. Collect every part after
  process completion. Forced termination may leave incomplete files; preserve them
  as partial evidence. A valid footer does not establish zero publication loss.
- OTLP uses the configured full logs URL, timeout and service name. Configure a
  reachable collector and trust material in deployment inputs. A successful send
  establishes exporter acceptance, not durable retention by that collector.

[Runtime and configuration details](../../docs/102026-runtime.md)
