# Console configuration

Console loads protobuf text from the command-line configuration file. Images
must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. This example reuses the existing CPU
Viewer collector, recorder and feed; supply a host-specific port override last.
Build that image from the same revision; see [CPU Viewer deployment](../../apps/cpu-viewer/deploy/README.md).

```protobuf
system_apps {
  id: "cpu-monitor"
  label: "CPU monitoring and recording"
  default_selected: true
  input {
    directory: "/absolute/doramcap"
    files: "apps/cpu-viewer/deploy/compose.capture.yaml"
    files: "/absolute/cpu-overrides.yaml"
  }
  graph: "/absolute/doramcap/apps/cpu-viewer/deploy/flow.yml"
  output_mounts { service: "monitor" target: "/results" }
  output_mounts { service: "recorder" target: "/results" }
  output_mounts { service: "viewer" target: "/results" }
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.mcap" cpu: true }
  cpu_feed_url: "ws://127.0.0.1:8081/stream"
}
```

The override sets explicit Compose `user` fields and `viewer.command` when host
settings differ. Service users must match the configured Dora participant identity.
Output mounts bind the current run's artifact directory;
recording paths are relative to it. The current runtime requires stream growth
before workload launch; using the MCAP path also verifies writer progress.

`dora.graph` supplies the workload graph. Selected app graphs add nodes; duplicate
node IDs or conflicting graph settings reject preparation. Source graphs are read
again on Start. Console never injects demo producers. Reusable deployments must
supply healthchecks for subscriber/sink readiness before their producers start.

The monitor publishes BPF root CPU records, per-cgroup counters and capture health
through the CPU feed. Console derives total core utilization from root runtime deltas. Recordings are finalized by
the workflow and exposed by ID after completion. Absent or invalid display
metadata never bypasses the workflow's repository controller and reset.
