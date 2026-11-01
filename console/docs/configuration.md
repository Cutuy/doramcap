# Console configuration

Console loads protobuf text from the command-line configuration file. Images
must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. The CPU example requires the modular
collector, feed and recorder manifests from [PR35](https://github.com/Cutuy/doramcap/pull/35).
Supply explicit service users and the feed port in the final override file.

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
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.mcap" cpu: true }
  cpu_feed_url: "ws://127.0.0.1:8081/stream"
}
```

Service users must match the configured Dora participant identity. Output mounts
bind the current run's artifact directory; recording paths are relative to it.
The runtime requires stream growth before workload launch.

`dora.graph` supplies the workload graph. Selected app graphs add nodes; duplicate
node IDs or conflicting graph settings reject preparation. Source graphs are read
again on Start. Console never injects demo producers. Reusable deployments must
supply healthchecks for subscriber/sink readiness before their producers start.

The monitor publishes BPF root CPU records, per-cgroup counters and capture health
through the CPU feed. Console derives total core utilization from root runtime
deltas. The workflow finalizes recordings and exposes them by ID. Absent or invalid
display metadata never bypasses the workflow's repository controller and reset.
