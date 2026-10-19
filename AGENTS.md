# Agent workflow

Follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the leased checkout, reuse its non-expiring lease, and release after local integration or PR publication. Never wipe main; get approval before discarding dirty worktree files.

Keep main linear: rebase onto current main, then cherry-pick. Never integrate with `git merge`.

## Third-party crates

Always present suitable, well-maintained, highly reputable third-party crates to the user for evaluation before adopting a dependency or implementing a custom solution.

## Capability-required tests

Sudo is authorized for tests requiring Linux capabilities. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.

Use `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo's stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing or unreadable; stop and report authentication failures.

## ARM64 CI

Submit trusted source to the Pueue worker at `doramcap-ci@100.64.0.20` (ARM64 WSL2, Linux 6.18):

```sh
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

Use the serial default queue and verify the job result. Build unprivileged; native tests have passwordless sudo. Never copy credentials. WSL must be running; Git pushes do not trigger CI. The suite covers default/all-feature Rust tests and native cgroup accounting/allocations; run Dora end-to-end tests separately. See [worker documentation](scripts/ci/README.md).

## Documentation style

Milestone and planning documents must live under the owning component at `xxx/docs/mmddyy-some-feature.md`, using the creation date in `mmddyy` format and a descriptive kebab-case feature name. For example: `libs/resource-policy/docs/101926-resource-policy.md`. Keep updates in the same dated file and supporting SVGs alongside it.

State what things are or do directly. Avoid “this is A, not B” and equivalent contrastive phrasing.

For architecture diagrams, prefer SVG. Focus on component interactions and the artifacts passed between components. Minimize text; use short component and edge labels.

For future Markdown writing, keep prose brief and use structure that supports review. Engineering plans must lead with embedded architectural SVGs showing component interactions and the artifacts passed between them, with short labels and minimal text. Put design details in the SVGs. Write the plan primarily as milestone checklists, with explicit validation-gate checkboxes under every milestone. Keep milestones and gates unchecked until their evidence is recorded. Avoid lengthy narrative design sections.
