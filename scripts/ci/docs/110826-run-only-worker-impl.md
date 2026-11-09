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
- [ ] Type checking and snapshot inventory coverage on the final revision.
- [ ] Publish draft PR before remote end-to-end validation.
- [ ] Run all three archives on `.22`; record counts and gaps below.
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

Remote full-suite validation pending. The initial smoke test establishes ARM64
binary compatibility; it does not establish a CI pass or native BPF support.
