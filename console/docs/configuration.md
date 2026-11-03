# Console configuration

Console loads protobuf text from the command-line configuration file. With `download_missing_images: true`, selected missing images download before
Review. Otherwise images must already exist locally. The Dora coordinator and daemon are always included.

Optional apps use ordinary Compose inputs. Each app container launches from its own
pbtxt, bind-mounted read-only by its Compose file. The
[release template](../release/console.pbtxt) is a complete working example.

```protobuf
system_apps {
  id: "cpu-monitor"
  label: "CPU monitoring"
  input { directory: "/absolute/release/system-apps/cpu-monitor" files: "compose.yaml" }
  graph: "/absolute/release/system-apps/cpu-monitor/flow.yml"
  output_mounts { service: "cpu-monitor" target: "/results" }
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.pb" cpu: true }
  live_outputs { source: "cpu-monitor/snapshots" kind: CPU_ACCOUNTING }
  live_outputs { source: "cpu-monitor/dictionary" kind: CPU_ACCOUNTING }
}
system_apps {
  id: "fault-watchdog"
  label: "Fault Watchdog"
  input { directory: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone" files: "compose.yaml" }
  graph: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone/flow.yml"
  output_mounts { service: "fault-watchdog" target: "/results" }
  live_outputs { source: "fault-watchdog/faults" kind: FAULT_PUBLICATION }
  config_mount { service: "fault-watchdog" target: "/etc/fault-watchdog/watchdog.pbtxt" }
  launch_profiles { id: "live" label: "Live only" config: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone/config/watchdog.pbtxt" }
  launch_profiles {
    id: "mcap"
    label: "Live and MCAP"
    config: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone/config/watchdog-mcap.pbtxt"
    recordings { id: "faults" path: "faults.mcap" }
  }
  launch_profiles { id: "otel" label: "Live and OTLP" config: "/absolute/doramcap/apps/fault-watchdog/deploy/standalone/config/watchdog-otel.pbtxt" }
}
live_bridge { image: "doramcap/live-bridge:local" }
```

## Launch profiles

`launch_profiles` lets the user choose how an app launches during Review. Console
mounts the chosen `config` read-only at `config_mount`; the first profile is the
default. A profile's `recordings` and `live_outputs` add to the app's own. Only the
chosen file is mounted and frozen for the run, so each `config` must be self-contained,
without `include`.

```protobuf
config_mount { service: "cpu-monitor" target: "/config/monitor.pbtxt" }
launch_profiles {
  id: "record"
  label: "Live panels and recording"
  config: "/absolute/monitor.pbtxt"
  recordings { id: "cpu" path: "cpu.mcap" stream: "cpu.pb" cpu: true }
}
launch_profiles { id: "live" label: "Live panels only" config: "/absolute/live.pbtxt" }
```

## Live bridge

`live_bridge` adds the optional, default-selected Live telemetry app beside the
Dora services. Console generates its Dora node and Compose service from the
selected apps' `live_outputs`; it runs only when at least one is declared.
Kinds select the browser decoder and panel: `CPU_ACCOUNTING` or `FAULT_PUBLICATION`.
The bridge listens on loopback `port`; omit it to select a free port at startup, so several Consoles can run on one host. Build its image from [console/bridge](../bridge/README.md).

Service users must match the configured Dora participant identity. Output mounts
bind the current run's artifact directory; recording paths are relative to it.
Services of recording apps get `seccomp=unconfined` for io_uring and a memlock limit automatically.
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
