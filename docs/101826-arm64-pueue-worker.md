# ARM64 Pueue test worker

## Design

Use `doramcap-ci@100.64.0.20` with SSH keys, Pueue over its local Unix socket,
and a systemd daemon running as that dedicated user. One queued job at a time;
persistent Cargo downloads and target cache. Jobs build unprivileged and use
passwordless sudo only for the native-test launcher. That launcher executes
CI-owned code with effective root access. Restrict submissions to trusted code.
No credentials enter source snapshots.

Source snapshots are separate per job; record source provenance and kernel/tool
versions in logs. Pueue supplies queueing, status, cancellation, and retained logs;
Git push triggers and hosted CI integration are outside this milestone.

## Milestones and validation

- [x] Install pinned Pueue, native build dependencies, dedicated account and SSH.
  Validate versions, direct key login, and unprivileged build tools.
- [x] Install systemd daemon and explicit native-test sudo launcher.
  Validate enabled/active service, serial queue, passwordless launcher, and no TCP listener.
- [x] Add source submission and worker job scripts.
  Validate snapshot provenance, ordinary Rust tests and privileged native tests through Pueue.
- [x] Document operation and limitations.
  Validate status/log commands and service restart persistence after jobs complete.

Current native validation source comes from the existing native-kernel-validation
worktree. Its 10 Hz rstat flush design remains unresolved; setting up this worker
does not change or endorse that design. Linux 6.18 WSL2 testing does not qualify
Linux 6.8 or Jetson hardware.

## Validation results — 2026-10-18

Installed on ARM64 `.20`: Pueue/pueued 4.0.4 (release SHA256 verified), Rust
1.99.1, clang18 and native development headers. SSH public-key login as
`doramcap-ci` works; the daemon runs as UID 1001. Service is enabled and active;
Unix socket has mode0700, and there is no Pueue TCP listener.

Pueue job 0 succeeded in approximately 101 seconds including a cold build:
12 default Rust tests, 11 all-feature tests, native 65-snapshot accounting test,
and 40 steady-state collection iterations with 0 userspace allocations. Native
collection median 0.528ms; sampling interval median 99.997ms. Allocation probe
median 485µs, max 1821µs. These numbers include the current forced rstat flush.

Source snapshot: `/home/doramcap-ci/jobs/ce64840ac2004c29abc5766200130f4d`.
Its `SOURCE.json` records the exact dirty native-validation source and file
hashes. Job 1 intentionally exited 7 and was correctly recorded as failed;
job 2 subsequently succeeded as UID 1001. Recorded start/end times show serial
execution. Restarting the idle daemon retained all 3 results and concurrency 1.

Local validation: shell syntax checks, submission CLI execution, and diff
whitespace checks passed. See [worker instructions](../scripts/ci/README.md).
Dora end-to-end validation is not part of this queued job yet. Windows/WSL
startup scheduling and automatic Git triggers are not configured.
