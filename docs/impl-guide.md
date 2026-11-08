# Implementation guide

Repository rules, with their stated conditions and exceptions. Use alongside the
[design guide](design-guide.md) and applicable `AGENTS.md` instructions.

## I-01 — Decisions

- Ask the user and wait before every important decision; present a recommendation and alternatives.
- Obtain agreement before adding complexity. Present suitable, maintained, reputable crates for evaluation before adopting a dependency or implementing a custom solution.
- Every new environment configuration input or extension requires explicit approval. Encapsulate approved variables in a named interface listing supported variables; the Dora bridge uses `dora_app::environment`.

## I-02 — Checkout and integration

- Follow [worktree-pool](../.agents/skills/worktree-pool/SKILL.md). Prefer the main checkout when it has the target branch; otherwise acquire before feature edits, reuse the task's non-expiring lease, and work in that checkout.
- Rebase onto current main, then cherry-pick intended commits. Never integrate with `git merge`. Release after verified local integration or PR publication.
- Preserve existing source changes; get approval before discarding them. Never wipe main or steal a lease.

## I-03 — Contracts and changes

- Dependencies follow established ownership decisions; use [repository boundary checks](../scripts/ci/boundaries.json). Inspect semantic ownership separately.
- Ownership changes update placement, public names, dependencies, callers, documentation, and relevant tests together.
- Refactors preserve runtime behavior and persisted/wire compatibility unless a change is requested. Assess actual consumers before removing public APIs; aliases must not preserve forbidden boundaries.

## I-04 — Inputs and implementation

- Validate configuration constraints during loading or preparation when possible, before dependent side effects. Check rendered-input constraints once rendering is available; errors identify the constraint and relevant input.
- Avoid hardcoded constants. Give required fixed values descriptive names and reuse them.
- Keep Python out of the application serving stack. It is allowed for tests, benchmarks, and development tooling.

## I-05 — Concurrency

- Outside tests, signal work arrival, capacity, state changes, completion, failure, and shutdown explicitly. Block on channels, `select!`, condition variables, or kernel notifications.
- Avoid sleep timeouts and repeated timed waits, parks, or yields used to poll state. Subscribe before checking, recheck after notifications, and wake blocked workers on shutdown or disconnect.
- Use timers for sampling schedules, pacing, simulated work, and real deadlines. Name fixed durations; derive deadlines from the operation they bound.

## I-06 — Limits and observations

- Document supported limits, scope, and exhaustion behavior. Limits, platform constraints, and caller budgets fail explicitly or expose documented partial results.
- Previews and pagination preserve access to the supported underlying result. Label partial observations; failed or truncated traversal cannot establish completeness, empty success, or deletion.
- Test relevant limits at acceptance and exhaustion. Support completeness claims with evidence that all supported pages or records are processed.

## I-07 — Tests and CI

- Assert documented behavior and observable outcomes, using real integration inputs where contracts exist. Avoid implementation assumptions and reproducing the logic under test.
- Generally use explicit `// Arrange`, `// Act`, and `// Assert` sections; repeat Act/Assert for state transitions.
- Use focused checks incrementally. Reserve full CI for meaningful integration points, high-risk changes, or explicit requests. Record completed and deferred checks in the relevant plan milestone.
- Local builds, runs, tests, and Clippy use `--release`, or `--profile ci` when needed; GitHub CI uses `--profile ci`. Use locked dependencies. ARM64 uses Cargo's default profiles for debug coverage.
- When full local CI is warranted, prefer triggering cloud CI through the current PR branch after focused checks, within publication authorization. Unless waived, check the final revision against the [CI workflow](../.github/workflows/ci.yml): boundaries, formatting, default/all-feature Clippy and workspace tests, storage without default features, and snapshot synchronization.
- Run ARM64 at PR integration points, when requested, or for high-risk changes inadequately covered on x86, unless waived. Report local and ARM64 results separately, including skipped/blocked checks. Focused checks alone do not establish a full CI pass.
- Share promising results as provisional while validation continues.

## I-08 — Capability and hardware validation

- Sudo is authorized for capability-required tests. Build unprivileged; elevate only the test executable. System administration and persistent configuration changes require separate approval.
- Read `.sudo-password` from the main checkout (first entry in `git worktree list --porcelain`) through sudo stdin. Keep it gitignored and mode `0600`; never print, log, commit, copy, or pass its contents as arguments. Request local provisioning if missing/unreadable; stop and report authentication failures.
- Submit trusted source to `doramcap-ci@100.64.0.20` over Tailscale using `python3 scripts/ci/submit.py --source /absolute/path/to/checkout`. Use Pueue's serial default queue; verify completion with `pueue status` and `pueue log --full JOB_ID` over SSH. WSL must be running; Git pushes do not submit jobs. Native tests use passwordless sudo; never copy credentials.
- ARM64 covers default/all-feature Rust tests and native cgroup accounting/allocations. Run Dora end-to-end separately and CUDA/GPU/IPC qualification on a CUDA-capable host. See [worker documentation](../scripts/ci/README.md).

## I-09 — Documentation and plans

- Keep API contracts in docstrings/Rust doc comments beside code: meaning, behavior, constraints, parameters/results, errors, ownership, concurrency, and cancellation as applicable, with manpage precision. Keep docstrings at most three lines; use named sections or bullets when more detail is needed. Link READMEs to API docs.
- SVGs and web apps must omit caveat footnotes unless requested. Architecture SVGs use short component labels and exchanged artifacts.
- Put plans at `component/docs/mmddyy-feature-name.md`, using the creation date; keep updates there and supporting SVGs alongside. Lead with an embedded architecture SVG; a brief overview may precede milestones.
- Each milestone has **Work items** and **Validation** checklists with acceptance checks beside its work. Record evidence before checking items; complete a milestone only when both lists are complete.
- Validation records retain only the latest results and unresolved blockers in short bullets, linking CI/logs. Replace stale results instead of appending histories or transcripts.
- PR descriptions start with a brief summary and a single-line validation summary on the first two lines; put details afterward.
