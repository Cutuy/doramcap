# Agent workflow

Every AI coding CLI must use the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md) before touching code for a new feature. Acquire one of the five slots, keep its token, and work only in the returned worktree. Reuse the lease when continuing the same task. Release it after the PR is created and published.

Leases never expire. Never steal a lease or wipe the main checkout. Show the user the warning and obtain approval before discarding dirty pool-worktree files. See the skill for commands and refresh semantics.
