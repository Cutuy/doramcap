# Engineering requirements

Reusable engineering audit baseline. Apply each item within its relevant scope;
repository-specific instructions and explicit user requirements govern conflicts.
Read applicable `AGENTS.md` files for current project details and approval rules.
Preferences retain their stated strength; record exceptions and validation limits.

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
- **SE-017 — Simplicity.** Favor deletion, less code and explicit behavior. Under
  governing instructions that require agreement for added complexity, discuss the
  concrete tradeoff and obtain agreement before introducing it.
- **SE-018 — Implementation languages.** Prefer Rust and TypeScript for new code.
  Python is allowed for tests, benchmarks and development tooling; keep it out of
  the application serving stack. Use scripting languages in production environments
  and workflows only when necessary and consistent with governing instructions.
- **SE-019 — Typed configuration.** Favor protobuf messages for internal models and
  application configuration, with pbtxt configuration files. Use generated types
  directly where practical and avoid parallel handwritten Rust configuration structs
  for proto-defined fields. Preserve third-party configuration in its native format
  at integration boundaries.
- **SE-020 — Configuration processing.** Favor TypeScript for dynamic configuration
  processing and protobuf contracts between that processing and Rust execution.
  Every new environment configuration input or extension of an existing environment
  contract requires explicit user approval. Encapsulate approved exceptions behind
  a named interface listing supported variables; use the repository's designated
  interface where one exists.
- **SE-021 — Dependency evaluation.** Present suitable, well-maintained, reputable
  third-party crates for user evaluation before adopting a dependency or implementing
  a custom solution. Record the evaluation or approval evidence available for the
  work under review.
- **SE-022 — Behavior tests.** Assert documented behavior and observable outcomes;
  avoid implementation assumptions and duplicated implementation logic. Use real
  integration inputs where contracts already exist. Generally separate test setup,
  action and assertions with explicit Arrange/Act/Assert comments; repeat action
  and assertion sections for stateful transitions.
- **SE-023 — Validation scope.** Use focused checks for incremental changes and full
  CI at meaningful integration points, for high-risk changes or when requested.
  Follow repository build-profile and locked-dependency rules. When full local CI is
  warranted, prefer triggering cloud CI through the current PR branch after focused
  checks, within publication authorization. Verify the final revision against all
  required checks; record waivers and deferred checks. Targeted tests alone do not
  establish a full CI pass.
- **SE-024 — Capability and platform evidence.** Build unprivileged and elevate only
  capability-required test executables under the governing authorization. Follow
  repository credential handling; request provisioning for missing or unreadable
  credentials and stop on authentication failures. System administration requires
  separate approval. Run required architecture and hardware validation at the stated
  triggers, verify worker completion and report environments separately. Use the
  designated hosts for native, end-to-end and GPU qualification as applicable.
- **SE-025 — Worktree and integration safety.** Follow the repository lease process
  before edits, reuse the existing non-expiring lease and work in its checkout.
  Release after verified local integration or PR publication. Rebase onto current
  main and cherry-pick intended commits to keep main linear. Preserve source changes
  and obtain approval before discarding them; never wipe main or integrate with merge.
- **SE-026 — Documentation and plans.** Use brief prose, short bullets and compact
  tables. Keep READMEs minimal, with purpose, architecture SVGs, a small usage example
  and links. Keep API contracts beside code, with concise doc comments and named
  sections when needed; use one-line comments for key steps in lengthy logic.
  State behavior directly and omit unsolicited caveat footnotes in SVGs and web apps.
  Architecture SVGs show short component labels and exchanged artifacts. Follow
  repository plan paths and dates; lead plans with an embedded architecture SVG and
  give each milestone Work items and Validation checklists. Record evidence before
  checking items and complete milestones only when both checklists are complete.
- **SE-027 — Pull request communication.** Start PR descriptions with a brief summary
  and a single-line validation summary on the first two lines. Follow with concise
  explanations and useful architectural graphs. Explain complex changes through
  their idea and behavior, limiting component inventories.
