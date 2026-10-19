---
name: worktree-pool
description: Acquire a worktree before coding; release after local integration or PR publication.
---

Run from the main checkout:

```sh
python3 .agents/skills/worktree-pool/scripts/pool.py acq --owner CLI-session-task --branch feature/unique-name
```

Use the existing five slots; never create an extra slot to bypass acquisition failures. A reusable slot must have no staged, unstaged, or non-ignored untracked files. Ignored caches and local configuration are preserved and do not block leasing; a collision with incoming main files fails safely.

Save the token; work only in the returned path. Continue existing tasks under their existing lease. Acquisition starts at main's HEAD and copies tracked edits (unstaged) and non-ignored untracked files; ignored files and submodule edits are excluded. Keep main stable during acquisition and review the inherited diff.

When the user requests commit, cherry-pick (CP), and release:

1. Commit the intended changes in the leased worktree.
2. Rebase onto current main, then cherry-pick the feature commits in the main checkout. Preserve existing main edits and resolve conflicts.
3. Release the clean worktree after verifying integration:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --main
```

`--main` checks that every feature patch is present on main and records both commit IDs. Release preserves the worktree and branch. After integration or PR verification, the script removes the ignored build directories `target/`, `apps/cpu-viewer/frontend/dist/`, and `apps/activity-demo/frontend/dist/` before freeing the lease. Cleanup failure keeps the lease active; fix the reported problem and retry with the same token. Symlinked paths, tracked files, and directories that are no longer ignored block cleanup. Other ignored files, dependency caches, and external build caches are preserved.

For a PR workflow, commit, push, and publish the PR (draft qualifies), then:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --pr PR_URL
```

PR release uses authenticated `gh` to verify the branch and HEAD. Both release modes reject staged, unstaged, and non-ignored untracked files, and recheck cleanliness after build cleanup. Commit all work before release; local release also requires every feature patch to be cherry-picked into main. A failed check retains the lease.

- Five slots, no expiry. Use `status` to inspect; never steal leases. Failed acquisitions stay reserved for manual recovery.
- Integration: rebase the feature branch onto main’s latest HEAD, then cherry-pick (`git cherry-pick`) only its intended commits onto main. Never use `git merge` to integrate worktrees into main; keep history linear. Preserve local edits and resolve conflicts before proceeding.
- Never wipe main or delete feature branches during cleanup.
- Before `--discard-dirty`, show the exact warning/path and obtain user approval; it removes tracked, untracked, and ignored files in that slot.

Validate release behavior in disposable repositories with `python3 .agents/skills/worktree-pool/scripts/test_pool.py`.
