# Console installation

Requires Ubuntu 24.04 or newer on x86_64 or ARM64, Docker Engine, Compose v2, rsync,
systemd, cgroup v2 and OverlayFS.

Download `install.sh`, inspect it, then run it as your normal user. It verifies the
archive and installs into `/opt/doramcap-console` with sudo.

```sh
curl -fLO https://github.com/Cutuy/doramcap/releases/latest/download/install.sh
sh install.sh
doramcap-console doctor
doramcap-console start
doramcap-console --help
```
