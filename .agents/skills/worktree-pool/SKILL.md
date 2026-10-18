---
name: worktree-pool
description: Acquire and release one of this repository's five persistent Git worktrees when an AI coding CLI starts a feature or publishes its PR.
---

Use this protocol for every coding CLI working in this repository. Run the Python helper from the main checkout (or give `--repo /absolute/checkout`). It requires Python 3.9+, Git, and `gh` authenticated for release.

1. Before editing for a new feature, run `python3 .agents/skills/worktree-pool/scripts/pool.py acq --owner CLI-session-task --branch feature/unique-name`. Save the returned token and absolute path. Perform all feature edits, tests, commits, and PR operations inside that path. A continued task reuses its existing lease.
2. Acquisition creates a new branch from the main checkout's current HEAD, overlays tracked staged/unstaged changes as unstaged changes, and copies non-ignored untracked files. It does not fetch or rebase old feature commits. Ignored files and submodule working-tree contents are not copied. Avoid changing the main checkout while acquiring; inspect the inherited diff before coding. Inherited edits are part of the feature unless explicitly separated.
3. Commit, push, and create the PR using the task's authorized workflow. Verify it is published, then run `python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --pr https://github.com/OWNER/REPO/pull/NUMBER`. The helper verifies the PR's branch and HEAD with `gh`. PR creation failure means retain the lease. A published draft PR qualifies. Release preserves the branch and files; subsequent work needs a new acquisition.

Leases never expire. `status` lists owners and tokens; `init` idempotently creates exactly five slots. A full pool means wait or ask the owner to finish, never steal a slot. Interrupted acquisitions remain reserved for explicit manual recovery.

Never reset, clean, remove, or otherwise wipe the main checkout. Never bypass the helper to reuse a slot. If acquisition reports dirty files, show the user the exact path and warning, and obtain approval before retrying with `--discard-dirty`. This flag permits removing tracked edits, untracked files, and ignored build artifacts in the selected free slot only. Never delete feature branches as part of pool cleanup. Release requires no uncommitted non-ignored changes, so preserve or publish remaining work first. All pool operations serialize via a shared filesystem lock; state lives under the common Git directory's `agent-pool/` and is never committed.
