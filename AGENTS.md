# Agent workflow

Follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the leased checkout, reuse its non-expiring lease, and release after local integration or PR publication. Never wipe main; get approval before discarding dirty worktree files.

Keep main linear: rebase onto current main, then cherry-pick. Never integrate with `git merge`.

## Implementation languages

Strongly prefer Rust and TypeScript for new code. Python is allowed for tests, benchmarks, and development tooling. Do not use Python in the application serving stack. Avoid Python, Bash, and other scripting languages in production environments and workflows unless absolutely necessary.

## Configuration

Prefer protobuf schemas and protobuf text format (pbtxt) for application configuration. Avoid JSON configuration when a protobuf schema can express it. Use generated protobuf types directly where practical; avoid parallel handwritten Rust configuration structs for fields already defined in a proto.

## Third-party crates

Always present suitable, well-maintained, highly reputable third-party crates to the user for evaluation before adopting a dependency or implementing a custom solution.

## Test structure

Test cases should generally use explicit `// Arrange`, `// Act`, and `// Assert`
sections to separate setup, the operation under test, and verification. For
stateful scenarios, repeat Act/Assert sections for distinct transitions.

## Capability-required tests

Sudo is authorized for tests requiring Linux capabilities. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.

Use `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo's stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing or unreadable; stop and report authentication failures.

## Testing and CI

- When implementing a plan document, prioritize fast feedback and early validation of the direction. Use focused checks for each milestone and strategically defer or skip redundant full CI runs; batch full CI at meaningful integration points and on the final source revision unless waived. Record deferred checks in the plan so milestone validation stays explicit.
- Profiles: local builds, runs, tests, and Clippy use `--release`; use `--profile ci` when needed. GitHub CI uses `--profile ci`. The ARM64 worker uses Cargo's default profiles for debug coverage. Use locked dependencies.
- Run local checks on the final source revision unless waived: [CI workflow](.github/workflows/ci.yml) library boundaries, formatting, default/all-feature Clippy and workspace tests, storage tests without default features, example/binary builds, and snapshot synchronization tests. Build unprivileged; follow the capability-required test rules above.
- Run ARM64 validation on the final revision for PRs, explicit requests, or high-risk changes inadequately covered on x86, unless waived. Report local and ARM64 results separately, including skipped/blocked checks; targeted tests alone do not establish a full CI pass.
- ARM64 worker: `doramcap-ci@100.64.0.20` over Tailscale (WSL2, Linux 6.18). Submit trusted source to Pueue's serial default queue and verify completion. WSL must be running; Git pushes do not submit jobs. Native tests use passwordless sudo; never copy credentials.
- ARM64 covers default/all-feature Rust tests and native cgroup accounting/allocations. Run Dora end-to-end tests separately and CUDA/GPU/IPC qualification on a CUDA-capable host. See [worker documentation](scripts/ci/README.md).

```sh
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

## Documentation style

- All Markdown: brief prose, short bullets, compact tables; no walls of text.
- Keep READMEs minimal: purpose, architecture SVGs, a small usage example and links to details.
- Document APIs primarily in docstrings/Rust doc comments. Keep docstrings to at most three lines; use named sections or bullet points when more detail is necessary. Follow Linux API manpage precision for behavior, parameters, return values, errors, ownership and concurrency as applicable. Keep API details beside the code; link to them from READMEs.
- State what things do directly; avoid contrastive “A, not B” phrasing.
- In SVGs and web apps, NEVER add footnotes such as “this is A, not B” or explanatory notes about caveats unless explicitly requested.
- Architecture: SVGs with short labels showing components and exchanged artifacts.
- Plans: use `component/docs/mmddyy-feature-name.md` with the creation date; keep updates in that file and supporting SVGs alongside it.
- Plans: lead with an embedded architecture SVG. A short high-level plan may precede the detailed milestones.
- Plans: give each milestone its own clearly labeled section containing both **Work items** and **Validation** checklists. Keep acceptance checks beside the work they verify; a separate global validation list does not replace per-milestone validation.
- Plans: record evidence in the relevant milestone before checking an item. Mark a milestone complete only when its work items and validations are complete.
