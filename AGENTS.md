# Agent workflow

Follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md):

1. Acquire a lease before editing. Ask before using main when it already holds the target branch.
2. Implement, run minimal local checks, then push and create/update the PR before remaining validation.
3. Finish validation, record results, mark the PR ready, then release. Keep the lease while local work uses the checkout.

Reuse an active task's lease; after release, acquire a fresh lease and fork to a new branch. Never wipe main; get approval before discarding source changes.

For local integration, rebase onto current main, cherry-pick, then release. Never integrate with `git merge`.

Read the [implementation guide](docs/impl-guide.md) for code rules and the
[design guide](docs/design-guide.md) for architectural guidance. Use
[Scanner](.agents/skills/scanner/SKILL.md) to review a PR or branch and reconcile
implementation/design guidance. Keep agent process instructions in this file.

## Important decisions

For every important decision, ask the user and wait for their answer before proceeding. Present your recommended option and alternatives.

Check large feature work against the [design guide](docs/design-guide.md) before implementation and during design review.

Added complexity is strongly discouraged; discuss it with the user and obtain agreement before introducing it.

## Configuration approval

Every new environment configuration input or extension of an existing environment contract requires the user's explicit approval.

## Third-party crates

Always present suitable, well-maintained, highly reputable third-party crates to the user for evaluation before adopting a dependency or implementing a custom solution.

## Capability-required tests

Sudo is authorized for tests requiring Linux capabilities. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.

Use `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo's stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing or unreadable; stop and report authentication failures.

## Testing and CI

- Use focused checks for incremental changes. Reserve full CI for meaningful integration points, high-risk changes, or explicit requests. Avoid repeating full CI for each interaction. For plan-driven work, record completed and deferred checks in the relevant milestone.
- Show promising results to the user before validation finishes; clearly mark them provisional and continue testing and validation in the background.
- Profiles: local builds, runs, tests, and Clippy use `--release`; use `--profile ci` when needed. GitHub CI uses `--profile ci`. The ARM64 worker uses Cargo's default profiles for debug coverage. Use locked dependencies.
- When full local CI is warranted, run these checks on the final source revision unless waived: [CI workflow](.github/workflows/ci.yml) library boundaries, formatting, default/all-feature Clippy and workspace tests, storage tests without default features, and snapshot synchronization tests. Build unprivileged; follow the capability-required test rules above.
- Run ARM64 validation at PR integration points, when explicitly requested, or for high-risk changes inadequately covered on x86, unless waived. Report local and ARM64 results separately, including skipped/blocked checks; targeted tests alone do not establish a full CI pass.
- ARM64 worker: `doramcap-ci@100.64.0.20` over Tailscale (WSL2, Linux 6.18). Submit trusted source to Pueue's serial default queue and verify completion. WSL must be running; Git pushes do not submit jobs. Native tests use passwordless sudo; never copy credentials.
- ARM64 covers default/all-feature Rust tests and native cgroup accounting/allocations. Run Dora end-to-end tests separately and CUDA/GPU/IPC qualification on a CUDA-capable host. See [worker documentation](scripts/ci/README.md).

```sh
npm --prefix scripts/ci ci --ignore-scripts --no-audit --no-fund
node scripts/ci/submit.mts --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

## Documentation style

- All Markdown: brief prose, short bullets, compact tables; no walls of text.
- Validation records: keep only the latest result and unresolved blockers in a few short bullets. Link to CI or logs for details; replace stale results instead of appending test histories, counts or command transcripts.
- Keep READMEs minimal: purpose, architecture SVGs, a small usage example and links to details.
- State what things do directly; avoid contrastive “A, not B” phrasing.
- In SVGs and web apps, NEVER add footnotes such as “this is A, not B” or explanatory notes about caveats unless explicitly requested.
- Architecture: SVGs with short labels showing components and exchanged artifacts.
- Plans: use `component/docs/mmddyy-feature-name.md` with the creation date; keep updates in that file and supporting SVGs alongside it.
- Plans: lead with an embedded architecture SVG. A short high-level plan may precede the detailed milestones.
- Plans: give each milestone its own clearly labeled section containing both **Work items** and **Validation** checklists. Keep acceptance checks beside the work they verify; a separate global validation list does not replace per-milestone validation.
- Plans: record evidence in the relevant milestone before checking an item. Mark a milestone complete only when its work items and validations are complete.

## Pull request descriptions

- Use the first two lines for a brief summary and a single-line validation summary. Put detailed explanations afterward.
- Favor small paragraphs and bullet points for detail. Favor architectural graphs.
- For complex PRs, explain the idea without listing too many component names.
- For targeted bug fixes, write the title and description in plain language: what went wrong, what happens now, and what stays the same. Leave out code terms. Add an SVG when the situation is hard to follow in words.
