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

## Local CI

Before declaring a PR ready, run the checks in [.github/workflows/ci.yml](.github/workflows/ci.yml) on the local host: library boundaries, formatting, default/all-feature Clippy and workspace tests, storage tests without default features, example/binary builds, and snapshot synchronization tests. Use the CI Cargo profile and locked dependencies. Build unprivileged; follow the capability-required test rules above when elevation is needed.

Run local CI and ARM64 worker validation on the final source revision unless the user explicitly waives either. Report each result separately, including skipped or blocked checks; targeted tests alone do not constitute a full CI pass.

## ARM64 CI

Submit trusted source to the Pueue worker at `doramcap-ci@100.64.0.20` (ARM64 WSL2, Linux 6.18):

```sh
python3 scripts/ci/submit.py --source /absolute/path/to/checkout
ssh doramcap-ci@100.64.0.20 pueue status
ssh doramcap-ci@100.64.0.20 pueue log --full JOB_ID
```

Use the serial default queue and verify the job result. Build unprivileged; native tests have passwordless sudo. Never copy credentials. WSL must be running; Git pushes do not trigger CI. The suite covers default/all-feature Rust tests and native cgroup accounting/allocations; run Dora end-to-end tests separately. See [worker documentation](scripts/ci/README.md).

The worker is a Qualcomm laptop without CUDA capability. Run CUDA execution and GPU/IPC qualification on a separate CUDA-capable host.

## Documentation style

- All Markdown: brief prose, short bullets, compact tables; no walls of text.
- Keep READMEs minimal: purpose, architecture SVGs, a small usage example and links to details.
- Document APIs primarily in docstrings/Rust doc comments. Aim for Linux API manpage structure and precision: synopsis, behavior, parameters, return values, errors, ownership and concurrency guarantees as applicable. Keep API details beside the code; link to them from READMEs.
- State what things do directly; avoid contrastive “A, not B” phrasing.
- Architecture: SVGs with short labels showing components and exchanged artifacts.
- Plans: use `component/docs/mmddyy-feature-name.md` with the creation date; keep updates in that file and supporting SVGs alongside it.
- Plans: lead with an embedded architecture SVG. A short high-level plan may precede the detailed milestones.
- Plans: give each milestone its own clearly labeled section containing both **Work items** and **Validation** checklists. Keep acceptance checks beside the work they verify; a separate global validation list does not replace per-milestone validation.
- Plans: record evidence in the relevant milestone before checking an item. Mark a milestone complete only when its work items and validations are complete.
