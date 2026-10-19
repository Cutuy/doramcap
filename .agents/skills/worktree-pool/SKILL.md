---
name: worktree-pool
description: Acquire a worktree before coding; release after local integration or PR publication.
---

Run from the main checkout:

```sh
python3 .agents/skills/worktree-pool/scripts/pool.py acq --owner CLI-session-task --branch feature/unique-name
```

Save the token; work only in the returned path. Continue existing tasks under their existing lease. Acquisition starts at main's HEAD and copies tracked edits (unstaged) and non-ignored untracked files; ignored files and submodule edits are excluded. Keep main stable during acquisition and review the inherited diff.

When the user requests commit, cherry-pick (CP), and release:

1. Commit the intended changes in the leased worktree.
2. Rebase onto current main, then cherry-pick the feature commits in the main checkout. Preserve existing main edits and resolve conflicts.
3. Release the clean worktree after verifying integration:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --main
```

`--main` checks that every feature patch is present on main and records both commit IDs. Release preserves the worktree files and branch.

For a PR workflow, commit, push, and publish the PR (draft qualifies), then:

```sh
python3 /absolute/main/.agents/skills/worktree-pool/scripts/pool.py --repo /absolute/main release --token TOKEN --pr PR_URL
```

PR release uses authenticated `gh` to verify the branch and HEAD. Uncommitted changes block both release modes.

- Five slots, no expiry. Use `status` to inspect; never steal leases. Failed acquisitions stay reserved for manual recovery.
- Integration: rebase the feature branch onto main’s latest HEAD, then cherry-pick (`git cherry-pick`) only its intended commits onto main. Never use `git merge` to integrate worktrees into main; keep history linear. Preserve local edits and resolve conflicts before proceeding.
- Never wipe main or delete feature branches during cleanup.
- Before `--discard-dirty`, show the exact warning/path and obtain user approval; it removes tracked, untracked, and ignored files in that slot.
