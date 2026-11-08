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

- **Ownership:** reuse the same owner and token for the task. Five slots, no expiry. Commands serialize on the shared lock; a full pool fails explicitly. Wait for a release; never steal a lease or add slots. `status` lists owners without tokens; `init` creates the pool once.
- **Starting point:** acquisition copies the main checkout's HEAD, tracked edits and non-ignored untracked files. Keep main stable, inspect the inherited diff, and preserve other work. Ignored files and submodule edits stay in main.
- **Release:** finish local work and validation first. Release checks owner, token, branch, source cleanliness and the PR's exact HEAD. For user-requested local cherry-pick integration, use `--main` instead of `--pr PR_URL`. Branches and checkouts remain.
- **Cleanup:** known build outputs are removed on acquire/release; source, caches and local data are preserved. Show the reported path and files and obtain user approval before `--discard-dirty`. Failed preparation stays reserved; failed release keeps ownership. Inspect and repair; never edit lease records by hand.

See `node .agents/skills/worktree-pool/scripts/pool.ts --help` for commands. Validate in disposable repositories:

```sh
node --test .agents/skills/worktree-pool/scripts/pool.test.ts
```
