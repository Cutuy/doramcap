# Shared admission lock: implementation plan

Created November 10, 2026. Status: draft; not started.
[Architecture](111026-admission-lock.md).

Move repository admission into `workload-lifecycle` and make Compose and systemd
use it. One pull request, three milestones. The Holoscan runner takes the same
lock when it is built.

## Current code

| Workflow | Code | Behavior |
| --- | --- | --- |
| Compose | `Controller::acquire` in [`reset.rs`](../../workload-docker/src/workflows/recorded_compose/reset.rs) | Locks `controller.lock` in a private per-user folder under `/tmp`. Clears close-on-exec so helper commands keep the lock. Error: "another experiment controller is active". |
| systemd | `lock_scope` in [`runner.rs`](../../workload-systemd/src/runner.rs) | Locks `lock` under `/run/doramcap/<repository hash>`. Taken by runs and operator cleanup. Error: "another run owns this repository". |

Both use `std::fs::File::try_lock`. Both folders also hold other run state, which
stays where it is.

## API

A new module, `workload_lifecycle::admission`.

```rust
/// Exclusive ownership of one repository for a run, build or cleanup.
/// Released on drop or when the holding process exits.
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
    /// The path is relative, missing, or names something other than a folder.
    Invalid(String),
}
```

Mechanism:

- Canonicalize the path, open the folder read-only and take a non-blocking
  exclusive `flock` on it through `File::try_lock`. Verified on this host: a
  second open of the same folder through another path is refused.
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

- [ ] Add `workload_lifecycle::admission` with `Admission` and `Refusal`.
- [ ] Document the contract in doc comments: scope, refusal, release, inheritance.

**Validation**

- [ ] A second `acquire` for the same folder is refused with `Busy`, including through `.` and a symlink.
- [ ] Two different folders are both admitted.
- [ ] Dropping the admission lets the next caller in.
- [ ] A relative path, a missing path and a file each return `Invalid`.
- [ ] A child process does not hold the lock by default and does after `keep_across_exec`.
- [ ] `cargo test --release --locked -p workload-lifecycle admission`

## Milestone 2: Compose

**Work items**

- [ ] `Controller::acquire` takes an `Admission` and calls `keep_across_exec`.
- [ ] Remove `controller.lock`. Keep the private folder checks and the view workspace.
- [ ] Keep reset running under the admission.

**Validation**

- [ ] Existing reset tests pass with the lock-file assertions updated.
- [ ] A helper command started by Compose keeps the repository busy after the launcher is gone.
- [ ] `cargo test --release --locked -p workload-docker reset`

## Milestone 3: systemd

**Work items**

- [ ] Replace `lock_scope` with `Admission::acquire` in runs and operator cleanup.
- [ ] Keep creating the runtime folder where it is still needed.

**Validation**

- [ ] The existing native test for a busy repository passes unchanged.
- [ ] A systemd run is refused while a Compose-style admission is held on the same repository.
- [ ] Native systemd tests, with the test executable elevated.
- [ ] Library boundary check, formatting and Clippy for the three crates.
- [ ] ARM64 worker run at pull request integration.

## Outside this plan

- The Holoscan runner and its Build request.
- Shared cleanup, or cleanup of one workflow's leftovers by another.
- Console's one-run rule.

Applicable rules: I-03, I-04, I-05, I-07 and I-09 in the
[implementation guide](../../../docs/impl-guide.md). Tests stay small and cover
the observable contract.
