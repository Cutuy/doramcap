# Console installation

Requires Ubuntu 24.04 or newer on x86_64 or ARM64, Docker Engine, Compose v2, rsync,
systemd, cgroup v2 and OverlayFS.

Download the published `install.sh`, inspect it, then run `sh install.sh` as your
normal user. It verifies the archive and installs into `/opt/doramcap-console` with sudo.

```sh
doramcap-console doctor
doramcap-console start
doramcap-console --help
```
