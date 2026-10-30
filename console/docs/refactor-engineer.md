# Selectable first-party apps: refactor review

Baseline: PR 12, `a35bc392462e24986e376f1b3113dda65ea8a8bf` plus the structural changes recorded below, 2026-10-30. Initial review: `573c79f`. Existing lease: pool slot 4. Governing inputs: root AGENTS.md, requirements.md (SE-001–009, SE-014–016), Console plan and shared Compose/Dora architecture documents. Scope: existing behavior and contracts; new app/catalog functionality deferred. Full CI and ARM64 waived by the user for this pass.

## Scope and component map

| Component | Responsibility / exchanged artifact |
| --- | --- |
| Console frontend | Render available app IDs and submit explicit selection; currently already list-based. |
| Host configuration / Console | Approve available recipes and defaults; validate user selection. |
| First-party app assembly | Expand selected typed configuration into projects, runtime participants and output contracts. |
| Shared Compose preparation | Resolve all projects, capture baselines and produce launch groups. |
| ComposeRunner | Own startup, readiness, stop, finalization and cleanup admission. |
| Observations / artifacts | Present capabilities and outcomes for selected apps; keep cleanup evidence independent. |

The structural pass centralizes selection-to-preparation and observation contracts. The shared runner remains the execution owner. Host catalog policy and additional app capabilities remain future features.

## Proposed target

[Current ownership boundaries](console-compose-boundaries.svg) show the implemented integration. The target preserves those owners and changes the selection/preparation contracts:

- **Host catalog:** protobuf app IDs, typed recipes, availability and default selection.
- **Selection:** validate requested IDs and produce only the selected app configuration; explicit empty selects no optional apps.
- **Shared resolution:** expand every selected recipe and resolve all system, ordered first-party and workload projects through shared workflow code.
- **Review receipt:** retain selected IDs, all resolved manifests/image identities and launch order. Start freshly resolves and compares the whole receipt before launch.
- **App outputs:** identify each app's readiness requirements, capabilities, artifacts and finalization outcome.

Console owns host policy, selection UI, admission and presentation. ComposeRunner owns execution and cleanup; orchestrator owns typed invocation and observation. Existing startup/shutdown barriers remain in the shared workflow. Required shared infrastructure is derived from selected participants and disclosed in Review. Optional-app dependency behavior remains an explicit product decision.

## Findings

### RE-APP-001 — Host app catalog and explicit selection

**Deferred feature; high confidence; high priority when another app is introduced.**

Initial evidence: `native/src/system_apps.rs:7–50` accepts at most one CPU-monitor ID, clones every configured app for omitted selection, and derives options/selection from one CPU field. `proto/console.proto:30–32` and frontend `SystemApps.tsx:3` already support lists. Console config rejects Dora flows (`native/src/config.rs:80`).

Introduce a protobuf host catalog with stable app ID, default selection and a typed recipe, plus a reusable selection resolver. Keep host availability separate from the selected execution inputs. Use existing generated schemas and a closed `oneof` for implemented recipes; add another recipe when its real requirements exist. Put app assembly alongside the shared system-app workflow; generic orchestration stays app-independent. No dependency or dynamic plugin system is needed.

Preserve omitted selection = configured defaults, explicit empty = no optional apps, and rejection of unknown/duplicate/unavailable IDs. Resolve launch order independently of checkbox order. A host label/default change should not require editing Console execution branches.

Validation: defaults/empty/single/multiple selection; unknown, duplicate and disabled IDs; selection replacement before Start; no preparation or launch of unselected apps. Dependencies need an explicit policy: auto-include and display required support, or reject an incomplete selection.

### RE-APP-002 — Shared complete review and execution resolution

**Structural work implemented; focused validation passed; full runtime journey unqualified.**

Initial evidence: `native/src/prepare.rs:125–159` resolves the workload plus only `system.projects.into_iter().next()`, stores `workload_digest`/`monitor_image`, and exposes workload services only. `native/src/session.rs:346–355` assembles a separate ComposeRun with empty first_party; `:398–415` checks only the workload manifest and CPU image. Shared `recorded_compose/preparation.rs:92–120` resolves every system/first-party/workload project.

Implemented in `libs/workload-docker/src/workflows/recorded_compose/resolution.rs`: Review and PrepareRun use the same expansion, complete resolution and ordered group assembly. Console retains one ComposeRun instead of reconstructing it at Start. A private ReviewReceipt covers selected configuration, every manifest, pinned image/service policy, group order and recording outputs. Start freshly resolves and compares it before mounts/launch. Only generated system-app artifact binds relocate between Review and Start. Displaying additional app services/infrastructure remains future UI work.

Keep review artifacts separate from execution baselines/resources. Reusing a stale PreparedRun would undermine the existing preflight guarantee. Preserve profile prompts, fresh image checks and cancellation before launch. Tradeoff: shared preparation must expose a preview stage without moving execution side effects into Review.

Validation: receipt tests resolve two generated system projects, ordered first-party and workload groups. They cover changes to a non-first app, image identity, completion policy, group order, foreign binds, frozen-manifest tampering and artifact relocation. Independent resolutions finish before errors propagate; profile-selection prompts retain priority. The existing Docker review test also compares receipts after relocation; runtime evidence is recorded below.

### RE-APP-003 — One app identity across current launch categories

**Existing ordered assembly centralized; selectable identity deferred feature.**

Initial evidence: ComposeRun has both `first_party` and `system_apps` (`recorded_compose/mod.rs:505–511`). Preparation starts system projects, then ordered first-party projects, then foreign workloads (`preparation.rs:148–160`); quiescence reverses groups (`projects.rs::QuiesceProjects`). `system_apps.rs:42` returns no prepared system project without a CPU monitor.

