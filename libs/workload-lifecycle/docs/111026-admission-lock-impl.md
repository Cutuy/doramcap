# Shared admission lock: implementation plan

Created November 10, 2026. Status: implemented November 11, 2026; validation recorded below.
[Architecture](111026-admission-lock.md).

Move repository admission into `workload-lifecycle` and make Compose and systemd
use it. One pull request, three milestones. The Holoscan runner takes the same
lock when it is built.

## Previous code

| Workflow | Code | Behavior |
| --- | --- | --- |
| Compose | `Controller::acquire` in [`reset.rs`](../../workload-docker/src/workflows/recorded_compose/reset.rs) | Locks `controller.lock` in a private per-user folder under `/tmp`. Clears close-on-exec so helper commands keep the lock. Error: "another experiment controller is active". |
| systemd | `lock_scope` in [`runner.rs`](../../workload-systemd/src/runner.rs) | Locks `lock` under `/run/doramcap/<repository hash>`. Taken by runs and operator cleanup. Error: "another run owns this repository". |

Both used `std::fs::File::try_lock`. Both folders also hold other run state, which
stays where it is.

## API

A new module, `workload_lifecycle::admission`.

```rust
/// Exclusive ownership of one repository for a run, build or cleanup.
/// Released after this holder and any inheriting children exit.
pub struct Admission { /* canonical root and the open folder */ }

impl Admission {
    /// Lock the canonical `repository` folder. Returns `Busy` at once when another holder exists.
    pub fn acquire(repository: &Path) -> Result<Self, Refusal>;
    /// Canonical repository root this admission covers.
    pub fn repository(&self) -> &Path;
    /// Let child processes started from now on keep the lock after this process exits.
    pub fn keep_across_exec(&self) -> std::io::Result<()>;
}

pub enum Refusal {
    /// Another run, build or cleanup holds this repository.
    Busy,
    /// Invalid, inaccessible or unsupported repository; includes the input and cause.
    Invalid(String),
}
```

Mechanism:

- Canonicalize the path, open the folder read-only and take a non-blocking
  exclusive `flock` on it through `File::try_lock`. Verified on this host: a
  second open of the same folder through another path is refused, `git status`
  stays clean and `git commit` works while the lock is held.
- The folder opens with close-on-exec, which is the standard library default.
  `keep_across_exec` clears it.
- No new dependency. The standard library call is the one both workflows use today.

Messages: `Busy` displays as "another run owns this repository", the text
systemd uses today.

## Compatibility

| Area | Change |
| --- | --- |
| Lock location | Both workflows move from their lock files to the repository folder. The old files are no longer created. |
| Mixed versions | An old binary and a new one do not exclude each other on one repository. |
| Compose across users | Two users can no longer run Compose on one repository at once. Today each user has a separate lock. |
| Compose error text | Changes to "another run owns this repository". No caller matches on the old text. |
| Repository access | A caller needs read access to the repository folder. No write access is needed. |
| Network filesystems | `flock` behavior differs on some. Local filesystems are the supported case. |
| Other state folders | Compose's `/tmp` scope folder and systemd's `/run/doramcap` folder keep their other contents and checks. |

## Milestone 1: the lock

**Work items**

- [x] Add `workload_lifecycle::admission` with `Admission` and `Refusal`.
- [x] Document the contract in doc comments: scope, refusal, release, inheritance.

**Validation**

- [x] A second `acquire` for the same folder is refused with `Busy`, including through `.` and a symlink.
- [x] Two different folders are both admitted.
- [x] Dropping the admission lets the next caller in.
- [x] A relative path, a missing path and a file each return `Invalid`.
- [x] A child process does not hold the lock by default and does after `keep_across_exec`.
- [x] `cargo test --release --locked -p workload-lifecycle admission`

## Milestone 2: Compose

**Work items**

- [x] `Controller::acquire` takes an `Admission` and calls `keep_across_exec`.
- [x] Remove `controller.lock`. Keep the private folder checks and the view workspace.
- [x] Keep reset running under the admission.

**Validation**

- [x] Existing reset tests pass with the lock-file assertions updated.
- [x] A helper command started by Compose keeps the repository busy after the launcher is gone.
- [x] `cargo test --release --locked -p workload-docker reset`

## Milestone 3: systemd

**Work items**

- [x] Replace `lock_scope` with `Admission::acquire` in runs and operator cleanup.
- [x] Keep creating the runtime folder where it is still needed.

**Validation**

- [x] The existing native test for a busy repository passes unchanged.
- [x] A systemd run is refused while a Compose-style admission is held on the same repository.
- [x] Native systemd tests, with the test executable elevated.
- [x] Library boundary check, formatting and Clippy for the three crates.
- [x] ARM64 worker run at pull request integration.

## Validation record

November 11, 2026; implementation commit `ecaeb582`.

| Check | Result |
| --- | --- |
| Shared admission / Compose reset / systemd cross-workflow refusal | 3 / 11 / 1 passed on x86_64 |
| Three crates' release unit tests | 126 passed, 4 ignored |
| Boundaries, formatting, default/all-feature Clippy | Passed |
| Native systemd, elevated executable | 26 passed, 2 baseline failures; includes the unchanged busy-repository scenario and both Dora scenarios |
| Hosted Rust x86_64 and Node x86_64/ARM64 | Passed |
| ARM64 worker | Job 661 passed: workspace default/all-feature checks, native cgroup/accounting and recording validation |
| Architecture SVG | Rendered and visually checked |

The two native failures reproduce on unchanged main `51ead064`:
`blocked_shm_survives_producer_exit_and_is_removed_after_the_run` expects one
preparation dependency but receives two;
`review_launches_frozen_units_and_preserves_nonroot_config_access` misses the ACL
input. Initial Dora prerequisite failures passed after supplying the verified
Dora 1.1.0 release. Full local workspace CI and Docker end-to-end scenarios were
not run; these focused checks do not establish a full local CI pass.

## Outside this plan

- The Holoscan runner and its Build request.
- Shared cleanup, or cleanup of one workflow's leftovers by another.
- Console's one-run rule.

Applicable rules: I-03, I-04, I-05, I-07 and I-09 in the
[implementation guide](../../../docs/impl-guide.md). Tests stay small and cover
the observable contract.
