# WSL2 ARM64 CI

`doramcap-ci@100.64.0.20` runs native Rust, Node, MCAP and privileged tests.
Keep WSL running. Pueue executes one job at a time.

```sh
npm --prefix scripts/ci ci --ignore-scripts --no-audit --no-fund
node scripts/ci/wsl2/submit.mts --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

- Submit trusted source: the existing native-test launcher gives it effective root access.
- [The worker](worker.mts) builds unprivileged with Cargo's dev profile and line-table debug information. Native tests use the installed sudo launcher.
- [Snapshots](../snapshot.mts) include uncommitted source and `SOURCE.json`, excluding Git-ignored files and `.git`. [Preparation](../prepare-build.mts) verifies a locked source cache. Jobs share `~/target`; snapshots and logs stay in `~/jobs` until removed manually.
- Use `pueue wait JOB_ID`, `pueue follow JOB_ID` or `pueue restart JOB_ID`. Keep active snapshots and the target cache intact. `pueue clean` removes finished queue records/logs.
- After `pueue kill JOB_ID`, inspect leftover `/sys/fs/cgroup/doramcap-validation-*` groups before further native tests. Resource-policy tests temporarily change root cpuset delegation; use an authorized serial host.
- Dora checks require `~/.local/bin/dora`. CUDA qualification needs a CUDA-capable host. Project Explorer builds in hosted CI because it needs Git metadata.

## Provisioning

Use ARM64 Ubuntu with systemd, SSH, kernel BTF, cgroup v2 and outbound downloads.
Copy [install-worker.sh](install-worker.sh), the SSH public key and
[.node-version](../../../.node-version), then run as administrator:

```sh
sudo bash install-worker.sh /path/to/authorized-key.pub /path/to/.node-version
sudo systemctl status doramcap-pueued
sudo journalctl -u doramcap-pueued
```

The installer creates the account, dependencies, service and sudo launcher,
replaces authorized keys/Pueue configuration and restarts the daemon. Reprovision
while idle; configure SSH and WSL startup separately. Restrict access to trusted users.

See the [worker record](docs/101826-arm64-pueue-worker.md) and
[shared CI instructions](../README.md).
