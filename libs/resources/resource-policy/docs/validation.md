# Resource policy validation

## Schema alignment — 2026-10-20

- Restored the `Placement` message and `Policy.placement` field expected by Rust, pbtxt profiles, and documented explicit-goal semantics; reserved the removed inheritance selector's tag/name.
- Seven local resource-policy tests, eighteen provenance tests, and focused strict Clippy passed.
- ARM64 job **72** passed the full workspace suite, native policy checks, Python/Rust interoperability, storage/activity smoke tests, and existing Dora recording checks.
- Snapshot: `/home/doramcap-ci/jobs/a89fe65641e840cea751e4a6ea4f716d`; verified final Pueue success and full log.

## Shared pbtxt resolver extraction

- `pbtxt-config` owns App/Fragment/Deployment, typed `Any` payloads, includes, provenance, and fingerprints. Resource policy provides the domain adapter; observations use `ThreadResources`.
- x86_64: ten resolver/policy tests, focused Clippy, full workspace all-feature tests, and native enforcement passed.
- ARM64 job **66** succeeded: workspace default/all-feature checks, native enforcement, Python/Rust interoperability, and smoke tests.
- All 20 implementation/schema/fixture/test files matched the worker snapshot. Native coverage retained goal expansion/narrowing, drift repair, inheritance, RR priorities 0/1/30, permission denial, and cleanup.

## Explicit goal-state revision

- `placement.cpus` is required after composition; missing or empty placement fails.
- Caller state supplies observations only. Thread and owned-cgroup placement converge to the configured goal.
- Seven behavioral tests and native x86_64 tests passed. Native coverage includes goal expansion, narrowing, and external cpuset drift repair.
- ARM64 job **64** succeeded: workspace default/all-feature checks, native regressions, and smoke tests. Root cpuset delegation was restored; source hashes matched that revision.

## Initial implementation evidence

Evidence date: 2026-10-19. Initial scope covers cpulist goals, ordinary/RR
scheduling, caller enforcement, cgroup bootstrap, and resource-only systemd directives.

| Check | x86_64 Linux 7.0.0-38 | ARM64 WSL2 Linux 6.18 |
|---|---|---|
| Workspace formatting, default/all-feature Clippy | Passed | Passed (job 60) |
| Workspace default/all-feature Rust tests | Passed | Passed (job 60) |
| Resource-policy behavioral tests (6) | Passed | Passed (job 60) |
| Native cpuset and RR 0/1/30 | Passed | Passed (job 60) |
| Thread/exec-child inheritance, idempotence, drift | Passed | Passed (job 60) |
| Parent restriction, ownership, cleanup | Passed | Passed (job 60) |
| Dropped-privilege denial and bounded contention | Passed | Passed (job 60) |
| Python/Rust pbtxt + binary compatibility | Passed | Passed (job 60) |

Commands (builds run unprivileged):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --no-deps --all-targets --locked -- -D warnings
cargo clippy --workspace --no-deps --all-targets --all-features --locked -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --all-features
cargo build --locked -p resource-policy --examples
cargo run --locked -p resource-policy --example policy_overhead
# Elevate only the prebuilt native suite using the repository credential workflow.
python3 libs/resources/resource-policy/tests/validate_native.py --binary target/debug/examples/native_policy
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
```

The local all-feature build used the existing `/tmp/doramcap-native/root` LLVM 18
and libelf headers/libraries through `CLANG`, `LD_LIBRARY_PATH`, `CFLAGS`,
`LIBRARY_PATH`, `PKG_CONFIG_PATH`, and `PKG_CONFIG_SYSROOT_DIR`. The default local
build environment lacked libelf headers. No system packages were installed.
The Python interoperability suite used `/tmp/resource-policy-python` with
Protobuf 6.33.6; builds remained unprivileged.

The ARM64 worker initially lacked root cpuset delegation (jobs 56/57). The user
explicitly approved temporary enable/restore for tests. Job 58 passed cpuset/RR
checks and restored the controller, then failed because the unprivileged probe
could not traverse the private build directory. The corrected probe drops
credentials after exec. The test harness always attempts restoration in cleanup. Final job **60** completed
successfully at 15:37:49 America/Los_Angeles. Its snapshot is
`/home/doramcap-ci/jobs/11962705900a45389f10c52463cc362f` on the worker.
The initial snapshot hashes were checked against resource-policy code, configs, Cargo
manifests/lockfile, native-suite integration, and worker runner. The log confirms
`PASS native resource-policy on aarch64 6.18.54.1-microsoft-standard-WSL2` and
restoration of the original root cpuset setting. The full worker job also passed
native cgroup accounting, allocation checks, storage smoke tests, and Dora recording checks.

Informational debug-build overhead, 100 iterations on x86_64: composed resolution
468.39 microseconds mean; already-satisfied thread apply 49.94 microseconds mean.
ARM64 debug means were 519.20 microseconds for resolution and 59.03 microseconds
for satisfied apply, also over 100 iterations. These include allocation and sysfs reads and provide startup-path observations.
They establish no latency bound or production real-time guarantee.

## Dependency decisions

Candidates were presented in the implementation discussion before adoption.
The user authorized implementation after reviewing the plan.

| Dependency | Decision |
|---|---|
| Prost / prost-build 0.14 | Reuse the workspace Protobuf toolchain and schema generation. |
| prost-reflect 0.16.5 | Use maintained descriptor-backed text parsing; avoid a custom pbtxt parser. |
| cpulist 1.1.11 | Use Folo's parser/emitter, with prevalidation that bounds expansion and limits accepted syntax. |
| libc 0.2 | Reuse the existing workspace dependency for dynamic affinity masks and scheduler calls. |
| sha2 0.10.9 | RustCrypto SHA-256 for the canonical resolved-policy fingerprint. |
| thiserror 2 | Structured errors consistent with Rust error sources. |
| nix | Evaluated; fixed-size affinity wrapper and scheduler coverage require additional lower-level calls. |
| cgroups-rs | Evaluated; initial implementation uses a narrow cgroup v2 filesystem adapter with explicit ownership, preparation, and verification. |
| Figment | Evaluated; typed field-presence overlays avoid converting the Protobuf presence/default contract through a generic provider representation. |

Dependency versions and transitive integrity checks are pinned in Cargo.lock.
The initial parser/overlay code remains bounded and supports precisely the
schema's fields; there is no generic configuration language.

## Remaining scope

- NVIDIA BSP qualification and long-duration real-time load qualification.
- GPU, bandwidth, exclusivity, and resource budgeting.
- Full orchestration generators, deployment activation, and Kubernetes runtime adapters.
- Existing multithreaded-process reconciliation,
  continuous reconciliation, and dynamic shared-library loading.
- Cross-language policy constraints beyond structural parsing; each language must
  honor the documented cpulist/priority/composition contract or consume resolved policies.

Existing app startup and Dora dataflows are unchanged. The native suite integration
is the only modification under an existing app directory.
