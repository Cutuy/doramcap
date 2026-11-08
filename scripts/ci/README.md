# CI

GitHub Actions runs these suites on ARM64 and x86_64 Ubuntu runners:

| Suite | Coverage |
| --- | --- |
| [Rust](../../.github/workflows/ci.yml) | Library boundaries, formatting, default/all-feature Clippy and tests, storage without default features, snapshot synchronization. |
| [Node](../../.github/workflows/node.yml) | App and Console tests, Console builds/type checks and HTTP integration, Project Explorer build. |
| [MCAP](../../.github/workflows/mcap.yml) | Real-writer recordings checked by Rust protobuf and TypeScript streaming readers. |

The workflows define path filters and exact commands. Rust uses the `ci` Cargo
profile; runtimes are pinned in [rust-toolchain.toml](../../rust-toolchain.toml)
and [.node-version](../../.node-version). CI tools run directly as TypeScript using
[shared tooling libraries](../../libs/tooling/README.md), Git, SSH/SCP, and rsync.
The worker also uses system tar to extract submitted archives.

## ARM64 worker

`doramcap-ci@100.64.0.20` is an Ubuntu 24.04 / WSL2 worker reached over Tailscale.
Keep WSL running. Pueue runs jobs serially in its default queue.
Submit worker jobs explicitly.

```sh
npm --prefix libs/tooling ci --ignore-scripts --no-audit --no-fund
node scripts/ci/submit.mts --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue follow JOB_ID
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

Replace `JOB_ID` with the returned ID. On the worker, use `pueue wait JOB_ID` to await completion,
`pueue restart JOB_ID` to rerun the snapshot, or `pueue status --json` for automation.

- Submit trusted code only: the native-test launcher gives submitted code effective root access to the worker.
- [Submission](submit.mts) includes uncommitted source and a `SOURCE.json` manifest, excluding Git-ignored files and `.git`.
- [Jobs](run-worker-job.sh) run Rust, Node and MCAP checks plus privileged native cgroup/allocation tests. Rust uses the dev profile with line-table debug information.
- Dora recording checks require the CLI at `~/.local/bin/dora`. Use a CUDA-capable host for GPU/IPC qualification. Project Explorer builds in GitHub CI because it needs Git metadata.
- [Cache preparation](prepare-build.mts) verifies snapshots and synchronizes a locked source cache before tooling dependencies are installed. Jobs share `~/target`; snapshots stay in `~/jobs`.
- Logs and snapshots remain until manually removed. `pueue clean` removes finished queue records/logs; remove snapshots separately. Keep active snapshots and the shared target intact during jobs.
- After `pueue kill JOB_ID`, inspect leftover `/sys/fs/cgroup/doramcap-validation-*` groups before more native tests. Resource-policy tests temporarily change root cpuset delegation; use an authorized serial test host.

Boundary rules use standard minimatch syntax; matching is case-sensitive.

Run focused CI-tool tests locally with the pinned Node runtime, Git, GNU tar, and rsync:

```sh
npm --prefix libs/tooling ci --ignore-scripts --no-audit --no-fund
npm --prefix libs/tooling test
npm --prefix libs/tooling run typecheck
node --test scripts/ci/test-boundaries.mts scripts/ci/test-prepare-build.mts scripts/ci/test-submit.mts
```

## Provisioning

Use ARM64 Ubuntu with systemd, SSH, kernel BTF, cgroup v2 and outbound download
access. Copy [install-worker.sh](install-worker.sh), the SSH **public** key and
[.node-version](../../.node-version) to the host, then run as an administrator:

```sh
sudo bash install-worker.sh /path/to/authorized-key.pub /path/to/.node-version
sudo systemctl status doramcap-pueued
sudo journalctl -u doramcap-pueued
```

The installer creates the CI account, runtimes, build dependencies, queue service
and sudo launcher. It replaces the account's authorized keys and Pueue config and
restarts the daemon; reprovision only while idle. Configure SSH and WSL startup
separately. Restrict CI access to trusted users.

See [AGENTS.md](../../AGENTS.md#testing-and-ci) for validation policy and the
[Storage Demo README](../../apps/storage-demo/README.md) for manual HTTP checks.
