---
name: worktree-pool
description: Acquire an exclusive worktree lease, resume it, and release after publication or integration.
---

Follow [AGENTS.md](../../../AGENTS.md). Requires Linux, Node 22.18+, Git, `flock` and `gh`.

```sh
node .agents/skills/worktree-pool/scripts/pool.ts acq --owner TASK --branch codex/NAME               # work in the returned path
node .agents/skills/worktree-pool/scripts/pool.ts acq --owner TASK --branch codex/NAME --token TOKEN # resume
node .agents/skills/worktree-pool/scripts/pool.ts release --owner TASK --token TOKEN --pr PR_URL     # or --main
```

- Save the owner and token. Never use another task's lease. After release, start a new lease and branch.
- A new lease copies main's HEAD and uncommitted work; check the inherited diff.
- Release after validation, with the PR ready. The checkout must be clean and its HEAD must be the PR head, or already in main with `--main`.
- Ask the user before `init`, `--discard-dirty` or retrying an interrupted release. Never edit lease records.

`pool.ts --help` lists all commands. Test with `node --test .agents/skills/worktree-pool/scripts/pool.test.ts`.
