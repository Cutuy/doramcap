# Agent workflow

Every AI coding CLI must follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the returned checkout, and release after local integration or PR publication. Reuse the lease for continued work. Leases never expire. Never wipe main; obtain approval before discarding dirty worktree files.

Keep main history linear: rebase feature commits onto current main, then cherry-pick them onto main. Never use `git merge` for worktree integration.

## Capability-required tests

The user authorizes running this project's tests and test programs under sudo
when Linux capabilities are required (for example, native BPF load/attach and
collection validation). This authorization covers capability-required test
execution. System administration and persistent configuration changes require
separate authorization.

The shared local sudo password file is `.sudo-password` at the root of the
**main checkout**. Resolve the main checkout from Git's worktree list (the first
entry), so all pooled worktrees use the same file. It must remain gitignored in
the main checkout and have mode `0600`. Never print its contents, include it in
logs or command arguments, commit it, or copy it into worktrees or artifacts.
Pass it to sudo through standard input, for example:

```sh
doramcap_main_checkout="$(git worktree list --porcelain | sed -n '1s/^worktree //p')"
doramcap_sudo_password_file="$doramcap_main_checkout/.sudo-password"
test -n "$doramcap_main_checkout" && test -r "$doramcap_sudo_password_file" || exit 1
sudo -S -p '' -- ./target/debug/examples/snapshot < "$doramcap_sudo_password_file"
```

Build as the normal user; elevate only the test executable that needs it.
Ignored credential files are intentionally not copied by the worktree-pool
helper. A missing worktree-local `.sudo-password` is expected; use the shared
main-checkout path above. If the shared file is absent or unreadable, request
local provisioning. If authentication fails, stop and report the failure.

## ARM64 CI worker

Use Pueue on `doramcap-ci@100.64.0.20` (ARM64 WSL2, Linux 6.18).
Submit trusted source only: native tests have approved passwordless sudo and
can execute CI code as root. Build unprivileged; never copy sudo credentials.

```sh
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

Keep jobs in the serial default queue; Cargo caches persist. Check the job's
success/failure result. The suite covers default/all-feature Rust tests, native
cgroup accounting and allocations; Dora recording comparisons run when the pinned worker CLI is installed. WSL must
be running; Git pushes do not trigger jobs. See [worker setup and
operation](scripts/ci/README.md).

## Documentation style

Never use “this is A, not B” phrasing or equivalent contrastive framing in
documentation. State what something is or does directly.

For architecture diagrams, prefer SVG. Focus on component interactions and the artifacts passed between components. Minimize text; use short component and edge labels.

For future Markdown writing, keep prose brief and use structure that supports review. Engineering plans must lead with embedded architectural SVGs showing component interactions and the artifacts passed between them, with short labels and minimal text. Put design details in the SVGs. Write the plan primarily as milestone checklists, with explicit validation-gate checkboxes under every milestone. Keep milestones and gates unchecked until their evidence is recorded. Avoid lengthy narrative design sections.
