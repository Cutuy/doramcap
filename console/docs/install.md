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
Desktop launches attempt to open the access link as the invoking user;
`--no-open` suppresses this. `status` prints the current private link and SSH
instructions. Keep the forwarding terminal open while using a remote Console.
The browser signs in automatically. Links and sessions expire on restart.

Setup creates `/var/lib/doramcap-console/console.pbtxt`, a private playground
copy and run data. `--state /absolute/path` selects a different root-owned,
mode-0700 state directory. Only one Console service runs at a time.
Edit the pbtxt with sudo to select other project roots or system apps.
Selected missing images download when entering Review; later runs use Docker's
cache. Download failures leave the run unstarted and can be retried through Review.

Stop Console before upgrading. Each release installs into its own directory;
setup preserves the existing pbtxt, playground and recordings. Previous versions
remain under `/opt/doramcap-console/releases`; an administrator can switch the
`current` symlink back while Console is stopped. Inspect service failures with
`sudo journalctl -u doramcap-console.service`.

## Publishing

Build each architecture natively with the Node version in `.node-version`.
Supply a dedicated playground containing public release inputs and a pbtxt
`ConsoleConfig` template. Template project, graph and app input paths are relative
to the playground; runtime data and access credentials are generated locally.
Pin Dora and all distributed Compose images by digest. Publish those images to
ECR Public before publishing release archives.

```sh
node console/scripts/package.mts --output /tmp/console-amd64 --version 0.1.0 \
  --node /path/to/node --node-license /path/to/node/LICENSE \
  --playground /path/to/release-playground --config /path/to/release-console.pbtxt
node console/scripts/installer.mts --base-url https://downloads.example.com/0.1.0 \
  --version 0.1.0 --amd64-sha256 AMD64_HASH --arm64-sha256 ARM64_HASH \
  --output /tmp/install.sh
```

Upload both archives and the generated installer to the versioned R2 download
prefix. The installer embeds both archive hashes; serve it over HTTPS. Publish
checksums and signatures through your release process. Account credentials stay
in publisher tooling. The CLI needs no AWS or Cloudflare account.
