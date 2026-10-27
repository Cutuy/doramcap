# Library requirements audit

Reviewed baseline: `f093ab183aa3fe5264ff39b56effd3bc07ca80c1`, plus the fixes
described below (2026-10-27).
Scope: 21 crates under `libs/`; manifests, exports, selected public contracts,
callers/tests, resource boundaries and existing plans. Application implementation
review is excluded. The repository worker uses its standard workspace suite.

Sources: bundled Sane Engineer engineering requirements and root `AGENTS.md`.
No nested library `AGENTS.md` or prior repository requirements file was found.
Created `requirements.md`: 16 baseline items plus 15 repository obligations.
Product-specific acceptance requirements beyond those sources remain unspecified.

## Findings and fixes

The four concrete cases shown in the audit are fixed in this working tree.
The changes preserve workload limits, ownership, wire formats and checked milestone
states. The runtime change is more specific human-readable configuration errors.
This bounded patch updates the cited test cases and plan; it does not claim that
all older library tests and historical plans have been restyled.

- **F1 — Resolved:** `libs/pbtxt-config/src/lib.rs:205` now distinguishes depth,
  visits, per-file bytes and aggregate bytes. Each error names the input, threshold
  and observed count. The bounded file read reports an honest lower bound.
  Four regression tests cover acceptance at each limit and rejection beyond it.
- **F2 — Resolved:** `libs/telemetry/mcap/src/embedded.rs:143` documents publication
  before commit, callback lifetime, separate effects, acceptance and durability.
  Creation, polling, rotation and finish also have public contracts. Storage append
  methods document rejection ownership and later I/O failure reporting.
- **F3 — Resolved for the cited cases:** the state transitions in
  `libs/fault/aggregation/tests/lifecycle.rs:64` and the atomic-rejection test in
  `libs/step-provenance/tests/engine.rs:142` now have explicit Arrange/Act/Assert
  sections. Maintained pbtxt and recording tests also have phase comments.
- **F4 — Resolved for the cited plan:** all G1–G4 milestones in
  `libs/storage-persistence/docs/101926-growable-buffer-pool.md` have separate
  Work items and Validation checklists with evidence links beside them. Every
  historical work item and checked/unchecked state is preserved. Historical check
  timing remains unverified; no completion evidence was invented.

## Requirement-by-requirement results

Statuses apply to the stated evidence and scope. A met entry based on sampled
inspection is not a claim that every function or platform was exhaustively verified.
Conditional workflow obligations describe this pass, not historical compliance.

