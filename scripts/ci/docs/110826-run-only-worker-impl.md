# Run-only ARM64 worker implementation

Created: 2026-11-08. [Architecture](110826-run-only-worker.md).

## Milestone 1: submission and worker execution

**Work items**

- [x] Reuse the source snapshot and manifest verifier.
- [x] Queue native builds through WSL Pueue; require that exact job to succeed.
- [x] Pin and checksum nextest 0.9.151 for both ARM64 workers.
- [x] Transfer all three suites and source, preserving fixture paths.
- [x] Serialize runner execution and retain logs and per-suite exit status.

**Validation**

- [x] Existing WSL-built protocol executable: 3 tests passed on `.22`.
- [x] Monitor ABI smoke check succeeds; BPF attachment fails on the unavailable kernel symbol.
- [x] Focused CI-tool tests and shell syntax checks.
- [x] Type checking and snapshot inventory coverage.
- [x] Publish draft PR before remote end-to-end validation.
- [x] Run all three archives on `.22`; record counts and gaps below.
- [ ] Resolve missing Compose CLI and the registered-I/O memlock limit; repeat validation.
- [ ] Review against implementation/design guides, mark ready and release lease.

## Mechanisms

`submit-run-only.mts` stages a job under each account's `~/jobs`, waits for its
Pueue build and transfers the checksummed artifacts through the submitter.
SCP's `-3` mode streams archives through the submitter without local artifact
staging. The builder and runner need no SSH identity for one another.

`run-only-worker.sh` uses Bash because the compiler-free runner has no Node.
The build mode reuses the dependency-free TypeScript source verifier. The run
mode extracts the same source and nextest target paths, runs with one test
thread and writes `run.log`, `results.tsv` and `exit-code`. Each suite starts
with an empty extracted target directory. Source snapshots and archives remain
in the per-job directory; `/tmp` holds replaceable caches. Remove only idle
CI-owned job/cache directories when reclaiming disk.

`install-nextest.sh` installs into the builder's `~/.local/bin` without sudo or
services. The verified executable travels with the archives and installs on the
runner. Its release SHA256 is pinned from the upstream GitHub release asset digest.
No application dependencies or environment inputs are added.

## Validation record

WSL Pueue 622 built source `9a0a8471` in 30 seconds using its warm cache.
The cold-cache trial (621) took 3m43s. Archives total about 1.6 GiB; SCP `-3`
streams them without requiring that much free space on the submitter.

The Mac mini completed all three suites with zero cgroup OOM kills:

| Suite | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: |
| Default workspace | 438 | 15 | 99 |
| All-feature workspace | 443 | 16 | 99 |
| Storage without defaults | 62 | 3 | 1 |

Eight Compose preparation tests per workspace suite fail because Docker/Compose
CLI tools are absent. Recorder/storage tests return `ENOMEM` while registering
I/O buffers under an 8 MiB hard memlock limit. Raising the limit and transferring
the existing CLI tools require user approval; no tests have been excluded.
This is a completed execution trial with failures, not a full CI pass.

Source and results: `~/jobs/45a76376ddce49f8bfbeed7794f764ef` on each host.
The runner retains `run.log`, `results.tsv` and `exit-code` (1).
Corrupted-archive rejection, stale-result reset and failed-build transfer gating
also passed. SSH intermittently stalled during handshakes and output streaming;
the retained files confirmed execution completion. Key-exchange diagnosis is
unconfirmed. The latest coordinator returns compact statuses and keeps verbose
logs on the runner. Native BPF remains unsupported in this container.
