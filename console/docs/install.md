# Console installation

Console release builds target Ubuntu 24.04 or newer on x86_64 and ARM64
(glibc 2.39 or newer). Release archives include
Node, the native bridge, the browser UI and a versioned playground.

Download the published `install.sh`, inspect it, then run `sh install.sh` as your
normal user. It selects the host architecture, verifies the archive against its
embedded SHA256 and requests sudo to install into `/opt/doramcap-console`.
Docker Engine, Compose v2, rsync, systemd, cgroup v2 and OverlayFS are required.

```sh
doramcap-console doctor
doramcap-console start
# Alternatively, start on a headless host to print a command to run on the browser computer.
doramcap-console start --ssh alice@workstation
# Choose exact host and browser ports when needed.
doramcap-console start --ssh alice@workstation --port 43189 --browser-port 45231
doramcap-console status
doramcap-console stop
```

`start` launches a transient system service that survives SSH disconnection.
Stop the service before changing its port or SSH launch options.
A fresh desktop start attempts to open the access link as the invoking user;
`--no-open` suppresses this. Repeated starts and `status` print the current
private link and SSH instructions and never open a browser. Keep the forwarding
terminal open while using a remote Console.
The browser signs in automatically. Links and sessions expire on restart.

| Command | Reports |
| --- | --- |
| `doctor` | Every host requirement, with a `Fix:` line for each failure. Kernel BTF and Docker disk space only warn. |
| `status` | Release, start time and access link. Exits 3 when stopped. |
| `stop` | Stops any active run, finalizes it and removes the access link. Succeeds when already stopped. |

A failed `start` prints the service's recent log.

Setup creates `/var/lib/doramcap-console/console.pbtxt`, a private playground
copy and run data. `--state /absolute/path` selects a different root-owned,
mode-0700 state directory. Only one Console service runs at a time.
Edit the pbtxt with sudo to select other project roots or system apps.
Selected missing images download when entering Review; later runs use Docker's
cache. **Cancel** stops a download, and each image has a 30-minute limit. Failures
leave the run unstarted, report the image in the service log and retry through Review.

## Shipped apps

| App | Launch | Output |
| --- | --- | --- |
| CPU monitoring (default) | `system-monitor` with [monitor.pbtxt](../release/playground/system-apps/cpu-monitor/monitor.pbtxt) | `cpu.mcap`; `live_outputs` for the live bridge |
| Dora topic recording | `mcap-recorder` with [recorder.pbtxt](../release/playground/system-apps/cpu-recorder/recorder.pbtxt) | `cpu-topics.mcap` from the monitor's Dora outputs; select CPU monitoring too |
| Sample workload | [workloads/cpu-burn](../release/playground/workloads/cpu-burn/compose.yaml) | One busy process to observe |

Stop Console before upgrading. Each release installs into its own directory;
setup preserves the existing pbtxt, playground and recordings. Previous versions
remain under `/opt/doramcap-console/releases`; an administrator can switch the
`current` symlink back while Console is stopped. Inspect service failures with
`sudo journalctl -u doramcap-console.service`.

## Publishing

Build each architecture natively with the Node version in `.node-version`.
Copy [console/release](../release) and replace each `:local` image with its
published digest; packaging rejects unpinned images. Template project, graph and
app input paths are relative to the playground; runtime data and access credentials
are generated locally. Build `system-monitor` with `--features bpf,task-bpf,dora`.
Publish all images to ECR Public before publishing release archives.

```sh
node console/scripts/package.mts --output /tmp/console-amd64 --version 0.1.0 \
  --node /path/to/node --node-license /path/to/node/LICENSE \
  --playground /path/to/release/playground --config /path/to/release/console.pbtxt
node console/scripts/installer.mts --base-url https://downloads.example.com/0.1.0 \
  --version 0.1.0 --amd64-sha256 AMD64_HASH --arm64-sha256 ARM64_HASH \
  --output /tmp/install.sh
```

Upload both archives and the generated installer to the versioned R2 download
prefix. The installer embeds both archive hashes; serve it over HTTPS. Publish
checksums and signatures through your release process. Account credentials stay
in publisher tooling. The CLI needs no AWS or Cloudflare account.
