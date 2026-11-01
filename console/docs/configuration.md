# Console configuration

Console loads protobuf text from the command-line configuration file. Images
must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. The CPU example requires the modular
collector and recorder manifests from [PR35](https://github.com/Cutuy/doramcap/pull/35).
Supply explicit service users in the final override file.

```protobuf
system_apps {
  id: "cpu-monitor"
  label: "CPU monitoring and recording"
  input {
    directory: "/absolute/doramcap"
    files: "apps/system-monitor/deploy/reusable/monitor.yaml"
    files: "apps/cpu-viewer/deploy/reusable/cpu-recording.yaml"
    files: "/absolute/cpu-overrides.yaml"
  }
  graph: "/absolute/doramcap/apps/cpu-viewer/deploy/reusable/cpu-flow.yml"
  output_mounts { service: "cpu-recorder" target: "/results" }
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.mcap" cpu: true }
  live_outputs { source: "cpu-monitor/snapshots" kind: TELEMETRY_PACKET }
  live_outputs { source: "cpu-monitor/dictionary" kind: TELEMETRY_PACKET }
}
system_apps {
  id: "fault-watchdog"
  label: "Fault Watchdog"
  input { directory: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone" files: "compose.yaml" }
  graph: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone/flow.yml"
  live_outputs { source: "fault-watchdog/faults" kind: FAULT_PUBLICATION }
}
live_bridge { image: "doramcap/live-bridge:local" port: 43190 }
```

## Live bridge

`live_bridge` adds the optional, default-selected Live telemetry app beside the
Dora services. Console generates its Dora node and Compose service from the
selected apps' `live_outputs`; it runs only when at least one is declared.
Kinds select the browser decoder: `TELEMETRY_PACKET` or `FAULT_PUBLICATION`.
The bridge listens on loopback `port`. Build its image from [console/bridge](../bridge/README.md).

Service users must match the configured Dora participant identity. Output mounts
bind the current run's artifact directory; recording paths are relative to it.
The runtime requires stream growth before workload launch.

`dora.graph` supplies the workload graph. Selected app graphs add nodes; duplicate
node IDs or conflicting graph settings reject preparation. Source graphs are read
again on Start. Console never injects demo producers. Reusable deployments must
supply healthchecks for subscriber/sink readiness before their producers start.

The monitor publishes BPF root CPU records, per-cgroup counters and capture health.
The browser derives total core utilization from root runtime deltas in relayed frames. The workflow finalizes recordings and exposes them by ID. Absent or invalid
display metadata never bypasses the workflow's repository controller and reset.

## Browser access

Each launch generates an access link and an HTTP-only browser session. The link
exchanges its fragment token before the UI loads and removes the fragment from
browser history. Sessions expire when the process exits. Keep access links private.
`access_secret_file` optionally supplies a stable token; omit it for release launches.

The launcher binds `127.0.0.1`. `listen_address` selects the first candidate port;
`--port` requires an exact port. `--browser-port` selects the browser-facing port
and origin for SSH forwarding. Startup prints the effective URL and forwarding
command when `--ssh user@host` is supplied.
