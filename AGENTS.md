# Agent workflow

Every AI coding CLI must follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the returned checkout, and release after publishing the PR. Reuse the lease for continued work. Leases never expire. Never wipe main; obtain approval before discarding dirty worktree files.

Keep main history linear: rebase feature commits onto current main, then cherry-pick them onto main. Never use `git merge` for worktree integration.

## Capability-required tests

The user authorizes running this project's tests and test programs under sudo
when Linux capabilities are required (for example, native BPF load/attach and
collection validation). This authorization covers test execution, not unrelated
system administration or persistent system configuration changes.

The local sudo password file is `.sudo-password` at the root of the active
worktree. It must remain gitignored and have mode `0600`. Never print its
contents, include it in logs or command arguments, commit it, or copy it into
artifacts. Pass it to sudo through standard input, for example:

```sh
sudo -S -p '' -- ./target/debug/examples/snapshot < .sudo-password
```

Build as the normal user; elevate only the test executable that needs it.
Ignored credential files are not copied by the worktree-pool helper. If the file
is absent in another worktree, request local provisioning rather than adding a
credential to tracked files. If authentication fails, stop and report the failure.
