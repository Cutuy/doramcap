# Console configuration

Console loads protobuf text from the command-line configuration file. Images
must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. This example connects the reusable CPU
components from #34/#35; supply host-specific user/port overrides as the final file.

```protobuf
system_apps {
  id: "cpu-monitor"
  label: "CPU monitoring and recording"
  input {
    directory: "/absolute/doramcap"
    files: "apps/system-monitor/deploy/reusable/monitor.yaml"
    files: "apps/cpu-viewer/deploy/reusable/feed.yaml"
    files: "apps/cpu-viewer/deploy/reusable/cpu-recording.yaml"
    files: "/absolute/cpu-overrides.yaml"
  }
  graph: "/absolute/doramcap/apps/cpu-viewer/deploy/reusable/cpu-flow.yml"
  output_mounts { service: "cpu-recorder" target: "/results" }
  recordings { id: "cpu" path: "cpu.mcap" cpu: true }
  cpu_feed_url: "ws://127.0.0.1:8081/stream"
}
```

The override must set the feed to that port and app users to the configured Dora
participant identity. Output mounts bind the current run's artifact directory;
recording paths are relative to it. An omitted stream waits for recorder arming,
while an explicit stream also requires data growth before workload launch.

`dora.graph` supplies the workload graph. Selected app graphs add nodes; duplicate
node IDs or conflicting graph settings reject preparation. Source graphs are read
again on Start. Console never injects demo producers. Reusable deployments must
supply healthchecks for subscriber/sink readiness before their producers start.

The existing CPU feed provides live metrics. Declared recordings are finalized by
the workflow and exposed by ID after completion. Absent or invalid display
metadata never bypasses the workflow's repository controller and reset.
