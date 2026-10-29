# Gallery hosting

The server manages registered Compose apps and requires Docker Engine access.
Use a trusted listener and a separate access policy for external access.

## Build

```sh
npm ci --prefix apps/project-explorer/frontend
npm run build --prefix apps/project-explorer/frontend
cargo build --locked -p project-explorer-server --bins
```

Build app images independently; start with [Activity Lab](../../../activity-demo/deploy/README.md).

## Run

Choose a private, persistent data directory. From the checkout:

```sh
mkdir -p /absolute/path/gallery-data
chmod 700 /absolute/path/gallery-data
target/debug/project-explorer-server \
  --workspace "$PWD" --data /absolute/path/gallery-data \
  --assets "$PWD/apps/project-explorer/frontend/dist" \
  --origin http://127.0.0.1:8096
```

In another terminal, run Caddy on the host or in a container. This example uses
host networking so Caddy can reach the manager and the app's private Docker network:

```sh
docker run --rm --name doramcap-gallery-caddy --network host --read-only \
  --cap-drop ALL --cap-add NET_BIND_SERVICE --tmpfs /config --tmpfs /data \
  -v "$PWD/apps/project-explorer/server/caddy:/etc/caddy:ro" \
  caddy:2 caddy run --config /etc/caddy/Caddyfile
```

Open <http://127.0.0.1:8096/#/apps/activity-demo>. Launch opens a viewer in the
same tab. Each viewer renews its own session; the default heartbeat is 15 seconds
and expiry is 120 seconds. `--heartbeat-seconds` and `--idle-seconds` configure
them; heartbeat must be positive and shorter than expiry. A suspended viewer
can expire and show a relaunch action. The gallery tab alone does not keep an app
running. The viewer offers the finalized recording after the generation stops.

The data directory retains each generation's recording, frozen Compose manifest
and categorized filesystem metadata. Temporary container resources are removed
only after writers stop. The server refuses a second manager for the same data
root and checks Docker for manager-owned resources before accepting launches.

## App registrations

| App ID | Selection | Retained result | Wrapper guide |
| --- | --- | --- | --- |
| `activity-demo` | Default | Activity MCAP | [Activity Lab](../../../activity-demo/deploy/README.md) |
| `storage-demo` | Opt-in | Activity and storage MCAPs | [Storage Observatory](../../../storage-demo/deploy/README.md) |
| `cpu-viewer` | Opt-in, native BPF | CPU MCAP | [CPU Viewer](../../../cpu-viewer/deploy/README.md) |
| `mcap-recorder` | Headless; opt-in, synthetic | Recording MCAP | [MCAP Recorder](../../../mcap-recorder/deploy/README.md) |
| `system-monitor` | Headless; opt-in, synthetic | Protobuf; optional MCAP and experimental native profiles | [System Monitor](../../../system-monitor/deploy/README.md) |
| `fault-demo` | Opt-in | Final state | [Fault Demo](../../../fault-demo/deploy/README.md) |
| `fault-watchdog` | Opt-in dashboard | Final state; optional MCAP or OTLP profile | [Fault Watchdog](../../../fault-watchdog/deploy/README.md) |
| `provenance-demo` | Opt-in | Summary | [Provenance Lab](../../../provenance-demo/deploy/README.md) |
| `orchestrator-demo` | Pending | CPU recordings when qualified | [Orchestrator Lab](../../../orchestrator-demo/deploy/README.md) |
| `cuda-shm-demo` | Explicit enable; NVIDIA GPU | Per-run MCAP/Protobuf traces | [CUDA demo](../../../cuda-shm-demo/deploy/README.md) |

Build each image independently. The manager scans `apps/*/deploy/registration.pbtxt`
using the [registration schema](../proto/registration.proto). It loads trusted files at startup; opt-in apps require `--enable-app ID`. Install one profile
registration per app before startup. Adding a supported registration requires a
restart, not a manager rebuild. See [Registration](../src/registry.rs) and the
[future-app guide](docker-hostable-apps.md) for fields and result declarations.

Headless apps have no gallery Start button. Declared finalized results are
published; operational logs and filesystem reports remain private. Registrations
can declare exclusive host resources to prevent conflicting launches.

## Prefix hosting

Set `GALLERY_PREFIX` to a path without a trailing slash and use the prefix config.
For example, add `-e GALLERY_PREFIX=/gallery` to the Caddy container command and
replace its config with `/etc/caddy/Caddyfile.prefix`. Open
<http://127.0.0.1:8096/gallery/#/apps/activity-demo>. `--origin` remains the
scheme and host without the prefix. The proxy URL, listening interface and backend address can be changed with
`GALLERY_BIND`, `GALLERY_LISTEN` and `GALLERY_API`, respectively. The default
listener is loopback. For a trusted Tailscale network, set `GALLERY_LISTEN` to
the host’s Tailscale IPv4 address, `GALLERY_BIND` to that address and port, and
the server’s `--origin` to the matching HTTP origin. Every permitted peer can
launch registered apps; apply an appropriate Tailscale access policy.

## Recovery

An unexpected manager exit can leave a generation running. Restarting the manager
reports its labeled resources and blocks replacement. Stop the manager, then use
the 32-character generation from its diagnostic:

```sh
target/debug/recover "$PWD" /absolute/path/gallery-data activity-demo GENERATION /var/run/docker.sock
```

Replace `activity-demo` with the affected app ID and `GENERATION` with its saved
32-character identifier. Recovery reads the trusted registration for that app.

Recovery verifies the saved project identity, stops workloads before their
supporting services, and collects metadata before removing owned resources. A
synced private checkpoint preserves completed container-layer evidence. Missing
evidence or incomplete nested volume coverage retains resources and blocks the
owning app. Results and diagnostic logs remain in the data directory; only
successfully finalized, declared results are downloadable. Investigate reported
errors before retrying. Unknown ownership blocks all apps until resolved.
