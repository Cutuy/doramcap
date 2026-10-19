# ARM64 test worker

Worker: `doramcap-ci@100.64.0.20`, ARM64 Ubuntu 24.04 / WSL2 Linux 6.18.
[Pueue 4.0.4](https://github.com/Nukesor/pueue/releases/tag/v4.0.4) runs as a
systemd service, with one job at a time and a private Unix socket. SSH provides
remote access; no Pueue TCP service is exposed.

## Submit and inspect

From a checkout containing the native validation example and script:

```sh
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue follow 0
ssh doramcap-ci@100.64.0.20 pueue log --full 0
ssh doramcap-ci@100.64.0.20 pueue wait 0
ssh doramcap-ci@100.64.0.20 pueue kill 0
```

Replace `0` with the returned job ID. Inspect `pueue status --json` for
automated success/failure handling. Use `pueue restart ID` to rerun the same
snapshot. Git pushes do not submit jobs. Keep native test jobs in the serial
default group.

Submission copies an allowlisted snapshot of Cargo files, libraries and applications,
including uncommitted source changes, excluding Git-ignored files. It adds a
runner and a `SOURCE.json` manifest with HEAD, dirty status and per-file SHA256.
The transferred archive's checksum is verified before enqueueing. Passwords,
`.git`, and local build directories are not copied. Each job has a separate
snapshot writable by the trusted CI account.

Each job runs default and all-feature Rust tests, builds native binaries/examples,
then runs privileged cgroup accounting integration and allocation validation.
The worker also benchmarks storage allocations and runs synthetic/native Dora
recording comparisons when the pinned CLI is installed at
`/home/doramcap-ci/.local/bin/dora`. Native Dora validation runs through the
approved root launcher; synthetic validation runs unprivileged.

Persistent locations on the worker:

- `/home/doramcap-ci/jobs/`: source snapshots and manifests.
- `/home/doramcap-ci/target/`: shared incremental build artifacts.
- `/home/doramcap-ci/.cargo/`, `.rustup/`: downloads and pinned Rust 1.99.1.
- `/home/doramcap-ci/.local/share/pueue/`: queue state, logs and socket.
- `/etc/systemd/system/doramcap-pueued.service`: daemon service.
- `/etc/sudoers.d/doramcap-ci`: passwordless native-test permission.
- `/usr/local/libexec/doramcap-ci-native-tests`: root-owned test launcher.

Finished logs and snapshots are retained until manually removed. `pueue clean`
removes finished queue records/logs; source snapshots need separate cleanup.
Do not remove active snapshots or modify the shared target during a job.
Cancellation can interrupt cleanup in native tests; inspect leftover
`/sys/fs/cgroup/doramcap-validation-*` groups before further native tests.

## Provisioning another worker

Requirements: ARM64 Ubuntu, systemd, SSH, a Rust-compatible kernel with BTF and
cgroup v2, and outbound package/GitHub/Rust download access. Existing `.20` has
these platform prerequisites. Protobuf validation also requires `python3-protobuf`
(`sudo apt-get install python3-protobuf` on an existing worker). As an administrator, copy `install-worker.sh` and the intended SSH
**public** key, then run:

```sh
sudo bash install-worker.sh /path/to/authorized-key.pub
```

This creates the dedicated user, installs clang18/libelf/zlib/build tooling,
verifies pinned Pueue binary hashes, installs the service and sudo launcher, and
installs Rust unprivileged. It replaces this account's authorized_keys and Pueue
config; it restarts the daemon, so only reprovision while idle. Provision SSH
separately and keep the WSL distro running. Systemd starts Pueue when the distro
boots. Configure Windows startup and sleep policy separately if required.

Administrator service commands:

```sh
sudo systemctl status doramcap-pueued
sudo systemctl restart doramcap-pueued  # only while idle
sudo journalctl -u doramcap-pueued
```

The approved trust boundary is explicit: the launcher runs CI-owned scripts and
binaries as root. Anyone with CI SSH access or control over submitted source has
effective root access to this WSL distro. Restrict submissions to trusted code.
No sudo password is stored in the CI account or submitted snapshots.

## Component layout compatibility

Source snapshots include `libs/` and `apps/`. The native integration test lives
at `apps/system-monitor/tests/validate_native.py`; new privileged launchers use
that path. Submission adds a small forwarding script at the legacy path inside
the worker archive so previously installed launchers continue to work. This shim
exists only in the submitted snapshot. Python validation requires the protobuf
runtime described in `libs/telemetry-protocol/python/requirements.txt`.

Source archives carry the snapshot creation timestamp on every entry so Cargo
invalidates workspace artifacts in the shared target when a snapshot changes.

SSH/SCP submissions use connection and keepalive timeouts. The installed ARM64
Dora CLI is version 1.1.0, release archive SHA256
`77fe85ff1eaf2cac2bf9e0d46d1efe00cad5076b5a0cf4e42b2cf2c646b4752f`.

The storage-enabled runner also runs Clippy on changed storage/monitor components
with dependency linting excluded; generated protocol code has separate lint policy.

The storage-demo HTTP validation exercises live SSE, commands, aligned MCAP
rotation, tail inspection and independent payload readback on ARM64.

## Resource policy native coverage

The aggregate native suite also invokes `libs/resource-policy/tests/validate_native.py`
against the unprivileged-built `native_policy` example. It checks cpuset placement,
RR scheduling, inheritance, denial, contention, cleanup, and Python/Rust Protobuf
compatibility. Its `--enable-root-cpuset` test option temporarily enables root
cpuset delegation when absent, then restores the previous setting after removing
its test subtree. This host-level test setup was explicitly approved on 2026-10-19.
Run only on the trusted serial CI worker or an equivalently authorized test host.
