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

After installing the release files and loading its images:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now doramcap-gallery doramcap-gallery-caddy
```

See [hosting](../../docs/hosting.md) for updates and public HTTPS, and the
[EC2 qualification record](../../docs/103126-ec2-app-qualification.md) for tested apps.
Keep credentials and connection details outside Git.
