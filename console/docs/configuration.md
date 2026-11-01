# Console configuration

Console loads protobuf text from the command-line configuration file. Images
must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. This example reuses the existing CPU
Viewer collector, recorder and feed; supply a host-specific port override last.

```protobuf
system_apps {
  id: "cpu-monitor"
  label: "CPU monitoring and recording"
  default_selected: true
  input {
    directory: "/absolute/doramcap"
    files: "apps/cpu-viewer/deploy/compose.yaml"
    files: "/absolute/cpu-overrides.yaml"
    environment { key: "CPU_VIEWER_UID" value: "0" }
    environment { key: "CPU_VIEWER_GID" value: "0" }
    environment { key: "CPU_VIEWER_RESULTS_DIR" value: "/absolute/existing-placeholder" }
  }
  graph: "/absolute/doramcap/apps/cpu-viewer/deploy/flow.yml"
  output_mounts { service: "monitor" target: "/results" }
  output_mounts { service: "recorder" target: "/results" }
  output_mounts { service: "viewer" target: "/results" }
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.mcap" cpu: true }
  cpu_feed_url: "ws://127.0.0.1:8081/stream"
}
```

The override sets `viewer.command` to bind the feed to that loopback port. App
users must match the configured Dora participant identity. Output mounts replace
the placeholder with the current run's artifact directory;
recording paths are relative to it. The current runtime requires stream growth
before workload launch; using the MCAP path also verifies writer progress.

`dora.graph` supplies the workload graph. Selected app graphs add nodes; duplicate
node IDs or conflicting graph settings reject preparation. Source graphs are read
again on Start. Console never injects demo producers. Reusable deployments must
supply healthchecks for subscriber/sink readiness before their producers start.

Host CPU totals use Node's OS counters, including runs with recording disabled.
The CPU feed supplies per-cgroup counters and capture health. Recordings are finalized by
the workflow and exposed by ID after completion. Absent or invalid display
metadata never bypasses the workflow's repository controller and reset.
