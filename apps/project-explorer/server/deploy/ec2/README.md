# Small ARM64 EC2 host

Prebuilt manager, gallery assets, selected app registrations and Docker images
run under systemd. The source revision and image identities accompany each
release. See the [deployment plan](../../docs/103126-arm64-deployment.md)
for build/runtime evidence and [hosting](../../docs/hosting.md) for lifecycle behavior.

## Layout

| Path | Contents |
|---|---|
| `/opt/doramcap-gallery/releases/REVISION/` | Root-owned release bundle |
| `/opt/doramcap-gallery/current` | Active release symlink |
| `/var/lib/doramcap-gallery/` | Gallery-owned private results and manager lock |
| `/tmp/doramcap-experiments-*` | Lifecycle admission and reset state |

Provision Docker Engine/Compose, a `gallery` system account in the `docker`
group, and a 1 GiB swap file. Install these units in `/etc/systemd/system/`.
Docker group access grants control of the host; keep the account dedicated.

Each bundle contains `bin/`, `assets/`, `workspace/apps/activity-demo/deploy/`,
`caddy/`, `images/arm64.tar.gz`, `SOURCE_REVISION`, `IMAGES`, and `SHA256SUMS`.
Build binaries and app assets on ARM64 Ubuntu 24.04 with the pinned toolchain;
use `--release --locked` for Rust and `npm ci` for frontends. The gallery frontend
build also needs workspace Cargo metadata. The app Dockerfile stages prebuilt
inputs; its Compose image name must match the imported tag.

## Activate or update

Verify the archive checksum supplied through the trusted SSH connection. Extract
it into a new revision directory, then verify its contents there:

```sh
sha256sum -c SHA256SUMS
sudo docker load -i images/arm64.tar.gz
cat SOURCE_REVISION IMAGES
```

Before importing an update, stop Caddy and the manager so active apps drain.
Preserve the previous release and image IDs for rollback. With `REVISION` set to
the extracted directory's basename:

```sh
sudo systemctl stop doramcap-gallery-caddy doramcap-gallery
sudo ln -sfn "/opt/doramcap-gallery/releases/$REVISION" /opt/doramcap-gallery/current
sudo systemctl daemon-reload
sudo systemctl enable --now doramcap-gallery doramcap-gallery-caddy
sudo systemctl status doramcap-gallery doramcap-gallery-caddy
curl --fail http://127.0.0.1:8096/api/hosting/apps
```

For rollback, stop both services, reload the previous bundle's images to restore
the Compose/Caddy tags, switch `current` to that release, and start both services.
Keep the previous workspace and private recordings until recovery is verified.

## Access and maintenance

Keep inbound access restricted to SSH from the operator's IP. On the operator's
machine:

```sh
ssh -i ~/.ssh/KEY_NAME.pem -N -L 8096:127.0.0.1:8096 ubuntu@HOST_IP
```

Open <http://127.0.0.1:8096/#/apps>. That exact origin matches the manager unit.

```sh
sudo journalctl -u doramcap-gallery -u doramcap-gallery-caddy -n 100
sudo docker stats --no-stream
free -h
df -h /
```

Recordings persist across service restarts. Review disk use and remove only
unneeded, finalized generation directories while the manager is stopped;
automatic retention is deferred. Keep connection details and credentials outside
Git. Other apps require their own images and host qualification before enablement.
