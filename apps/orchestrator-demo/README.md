# Orchestrator lab

A live view of real Docker CPU-monitor experiments: step timing, dependencies,
inputs/outputs, errors and verified cleanup. The library emits shared Protobuf
telemetry; the demo owns transport and presentation.

![Architecture](docs/102626-live-workflow.svg)

```sh
python3 libs/orchestrator/scripts/docker-monitor-e2e.py --sudo --live-demo --output /tmp/orchestrator-live
```

Open http://127.0.0.1:8092. **Run experiment** executes the monitor workflow;
**Test failure & cleanup** fails its readiness audit after native telemetry starts.
Select a timeline row to inspect its evidence. Closing a browser leaves cleanup
running; reconnect recovers bounded history. Ctrl-C stops the launcher after cleanup.

Requires the [native Docker fixture prerequisites](../../libs/orchestrator/docs/102626-docker-monitor-e2e.md#run-and-inspect).
The web server runs unprivileged on loopback; only the fixed Docker worker is
optionally elevated. Node is needed for building static assets. No development
server or Node process remains resident. `--port` changes the HTTP port.

[Milestones and validation](docs/102626-live-workflow.md) ·
[Shared workflow fixture](../../libs/orchestrator/examples/support/docker_workflow.rs) ·
[Telemetry observer](../../libs/orchestrator/src/telemetry.rs)