| ID | Concrete requirement | Status | Evidence / gap |
| --- | --- | --- | --- |
| SE-001 | Concept ownership | met | Public concepts follow their owners in the sampled APIs: resource snapshots, fault state, storage leases, Docker plans and executor events. See crate coverage below. |
| SE-002 | Execution boundaries | met | orchestrator/src/lib.rs:13 and executor.rs own invocation/factories; workload-docker/src/workflows/recorded_compose.rs owns run policy and teardown. Dependency rules pass. |
| SE-003 | Backend contracts | met | DockerAdapter, ComposeProject and filesystem steps are in workload-docker; cuda-dora-shm owns Dora Ready/Done protocol. No backend dependencies enter orchestrator production edges. |
| SE-004 | Input independence | met | Step inputs are typed serialized values; ComposeProject and resource-policy Goal describe prepared settings. pbtxt-config owns file composition as an explicit adapter. |
| SE-005 | Shared abstractions | met | session/src/lib.rs and workload-lifecycle/src/lib.rs remain documented design scaffolds without invented runtime interfaces. Shared fault/schema and workflow contracts have demonstrated consumers. |
| SE-006 | Dependency direction | met | scripts/ci/boundaries.json, local checker plus four fixtures, and ARM64 job 149 passed. The graph check is supplemented by ownership inspection. |
| SE-007 | Configuration and lifetime | met | ComposeProject, ResolvedComposeProject, ComposeRunner and ComposeRunReport have distinct lifetime semantics; storage and CUDA leases retain actual backing resources. |
| SE-008 | Capacity contracts | met | pbtxt README documents limits; six probes verify exact admission/exhaustion at visit, file-byte and aggregate-byte boundaries. Other sampled limits expose budgets, counts or loss. No limit is rejected solely for its magnitude. |
| SE-009 | Early validation | met | F1 fixed: four distinct limit errors include source path, threshold and observed count; oversized files use a bounded-read lower bound. Four at/over-boundary regression tests pass. |
| SE-010 | Explicit exhaustion | met | Explicit errors in pbtxt/config, fault publications and provenance; monitor-core capacity_overflow; task accounting losses; CUDA full-pool None; tracing loss counters. No sampled exhaustion path falsely reported complete success. |
| SE-011 | Complete observations | met | workload-docker/compose/filesystem.rs:279, :540 and :565 suppress diff inference after incomplete scans; public streams preserve categories. Large/deep readback and malformed-row tests passed on ARM64. Provenance Lineage separates missing/expired roots. |
| SE-012 | Resource growth | met | Filesystem docs and MetadataWalk describe depth-dependent handles and materialized metadata maps. Storage growable-pool docs describe retained buffers/registration. Native allocation validation passed; CUDA pinning cost is documented. |
| SE-013 | Behavior evidence | unverified | Threshold/error tests passed in standard worker suites and six extra probes. Hardware-dependent CUDA tests and the ignored real-Docker workflow harness were not run in this pass; their relevant behavior remains unverified. |
| SE-014 | Compatibility | met | Only human-readable Error::Invalid diagnostics change at runtime. Workload bounds, public types and persisted/wire schemas are preserved; recording API edits document existing behavior. |
| SE-015 | Coherent changes | met | The bounded corrections update resolver behavior/doc comments/tests, public recording contracts, cited stateful tests, milestone structure and this audit together. No dependencies or ownership migrations. |
| SE-016 | API documentation | met | F2 fixed for reviewed APIs: Recorder and McapWriter methods document borrowing, rejection ownership, publication/commit ordering, rotation and finalization. Strict Rustdoc with warnings denied passes. |
| AG-001 | Worktree ownership | met | Lease db8533bb-08a0-4457-a912-4ba8e0cabcbd, slot 2, codex/libs-requirements-audit. Acquired before edits with a clean inherited diff; release uses the user-requested local integration flow and pool verification. |
| AG-002 | Linear integration | met | User requested release after validation. Local integration follows rebase onto current main, then cherry-pick; the pool registry records feature/main commit IDs after verification. |
| AG-003 | Dependency evaluation | unverified | No dependency or custom implementation adopted in this audit. Historical evaluation/approval cannot be established from manifests alone; resource-policy plan retains unchecked evaluation evidence. |
| AG-004 | Test structure | met | F3 cited fault/provenance stateful tests now have explicit Arrange/Act/Assert transitions. Maintained pbtxt and recording tests also follow this structure; unreviewed older tests are outside this bounded patch. |
| AG-005 | Capability tests | met | Worker runner builds with unprivileged Cargo then invokes the authorized native-test launcher through sudo. No local privileged tests or persistent configuration changes were performed. |
| AG-006 | Test credentials | unverified | Local password-based sudo was not exercised. Worker uses its separately authorized passwordless native launcher; no credentials copied or read by this audit. |
| AG-007 | ARM64 validation | met | Audit job 149 and fix job 150 passed in the serial default queue. Fix snapshot 9b9eac5e04314dc19409b0b0e820992f passed the full standard worker suite. |
| AG-008 | Markdown style | met | New audit files use compact bullets/tables. Existing library documents were sampled; this is not an exhaustive prose-style lint. |
| AG-009 | README scope | needs clarification | README purpose/examples/details links are present. activity-tracing and native backend READMEs also retain substantial contract/qualification detail. Minimal has no numeric threshold; no defect is inferred from length alone. |
| AG-010 | API docs | met | F2 resolved by API-adjacent doc comments in telemetry/mcap/src/embedded.rs and storage-persistence/src/mcap.rs; strict Rustdoc passes. |
| AG-011 | Direct statements | met | Audit requirements/findings state behavior directly. Existing prose was sampled; no exhaustive editorial compliance claim. |
| AG-012 | Architecture figures | met | Architecture SVGs are colocated under library docs. XML validation performed for all in-scope SVGs; component and exchanged-artifact labels sampled. |
| AG-013 | Plan location | met | Sampled plan paths use component/docs/mmddyy-feature-name.md and adjacent SVGs. Existing creation dates are preserved; no new plan was created. |
| AG-014 | Plan structure | met | F4 cited growable-pool plan now has Work items and Validation checklists in each of G1–G4, with per-milestone evidence links. Other historical plans were not migrated by this bounded patch. |
| AG-015 | Plan evidence | unverified | Current cited milestones contain evidence links and preserve every existing completion state. Historical timing of checked items cannot be reconstructed from this change. |

## Crate coverage

Every manifest and public entrypoint was inventoried. Deeper review focused on the
contracts below; remaining internal algorithms are not claimed exhaustively proved.

