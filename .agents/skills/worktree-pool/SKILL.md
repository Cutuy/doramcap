---
name: worktree-pool
description: Use the main checkout when it already has the target branch; otherwise acquire a worktree and release after local integration or PR publication.
---

Before feature edits, check the main checkout (the first entry in `git worktree list --porcelain`). If it already has the task's target branch checked out, prioritize working there; no pool lease is needed. Preserve existing changes. Otherwise, reuse the task's existing non-expiring lease or acquire one before editing.

To acquire a lease, run from the main checkout:

```sh
python3 .agents/skills/worktree-pool/scripts/pool.py acq --owner CLI-session-task --branch feature/unique-name
```

Use the existing five slots; never create an extra slot to bypass acquisition failures. Acquisition clears known generated build outputs before selecting a slot and after copying the main snapshot. A reusable slot must then have no staged, unstaged, or non-ignored untracked source files. Ignored caches and local configuration are preserved and do not block leasing; a collision with incoming main files fails safely.

Save the token; work only in the returned path. Continue existing tasks under their existing lease. Acquisition starts at main's HEAD and copies tracked edits (unstaged) and non-ignored untracked files; ignored files and submodule edits are excluded. Keep main stable during acquisition and review the inherited diff.

When the user requests commit, cherry-pick (CP), and release:

1. Commit the intended changes in the leased worktree.
2. Rebase onto current main, then cherry-pick the feature commits in the main checkout. Preserve existing main edits and resolve conflicts.
3. Release the clean worktree after verifying integration:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --main
```

`--main` checks that every feature patch is present on main and records both commit IDs. Release preserves the worktree and branch. Release clears known generated build outputs before checking source cleanliness and again after integration or PR verification. The explicit `BUILD_OUTPUTS` paths in `scripts/pool.py` cover Cargo, frontend and Console outputs, including unignored leftovers. Build-output cleanup needs no approval. Symlinked paths and tracked files block cleanup. Cleanup failure prevents acquisition or retains the releasing lease; fix the reported problem and retry. Source changes, credentials, configuration, runtime data and dependency caches are preserved.

For a PR workflow, commit, push, and publish the PR (draft qualifies), then:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --pr PR_URL
```

PR release uses authenticated `gh` to verify the branch and HEAD. Both release modes reject staged, unstaged, and non-ignored untracked source files, and recheck cleanliness after build cleanup. Commit all work before release; local release also requires every feature patch to be cherry-picked into main. A failed check retains the lease.

- Five slots, no expiry. Use `status` to inspect; never steal leases. Failed acquisitions stay reserved for manual recovery.
- Integration: rebase the feature branch onto main’s latest HEAD, then cherry-pick (`git cherry-pick`) only its intended commits onto main. Never use `git merge` to integrate worktrees into main; keep history linear. Preserve local edits and resolve conflicts before proceeding.
- Never wipe main or delete feature branches during cleanup.
- Before `--discard-dirty`, show the exact warning/path and obtain user approval; it removes tracked, untracked, and ignored files in that slot.

Validate release behavior in disposable repositories with `python3 .agents/skills/worktree-pool/scripts/test_pool.py`.
