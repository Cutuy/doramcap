# Engineering requirements

Baseline extracted from the former architecture-review skill, incorporating the
load-time validation discussion. Apply each item within its relevant scope;
repository-specific instructions and explicit user requirements govern conflicts.

- **SE-001 — Concept ownership.** Public types and operations belong to the component
  owning their semantics and required assumptions. Assess fields and actual callers.
- **SE-002 — Execution boundaries.** Generic execution owns invocation, serialization
  and observation. Lifecycle policy, backend mechanics and concrete workflows have
  their own owners.
- **SE-003 — Backend contracts.** Backend-specific fields and operations live with
  that backend and use names reflecting their scope. Fix misplaced APIs through
  placement and dependencies as well as naming and documentation.
- **SE-004 — Input independence.** Core contracts describe meaning, constraints and
  effects. UI, CLI and file-input choices enter those contracts only when required
  by their behavior.
- **SE-005 — Shared abstractions.** Shared contracts reflect demonstrated common
  semantics. Additional backends alone do not justify generic bags, empty interfaces
  or a plugin framework.
- **SE-006 — Dependency direction.** Dependencies follow established ownership
  decisions. Enforce agreed crate boundaries with repository-owned checks where
  appropriate; inspect semantic ownership independently of dependency checks.
- **SE-007 — Configuration and lifetime.** Keep backend configuration distinguishable
  from resolved launch state, live resource ownership and execution results when
  those concepts have different invariants or lifetimes.
- **SE-008 — Capacity contracts.** Distinguish processing units and batching from
  supported-workload ceilings. Document limits, their scope and exhaustion behavior.
  Assess actual workload requirements and impact before treating a limit as a defect.
- **SE-009 — Early validation.** Validate configuration constraints during loading or
  preparation when possible, before dependent side effects. Errors identify the
  exceeded constraint and relevant input. Constraints requiring rendered input are
  checked during preparation once that input is available.
- **SE-010 — Explicit exhaustion.** Supported limits, platform constraints and caller
  budgets fail explicitly or expose documented partial-result semantics. Generous,
  deliberate admission limits may remain when consistent with supported workloads.
- **SE-011 — Complete observations.** Presentation previews and pagination preserve
  access to the underlying supported result. Label partial observations. Failed or
  truncated traversal does not establish completeness, an empty success or deletion.
- **SE-012 — Resource growth.** Describe actual retained memory and resource growth.
  Streaming traversal does not imply bounded memory when results or indexes are
  materialized. Select allocation, backpressure or incremental processing according
  to the supported workload.
- **SE-013 — Behavior evidence.** Tests for relevant limits cover acceptance and
  exhaustion, including error or partial-result behavior. Completeness claims have
  evidence that all supported pages or records are processed.
- **SE-014 — Compatibility.** Refactors preserve runtime behavior and persisted/wire
  compatibility unless a change is requested. Assess real consumers before removing
  public APIs; compatibility aliases must not silently preserve a forbidden boundary.
- **SE-015 — Coherent changes.** Ownership changes update placement, public names,
  dependencies, callers, documentation and relevant tests together.
- **SE-016 — API documentation.** Keep behavioral contracts beside APIs: meaning,
  constraints, parameters/results, errors, ownership, concurrency and cancellation
  as applicable. Scale detail to the API rather than imposing boilerplate.

- **AG-001 — Worktree ownership.** Acquire before feature edits; work in the leased
  checkout, reuse its non-expiring lease, release after integration or PR publication.
  Never wipe main; obtain approval before discarding dirty files.
  Source: AGENTS.md / Agent workflow and referenced worktree-pool skill.
- **AG-002 — Linear integration.** Rebase onto current main, then cherry-pick;
  never integrate with git merge. Source: AGENTS.md / Agent workflow.
- **AG-003 — Dependency evaluation.** Present suitable, maintained, reputable crates
  for user evaluation before adopting a dependency or implementing a custom solution.
  Source: AGENTS.md / Third-party crates.
- **AG-004 — Test structure.** Generally use explicit Arrange, Act and Assert
  sections; repeat Act/Assert for distinct state transitions.
  Source: AGENTS.md / Test structure.
- **AG-005 — Capability tests.** Build unprivileged; elevate only the test executable.
  Sudo is authorized for capability-required tests; persistent administration needs
  separate approval. Source: AGENTS.md / Capability-required tests.
- **AG-006 — Test credentials.** Read .sudo-password from the first main checkout
  through sudo stdin; keep it gitignored and mode 0600. Never expose, commit, copy
  or pass its contents as arguments. Request provisioning if unavailable and stop
  on authentication failure. Source: AGENTS.md / Capability-required tests.
- **AG-007 — ARM64 validation.** Submit trusted checkout source using scripts/ci/submit.py
  to doramcap-ci@100.64.0.20; use the serial default queue and verify the job result
  through Pueue. WSL must be running; pushes do not trigger CI. Build unprivileged;
  native tests use passwordless sudo. Never copy credentials. Run Dora end-to-end
  tests separately. Source: AGENTS.md / ARM64 CI; scripts/ci/README.md.
- **AG-008 — Markdown style.** Use brief prose, short bullets and compact tables.
  Source: AGENTS.md / Documentation style.
- **AG-009 — README scope.** Keep purpose, architecture SVGs, a small usage example
  and links to details. Source: AGENTS.md / Documentation style.
- **AG-010 — API docs.** Document APIs beside code in docstrings/Rust doc comments;
  cover synopsis, behavior, parameters/results, errors, ownership and concurrency
  as applicable, with README links. Refines SE-016.
  Source: AGENTS.md / Documentation style.
- **AG-011 — Direct statements.** State what things do directly and avoid contrastive
  A-not-B phrasing. Source: AGENTS.md / Documentation style.
- **AG-012 — Architecture figures.** Use SVGs with short labels showing components
  and exchanged artifacts. Source: AGENTS.md / Documentation style.
- **AG-013 — Plan location.** Use component/docs/mmddyy-feature-name.md with its creation
  date; keep updates there and supporting SVGs alongside it.
  Source: AGENTS.md / Documentation style.
- **AG-014 — Plan structure.** Lead with an architecture SVG; an optional short
  high-level plan may precede detailed milestones. Give every milestone labeled
  Work items and Validation checklists with acceptance checks beside its work.
  A separate global validation list cannot replace these.
  Source: AGENTS.md / Documentation style.
- **AG-015 — Plan evidence.** Record evidence in the relevant milestone before
  checking an item; complete a milestone only when its work and validations finish.
  Source: AGENTS.md / Documentation style.
