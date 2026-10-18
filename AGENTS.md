# Agent workflow

Every AI coding CLI must follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the returned checkout, and release after publishing the PR. Reuse the lease for continued work. Leases never expire. Never wipe main; obtain approval before discarding dirty worktree files.

Keep main history linear: rebase feature commits onto current main, then cherry-pick them onto main. Never use `git merge` for worktree integration.
