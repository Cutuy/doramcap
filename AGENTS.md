# Agent workflow

Follow the [worktree-pool skill](.agents/skills/worktree-pool/SKILL.md): acquire before feature edits, work in the leased checkout, reuse its non-expiring lease, and release after local integration or PR publication. Never wipe main; get approval before discarding dirty worktree files.

Keep main linear: rebase onto current main, then cherry-pick. Never integrate with `git merge`.

## Simplicity

Strongly favor deletion, less code, and explicit behavior. Keep changes as simple as practical. Added complexity is strongly discouraged; discuss it with the user and obtain agreement before introducing it.

## Implementation languages

Strongly prefer Rust and TypeScript for new code. Python is allowed for tests, benchmarks, and development tooling. Do not use Python in the application serving stack. Avoid Python, Bash, and other scripting languages in production environments and workflows unless absolutely necessary.

## Configuration

Favor strongly typed protobuf messages over untyped JSON for internal models and application configuration; use protobuf text format (pbtxt) for configuration files. Use generated protobuf types directly where practical; avoid parallel handwritten Rust configuration structs for fields already defined in a proto.

Favor TypeScript for dynamic configuration processing. Use protobuf contracts to decouple TypeScript configuration processing from Rust execution. Preserve arbitrary third-party configuration in its native format at integration boundaries.

Environment-based application configuration is highly discouraged. Every new environment configuration input or extension of an existing environment contract requires the user's explicit approval. Encapsulate approved exceptions behind a named interface with an explicit list of supported variables; the Dora bridge is `dora_app::environment`.

## Third-party crates

Always present suitable, well-maintained, highly reputable third-party crates to the user for evaluation before adopting a dependency or implementing a custom solution.

## Test structure

Test cases should generally use explicit `// Arrange`, `// Act`, and `// Assert`
sections to separate setup, the operation under test, and verification. For
stateful scenarios, repeat Act/Assert sections for distinct transitions.

Unit tests must assert documented behavior and observable outcomes. Do not hardcode
assumptions about implementation details or reproduce the logic being tested; use
real integration inputs where those contracts already exist.

## Capability-required tests

Sudo is authorized for tests requiring Linux capabilities. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.

Use `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo's stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing or unreadable; stop and report authentication failures.

## Testing and CI

- Use focused checks for incremental changes. Reserve full CI for meaningful integration points, high-risk changes, or explicit requests. Avoid repeating full CI for each interaction. For plan-driven work, record completed and deferred checks in the relevant milestone.
- When full local CI is needed, prefer pushing the current PR branch after focused checks to trigger cloud CI before starting full local CI.
- Show promising results to the user before validation finishes; clearly mark them provisional and continue testing and validation in the background.
- Profiles: local builds, runs, tests, and Clippy use `--release`; use `--profile ci` when needed. GitHub CI uses `--profile ci`. The ARM64 worker uses Cargo's default profiles for debug coverage. Use locked dependencies.
- When full local CI is warranted, run these checks on the final source revision unless waived: [CI workflow](.github/workflows/ci.yml) library boundaries, formatting, default/all-feature Clippy and workspace tests, storage tests without default features, example/binary builds, and snapshot synchronization tests. Build unprivileged; follow the capability-required test rules above.
- Run ARM64 validation at PR integration points, when explicitly requested, or for high-risk changes inadequately covered on x86, unless waived. Report local and ARM64 results separately, including skipped/blocked checks; targeted tests alone do not establish a full CI pass.
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
- In functions with lengthy business logic, prefer concise, one-line inline comments explaining the key steps.
- Document APIs primarily in docstrings/Rust doc comments. Keep docstrings to at most three lines; use named sections or bullet points when more detail is necessary. Follow Linux API manpage precision for behavior, parameters, return values, errors, ownership and concurrency as applicable. Keep API details beside the code; link to them from READMEs.
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
