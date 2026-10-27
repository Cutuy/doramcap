# Orchestrator lab

A live view of real Docker CPU-monitor experiments: step timing, dependencies,
inputs/outputs, errors and verified cleanup. The library emits shared Protobuf
telemetry; the demo owns transport and presentation.

![Architecture](docs/102626-live-workflow.svg)

```sh
python3 libs/orchestrator/scripts/docker-monitor-e2e.py --sudo --live-demo --output /tmp/orchestrator-live
```

Open http://127.0.0.1:8092. **Run experiment** executes the Compose workflow and filesystem fixture;
**Test failure & cleanup** uses an unhealthy service to exercise cleanup.
Select a timeline row to inspect its evidence. Closing a browser leaves cleanup
running; reconnect recovers bounded history. Ctrl-C stops the launcher after cleanup.

Requires the [native Docker fixture prerequisites](../../libs/orchestrator/docs/102626-docker-monitor-e2e.md#run-and-inspect).
The web server runs unprivileged on loopback; only the fixed Docker worker is
optionally elevated. Node is needed for building static assets. No development
server or Node process remains resident. `--port` changes the HTTP port.

[Milestones and validation](docs/102626-live-workflow.md) ·
[Filesystem tracking](docs/102626-filesystem-tracking.md) ·
[Compose workflow](../../libs/workload-docker/src/workflows/recorded_compose.rs) ·
[Telemetry observer](../../libs/orchestrator/src/telemetry.rs)

The filesystem panel separates host binds, container layers and named volumes,
with change filters, metadata and coverage warnings. OverlayFS mounting is pending;
the demo currently observes direct bind writes into fresh per-run directories.