Shared resolution now owns the existing system → ordered first-party → workload grouping. A future catalog can use one selectable first-party app identity at the host boundary. An app recipe may expand into existing support/ordered project categories and shared-runtime participants. Preserve the distinct startup and shutdown semantics underneath; merging the two vectors by name alone would erase real policy. Keep existing public request fields compatible during migration.

Validation: support readiness before dependent workloads, preserved first-party order, awaited sibling failures, reverse shutdown and cleanup of partially launched selected apps. Existing Docker test `parallel_projects_preserve_readiness_and_shutdown_barriers` is relevant but was not executed in this review.

### RE-APP-004 — App-owned capabilities, outputs and outcomes

**CPU-specific review state removed; new capabilities/outcomes deferred feature.**

Initial evidence: Console stored monitor_image, gates metrics/readiness/finalization on cpu_monitor (`native/src/session.rs:333–360,462–466`) and exposes CPU-specific status/artifact fields. PreparedSystemApps already returns vectors of projects/recordings (`workflows/system_apps.rs:18–20`). Recording probes specifically require stream advancement and MCAP finalization (`workload-lifecycle/src/recording.rs:14–58`).

Console no longer stores workload_digest/monitor_image or checks CPU-specific project names for preflight; the complete shared receipt owns that invariant. A future app assembly can return identity-linked capabilities, declared artifacts and applicable lifecycle checks. Keep concrete CPU metrics and MCAP checks specialized. Add per-app results distinguishing unavailable/not applicable, completed and failed; keep global execution and boundary-clean facts independent. Existing CPU wire fields can remain projections for compatibility. Add another probe/output variant only when a real selected app needs it.

Validation: workload-only run; CPU run; non-recording app; two apps with distinct outputs; one finalization failure; no artifact filename collisions; unverified global cleanup still blocks admission.

### RE-APP-005 — Reuse backend constraints in host validation

**Verified correction; high confidence.**

Previously Console accepted any positive stop_seconds while shared preparation required 1..300. Both now call `recorded_compose::validate_budgets`; invalid values fail host loading before state-directory or Docker initialization. The existing supported range is unchanged.

Validation: native config tests accept 1 and 300 and reject 0 and 301; the server addon test confirms 301 rejects before state-directory creation. Focused Rust and server suites passed.

### RE-APP-006 — Coordinate with shared Dora ownership

**Architecture dependency; high confidence; deferred until the next concrete app.**

The agreed `libs/workload-docker/docs/102926-shared-dora-experiment.md:5–26` gives one experiment a shared coordinator/daemon and orders user, producer and recorder shutdown. Treat that plan as an input, not proof that its implementation exists on this branch. Select app participants before launch and let the shared workflow derive required runtime infrastructure. Do not create one Dora runtime per checkbox or introduce a second Console-specific workflow engine.

Validation when implemented: complete selected participant graph, one experiment runtime, dependency disclosure in Review, producer-stop/recorder-drain barriers and ordinary Compose apps without Dora. Routing and dependency-selection product policy remain unresolved.

### RE-APP-007 — Workflow-owned presentation metadata

**Verified structural improvement; high confidence.**

Console previously inferred lifecycle retention, labels and observation status from Rust type names. Renaming a type or adding a workflow could silently change UI behavior.

Step implementations now declare stable ID, display name, retention and kind using generated protobuf StepDescriptor types. LocalExecutor validates declared metadata before invocation; telemetry carries it to native retention and frontend grouping/presentation. Both name lookup tables are removed. Existing generic steps can omit metadata; older captures display diagnostic names without inferred lifecycle classification.

Validation: protobuf transport survives a diagnostic type name change; duplicate/incomplete metadata rejects before factories/events; legacy Invocation JSON decodes. Native retention and frontend tests exercise stable metadata, including an unfamiliar workflow and renamed implementation. The demo consumer compiles.

## Coverage and continuation

| Target | Status / limits |
| --- | --- |
| Catalog and selection | RE-APP-001 deferred feature; existing selection behavior preserved. |
| Review → preflight → preparation | RE-APP-002 structural work implemented; complete receipt unit tests passed. |
| Startup, partial launch, stop, finalization, cleanup | Existing group semantics preserved by shared assembly; RE-APP-003 identity feature deferred. No new launch/cleanup qualification. |
| App outputs | RE-APP-004 review coupling removed; additional capabilities/results deferred feature. |
| Host constraint boundary | RE-APP-005 verified in native and addon tests. |
| Observation boundary | RE-APP-007 verified across Rust/protobuf/native/frontend consumers. |
| Shared Dora and persisted-schema migration | RE-APP-006 deferred; coordinate with the shared implementation owner. |

Executed on x86 Linux using locked dependencies and profile ci:

- `cargo test --profile ci --locked -p console-native -p workload-docker -p orchestrator`: 76 passed; 28 capability/runtime cases ignored.
- Frontend test suite: 30 passed; generated TypeScript/Vite build passed.
- Server test suite: 9 passed; generated TypeScript/native build passed.
- Focused Clippy for those three Rust packages and all targets passed with warnings denied; orchestrator-demo all-target check passed.
- Two existing Docker review-only tests passed using the compiled test executable with sudo: selected profiles/environment/receipt relocation, and profile-only prompting. Unprivileged attempts failed on socket access. No full CI, ARM64, Compose launches or cleanup/capability qualification in this pass.

Next: introduce the next concrete app before designing catalog recipes, capabilities or dependency-selection policy. The shared resolution/receipt and observation contracts are ready for that work. Live app addition, arbitrary plugins and a new scheduler remain deferred.