| Crate / path under libs | Contracts traced / evidence |
| --- | --- |
| activity-tracing | Config admission, capture loss, worker handoff, OtelSink capacity/drop accounting; capture/output/OTel tests in worker. |
| fault/schema | Generated schema ownership and descriptor compatibility tests. |
| fault/reporter | Fixed policy/text bounds, Publisher::batch, snapshot/drain and loss ownership. |
| fault/aggregation | Admission/revision preflight, reconciliation and bounded output gaps; lifecycle tests. |
| fault/protocol | Checked wire conversion, frame/record budgets and publication tests. |
| memory/cuda-host-memory | Independent slot leases, guard exclusion, unsafe enqueue and quarantine semantics; CPU/unit/compile-fail tests pass, live CUDA ignored. |
| memory/cuda-dora-shm | PoolConfig/Manifest validation, preallocated backing, Ready/Done ledger and no-timeout reuse; live registration tests ignored. |
| orchestrator | Typed Step contracts, factory lifetime, observer isolation, process I/O budgets, telemetry loss; integration tests. |
| pbtxt-config | Overlay order, cycle handling, loading thresholds and retained sources; six extra boundary probes. |
| resources/resource-policy | Immutable Goal/CpuSet, pbtxt adapter, Linux plan/apply partial-state errors and explicit host limits; worker native tests. |
| resources/cgroup-bpf | Numeric native collection, configured map/output capacity, explicit overflow and root flags; worker native accounting/allocation tests. |
| resources/task-accounting | Lifetime/migration identity, retained capacity and losses; experimental qualification boundaries remain documented. |
| resources/monitor-core | Snapshot push capacity_overflow, bounded lease handoff, virtual task aggregate ownership; allocation tests. |
| session | Design-only identity/evidence scaffold; no speculative runtime API. |
| step-provenance | Atomic rejected patches, retention/tombstones, lineage completeness and explicit traversal errors; engine tests. |
| storage-persistence | Managed/slab/external ownership, writer preflight, rejection returns, completion/durability, bounded observation history and growable buffers; worker tests and allocation probes. |
| telemetry/protocol | Producer measurements/dictionary ownership, frame size/schema validation, missing identity checks and wire tests. |
| telemetry/workflow-protocol | Independent workflow codec, host-owned size/version validation, fixed descriptor/v1 bytes tests. |
| telemetry/mcap | Standalone decode/write boundary and embedded publication-before-commit path; callback/recording equality tests. |
| workload-docker | Compose configuration/resolved plans, runner admission/cleanup, category-preserving filesystem scan/diff/stream; deep/large tests pass, real Docker harness ignored. |
| workload-lifecycle | Design scaffold; backend implementation remains in workload-docker. |

## Validation

| Check | Result |
| --- | --- |
| Local dependency graph and checker fixtures | Audit graph and four fixtures passed; dependency declarations are unchanged by the fixes. |
| Resolver probes via existing resource-policy example | Six passed: at/over 256 total visits, 1 MiB/file and 8 MiB aggregate. Errors reproduced F1. |
| ARM64 serial default queue | Job 149 succeeded, 10:03:04–10:04:24 PDT, 2026-10-27. Snapshot `ce51079464ed48fa875aa26d36ea3fb0`. |
| Worker default/all-feature Rust, formatting and strict Clippy | Passed. Includes storage without default features and examples/binaries build. |
| Native/resource/recording validation | Passed; native 40-iteration probe reported zero userspace allocations; Dora equality reported 24 snapshots / 26 messages. |
| Filesystem completeness | Deep/large test covered 10,142 entries and preserved every JSONL change category, plus malformed-record failure. |
| Live CUDA and real Docker workflow | Ignored by standard suite; not exercised separately in this pass. |

Worker log: `/tmp/doramcap-libs-audit-arm64-149.log`.
Resolver fixtures/results: `/tmp/doramcap-libs-audit-limits-_6ooif9i/results.json`.
Local example build log: `/tmp/doramcap-libs-audit-local-build.log`.

## Fix validation

| Check | Result |
| --- | --- |
| Local pbtxt-config, fault-aggregation and step-provenance | 43 tests and one doctest passed; includes four new acceptance/exhaustion tests. |
| Strict affected-target Clippy | Passed all-targets/all-features with warnings denied. |
| Strict affected API Rustdoc | Passed all-features with warnings denied. |
| Formatting and whitespace | Passed cargo fmt and git diff --check. |
| Milestone preservation | All four milestones have separate work/validation sections; links resolve and every prior work/completion state is preserved. |
| Final-fix ARM64 | Job 150 succeeded, 10:18:23–10:20:55 PDT on 2026-10-27; default/all-feature tests, formatting, strict Clippy, native accounting/allocation and recording validation passed. |

Logs: `/tmp/doramcap-libs-fixes-{tests,clippy,rustdoc}.log` and
`/tmp/doramcap-libs-fixes-arm64-150.log`. Worker snapshot:
`9b9eac5e04314dc19409b0b0e820992f`.

After submission, one doc-only clarification corrected `reserve_large` wording to
match its existing compatibility adapter behavior. Strict storage Rustdoc was
rerun successfully; log: `/tmp/doramcap-libs-fixes-storage-rustdoc.log`.

## Requirements preservation and handoff

Unchanged AGENTS/baseline sources require no second-pass reconciliation edits.
Requirements SHA-256 remains
`6991d20d2876d7a09889f0e726a11d760e4318fbf86d762fca26ff312bec080c`.
All 31 IDs are unique. Run-specific results remain in this checklist.

Six tracked library files contain the fixes; the two audit documents are added.
No dependencies or persisted/wire schemas change. The user requested local release
on 2026-10-27: commit the reviewed files, rebase onto current main, cherry-pick and
release with pool verification. The pool registry records integration commit IDs
and final lease state for slot 2, branch `codex/libs-requirements-audit`.
