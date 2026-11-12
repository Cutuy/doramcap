# Agent workflow

Follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md):

```sh
node .agents/skills/worktree-pool/scripts/pool.ts acq --owner CHAT-TASK --branch codex/task
# Work in the returned path; keep its owner and token.
node .agents/skills/worktree-pool/scripts/pool.ts release --owner CHAT-TASK --token TOKEN --pr PR_URL
```

1. Acquire a lease before editing. Ask before using main when it already holds the target branch.
2. Implement, run minimal local checks, then push and create/update the PR before remaining validation.
3. Finish validation, mark the PR ready, then release. Keep the lease while local work uses the checkout.

Reuse an active task's lease; after release, acquire a fresh lease and fork to a new branch. Never wipe main; get approval before discarding source changes.

For local integration, rebase onto current main, cherry-pick, then release. Never integrate with `git merge`.

Before planning, implementing, or reviewing code changes, read both the
[implementation guide](docs/impl-guide.md) and [design guide](docs/design-guide.md).
Briefly identify applicable rule IDs and check the final changes against them.
Re-read if the guides change or leave context.

## Engineering rules

The guides hold the full rules. These are the ones most often missed:

- **Behavior first (D-01):** agree on workflow, scope, success, failure and recovery before choosing mechanisms.
- **Simplicity (D-02):** favor deletion and less code. Justify each abstraction, option, dependency and compatibility layer with a current use case.
- **One owner (D-03):** each concept, fact and policy has one owner. Avoid duplicate models, defaults, stores and forwarding wrappers.
- **Languages (D-05, I-04):** write new code in Rust or TypeScript. Use protobuf for internal models and configuration. Keep Python out of the serving stack.
- **Events, not polling (D-10, I-05):** outside tests, block on channels, `select!`, condition variables or kernel notifications. Use timers only for sampling, pacing and real deadlines.
- **Lifetimes and outcomes (D-06, D-07):** own resources from the first side effect through verified cleanup. Represent unknown explicitly and keep evidence when another step fails.
- **Inputs and limits (I-04, I-06):** validate configuration before side effects and name fixed values. Document limits, fail explicitly at exhaustion and label partial results.
- **Tests (I-07, D-13):** assert observable behavior with real inputs, synchronize on events, and cover failure, cancellation, restart and cleanup.
- **API docs (I-09):** keep contracts in doc comments beside the code, at most three lines each.

Use [Scanner](.agents/skills/scanner/SKILL.md) to review a PR or branch and reconcile
implementation/design guidance. Keep agent process instructions in this file.

## Important decisions

**Before EVERY important decision, stop, ask the user and wait for their answer.** This holds during planning, implementation and review, for design choices, scope changes, dependencies, configuration and added complexity.

Explain each decision in rendered visuals (Mermaid or SVG; never ASCII art), each with a few lines of text:

1. **Context:** the current workflow and the components involved.
2. **Problem:** what goes wrong, shown with a concrete example. Say whether example names are made up or taken from the code.
3. **Options:** one visual per possible solution, showing its behavior and tradeoffs, including success, failure and recovery where they matter. Mark your recommendation.

- Raise one decision at a time; close it before moving to the next.
- Agree on a small set of terms and use only those. Explain a term from a document in plain words before using it.
- Ground every claim in the code or docs before stating it.
- Carry the same visuals into plan documents and PR descriptions.

Added complexity is strongly discouraged; discuss it with the user and obtain agreement before introducing it.

## Configuration approval

Every new environment configuration input or extension of an existing environment contract requires the user's explicit approval.

## Third-party crates

Always present suitable, well-maintained, highly reputable third-party crates to the user for evaluation before adopting a dependency or implementing a custom solution.

## Capability-required tests

Sudo is authorized for tests requiring Linux capabilities. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.

Use `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo's stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing or unreadable; stop and report authentication failures.

## Testing and CI

- Use focused checks for incremental changes. Reserve full CI for meaningful integration points, high-risk changes, or explicit requests. Avoid repeating full CI for each interaction. For plan-driven work, update the milestone checkboxes.
- Show promising results to the user before validation finishes; clearly mark them provisional and continue testing and validation in the background.
- Profiles: local builds, runs, tests, and Clippy use `--release`; use `--profile ci` when needed. GitHub CI uses `--profile ci`. The ARM64 worker uses Cargo's default profiles for debug coverage. Use locked dependencies.
- When full local CI is warranted, run these checks on the final source revision unless waived: [CI workflow](.github/workflows/ci.yml) library boundaries, formatting, default/all-feature Clippy and workspace tests, storage tests without default features, and snapshot synchronization tests. Build unprivileged; follow the capability-required test rules above.
- Run ARM64 validation at PR integration points, when explicitly requested, or for high-risk changes inadequately covered on x86, unless waived. Report local and ARM64 results separately, including skipped/blocked checks; targeted tests alone do not establish a full CI pass.
- ARM64 worker: `doramcap-ci@100.64.0.20` over Tailscale (WSL2, Linux 6.18). Submit trusted source to Pueue's serial default queue and verify completion. WSL must be running; Git pushes do not submit jobs. Native tests use passwordless sudo; never copy credentials.
- ARM64 covers default/all-feature Rust tests and native cgroup accounting/allocations. Run Dora end-to-end tests separately and CUDA/GPU/IPC qualification on a CUDA-capable host. See [worker documentation](scripts/ci/README.md).

```sh
npm --prefix scripts/ci ci --ignore-scripts --no-audit --no-fund
node scripts/ci/wsl2/submit.mts --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

## Documentation style

### Writing

- Use plain language, brief prose, short bullets and compact tables. State what things do directly; avoid contrastive “A, not B” phrasing.
- Keep READMEs minimal: purpose, architecture SVGs, a small usage example and links to details.
- Do not add validation prose to documentation unless explicitly requested.

### Visuals

- Use architecture SVGs with short labels showing components and exchanged artifacts.
- In SVGs and web apps, never add footnotes or explanatory notes about caveats unless explicitly requested.

### Plans

- Trivial changes need no plan document. For other work, keep architecture and implementation in separate, linked documents.
- Architecture explains the high-level idea, expected behavior, major components, responsibilities and key tradeoffs. Lead with an embedded architecture SVG.
- Implementation holds the code-level details: code/API changes, chosen mechanisms, milestones and checks.
- Use `component/docs/mmddyy-feature-name.md` for architecture and `component/docs/mmddyy-feature-name-impl.md` for implementation. Include the creation date, update each document in place and keep supporting SVGs alongside it.
- Give each implementation milestone a labeled section with **Work items** and **Validation** checklists.

## Pull request descriptions

- Use the first two lines for a brief summary and a single-line validation summary. Put detailed explanations afterward.
- Favor small paragraphs and bullet points for detail. Favor architectural graphs.
- For complex PRs, explain the idea without listing too many component names.
- For targeted bug fixes, write the title and description in plain language: what went wrong, what happens now, and what stays the same. Leave out code terms. Add an SVG when the situation is hard to follow in words.
