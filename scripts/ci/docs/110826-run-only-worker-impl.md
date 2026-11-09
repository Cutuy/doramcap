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
- [x] Verify the approved 64 MiB soft/hard memlock limit and 1 GiB RAM limit.
- [x] Package the approved Docker CLI and Compose executables with archive checksums.
- [x] Repeat all suites after CLI provisioning and the memlock change.
- [x] Review against implementation/design guides.
- [x] Confirm all six hosted checks.

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
The builder also packages its Docker CLI and Compose plugin. Both install into
the runner's home directory and their versions appear in the retained log.
No application dependencies or environment inputs are added.

## Validation record

Final code: `18f26316ef5704df24e9f2aba0639e59173c0d9a`. WSL Pueue 623 built
the native ARM64 archives in 34 seconds using its warm cache; the cold-cache
trial took 3m43s. Archives and runtime tools total about 1.7 GiB. SCP `-3`
streams them through the submitter, which stores only the 2.1 MiB source archive.

The Mac mini completed all three suites with zero cgroup OOM kills:

| Suite | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: |
| Default workspace | 453 | 0 | 99 |
| All-feature workspace | 459 | 0 | 99 |
| Storage without defaults | 65 | 0 | 1 |

The complete submit/build/transfer/execute command returned 0. The runner has
nextest 0.9.151, Docker CLI 29.9.2 and Compose 5.6.1; installed CLI hashes match
the builder's executables. The user raised soft/hard memlock from 8 to 64 MiB
and approved CLI transfer, resolving the first trial's allocation and preparation
failures. The 1 GiB RAM limit remains in place; Cargo and Node are absent.

Source and results: `~/jobs/c6a3e8469dc74a5a92e74d872ebb0bd7` on each host.
The runner retains `run.log`, all-zero `results.tsv` and `exit-code` (0).
Local checks passed: 20 CI-tool tests, TypeScript, shell syntax, boundaries and
diff whitespace. Corrupted-archive rejection, stale-result reset and failed-build
transfer gating also passed. Earlier SSH interruptions were not reproduced by
the final compact-result workflow; its full logs remain on the runner.

The skipped tests retain their existing ignore status. Privileged native BPF,
doctests, Dora and CUDA qualification remain separate checks. Hosted Rust, Node
and MCAP checks passed on both ARM64 and x86_64.
