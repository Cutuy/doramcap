---
name: worktree-pool
description: Acquire an exclusive worktree lease, resume it, and release after publication or integration.
---

Follow the development workflow in [AGENTS.md](../../../AGENTS.md). Requires Linux, Node 22.18+, Git, `flock`, and `gh` for PR release. Run from a checkout; use `--repo CHECKOUT` from elsewhere.

```sh
node .agents/skills/worktree-pool/scripts/pool.ts acq --owner CHAT-TASK --branch codex/task
# Save the returned path, owner and token; work in that path.
node .agents/skills/worktree-pool/scripts/pool.ts acq --owner CHAT-TASK --branch codex/task --token TOKEN
node .agents/skills/worktree-pool/scripts/pool.ts release --owner CHAT-TASK --token TOKEN --pr PR_URL
```

- **Ownership:** reuse an active lease with its owner and token. After release, acquire a fresh lease/new branch and carry the published commits there. Capacity is shared in the registered slots, with no expiry; commands serialize, and exhaustion fails explicitly. Never steal leases. Initialize or grow the pool only with explicit user approval: `init --slots COUNT`. COUNT must be a positive safe integer; shrinking is refused. Plain `init` preserves an existing pool. `status` hides tokens.
- **Starting point:** acquisition copies the main checkout's HEAD, tracked edits and non-ignored untracked files. Keep main stable, inspect the inherited diff, and preserve other work. Ignored files and submodule edits stay in main.
- **Release:** finish validation, record results and mark the PR ready first. Release checks owner, token, branch, source cleanliness and that the PR's head commit equals the checkout's HEAD; the PR branch name may differ from the leased branch. For user-requested local cherry-pick integration, use `--main` instead of `--pr PR_URL`. After verification, release detaches the checkout at its current commit and deletes the leased local feature branch. `main` cannot be leased or deleted; remote branches remain. The checkout stays available for reuse.
- **Interrupted release:** if the checkout is detached, show the warning and obtain user confirmation before restoring the leased branch and retrying. Reattach the existing branch, or recreate a deleted branch at the displayed commit. Preserve existing branch commits and never edit lease records.
- **Cleanup:** known build outputs are removed on acquire/release; source, caches and local data are preserved. Show the reported path and files and obtain user approval before `--discard-dirty`. Failed preparation stays reserved; failed release keeps ownership. Inspect and repair; never edit lease records by hand.

See `node .agents/skills/worktree-pool/scripts/pool.ts --help` for commands. Validate in disposable repositories:

```sh
node --test .agents/skills/worktree-pool/scripts/pool.test.ts
```
