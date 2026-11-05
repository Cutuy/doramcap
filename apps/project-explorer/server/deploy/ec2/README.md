# ARM64 EC2 gallery

Run prebuilt gallery assets, the manager and app images under systemd.
Install Docker Engine/Compose and create a dedicated `gallery` account with
Docker access. Build on ARM64 Ubuntu 24.04 using the pinned toolchain.

Reference host: `t4g.micro` (Graviton2 ARM64, 2 vCPUs, 1 GiB RAM),
Ubuntu 24.04 LTS, 30 GiB EBS with ext4, 1 GiB swap, and no GPU.

Install the [manager](doramcap-gallery.service) and
[proxy](doramcap-gallery-caddy.service) units in `/etc/systemd/system/`.
Their default endpoint is <http://127.0.0.1:8096/#/apps>.

| Host path | Purpose |
|---|---|
| `/opt/doramcap-gallery/current` | Symlink to the active release: `bin/`, `assets/`, `caddy/` |
| `/opt/doramcap-gallery/workspace` | Stable app deployment files |
| `/var/lib/doramcap-gallery` | Private, persistent results owned by `gallery` |
| `/etc/doramcap-gallery/caddy/` | Public hostname and proxy routes |
| `/etc/systemd/system/doramcap-gallery*.service.d/` | Operator service overrides |
| `/var/lib/doramcap-caddy/{data,config}` | Persistent certificate state |
| `/opt/doramcap-gallery/rollback/` | Operator rollback copies and validation receipts |

After installing the release files and loading its images:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now doramcap-gallery doramcap-gallery-caddy
sudo systemctl cat doramcap-gallery doramcap-gallery-caddy
sudo journalctl -u doramcap-gallery -u doramcap-gallery-caddy -n 50
```

See [hosting](../../docs/hosting.md) for updates and public HTTPS, and the
[EC2 qualification record](../../docs/103126-ec2-app-qualification.md) for tested apps.
Keep a private off-host operator record at `~/.local/share/doramcap-gallery/README.md`
with SSH/key references, the active origin, DNS/TLS settings and exported Caddy/systemd
configuration. Keep key material in its existing secure stores. Refresh the record
after configuration changes. `current/SOURCE_REVISION`, `VERSION` and `IMAGES.json`
identify the installed release.
