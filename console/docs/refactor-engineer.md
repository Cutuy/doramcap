# Selectable first-party apps: refactor review

Baseline: PR 12, `573c79f1b66c3901a059636c492a840f7a71fab5`, pushed 2026-10-30. Review only; no implementation changes or checks executed in this pass. Existing lease: pool slot 4. Governing inputs: root AGENTS.md, requirements.md (SE-001–009, SE-014–016), Console plan and shared Compose/Dora architecture documents.

## Scope and component map

| Component | Responsibility / exchanged artifact |
| --- | --- |
| Console frontend | Render available app IDs and submit explicit selection; currently already list-based. |
| Host configuration / Console | Approve available recipes and defaults; validate user selection. |
| First-party app assembly | Expand selected typed configuration into projects, runtime participants and output contracts. |
| Shared Compose preparation | Resolve all projects, capture baselines and produce launch groups. |
| ComposeRunner | Own startup, readiness, stop, finalization and cleanup admission. |
| Observations / artifacts | Present capabilities and outcomes for selected apps; keep cleanup evidence independent. |

The first target is selection-to-preparation: adding a second app currently touches several CPU-specific branches, and Review resolves fewer inputs than the full runner. The shared runner and its lifecycle barriers should remain the execution owner.

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

**Supported structural improvement; high confidence; proposed; high priority.**

Evidence: `native/src/system_apps.rs:7–50` accepts at most one CPU-monitor ID, clones every configured app for omitted selection, and derives options/selection from one CPU field. `proto/console.proto:30–32` and frontend `SystemApps.tsx:3` already support lists. Console config rejects Dora flows (`native/src/config.rs:80`).

Introduce a protobuf host catalog with stable app ID, default selection and a typed recipe, plus a reusable selection resolver. Keep host availability separate from the selected execution inputs. Use existing generated schemas and a closed `oneof` for implemented recipes; add another recipe when its real requirements exist. Put app assembly alongside the shared system-app workflow; generic orchestration stays app-independent. No dependency or dynamic plugin system is needed.

Preserve omitted selection = configured defaults, explicit empty = no optional apps, and rejection of unknown/duplicate/unavailable IDs. Resolve launch order independently of checkbox order. A host label/default change should not require editing Console execution branches.

Validation: defaults/empty/single/multiple selection; unknown, duplicate and disabled IDs; selection replacement before Start; no preparation or launch of unselected apps. Dependencies need an explicit policy: auto-include and display required support, or reject an incomplete selection.

### RE-APP-002 — Shared complete review and execution resolution

**Supported structural improvement; high confidence; proposed; highest priority.**

Evidence: `native/src/prepare.rs:125–159` resolves the workload plus only `system.projects.into_iter().next()`, stores `workload_digest`/`monitor_image`, and exposes workload services only. `native/src/session.rs:346–355` assembles a separate ComposeRun with empty first_party; `:398–415` checks only the workload manifest and CPU image. Shared `recorded_compose/preparation.rs:92–120` resolves every system/first-party/workload project.

Extract the shared app expansion and complete project-resolution stage for Review and execution. Retain existing ComposeRun as execution intent and PreparedRun as run-specific state; introduce only the missing resolved-input/receipt contract. The receipt covers normalized selected IDs, ordered launch groups and every resolved manifest/image identity, keyed by stable project ID. Start freshly resolves and compares the complete receipt before mounts/launch. Review presents all selected services and required infrastructure.

Keep review artifacts separate from execution baselines/resources. Reusing a stale PreparedRun would undermine the existing preflight guarantee. Preserve profile prompts, fresh image checks and cancellation before launch. Tradeoff: shared preparation must expose a preview stage without moving execution side effects into Review.

Validation: selecting two apps reviews and pins both; changing either invalidates Start; removed/added apps invalidate the receipt; unselected image failures do not block a run; no launch follows stale-review rejection.

### RE-APP-003 — One app identity across current launch categories

**Supported structural improvement; high confidence; proposed; medium priority.**

Evidence: ComposeRun has both `first_party` and `system_apps` (`recorded_compose/mod.rs:505–511`). Preparation starts system projects, then ordered first-party projects, then foreign workloads (`preparation.rs:148–160`); quiescence reverses groups (`projects.rs::QuiesceProjects`). `system_apps.rs:42` returns no prepared system project without a CPU monitor.

Use one selectable first-party app identity at the host boundary. An app recipe may expand into existing support/ordered project categories and shared-runtime participants. Preserve the distinct startup and shutdown semantics underneath; merging the two vectors by name alone would erase real policy. Keep existing public request fields compatible during migration.

Validation: support readiness before dependent workloads, preserved first-party order, awaited sibling failures, reverse shutdown and cleanup of partially launched selected apps. Existing Docker test `parallel_projects_preserve_readiness_and_shutdown_barriers` is relevant but was not executed in this review.

### RE-APP-004 — App-owned capabilities, outputs and outcomes

**Supported structural improvement; high confidence; proposed; medium priority.**

Evidence: Console stores monitor_image, gates metrics/readiness/finalization on cpu_monitor (`native/src/session.rs:333–360,462–466`) and exposes CPU-specific status/artifact fields. PreparedSystemApps already returns vectors of projects/recordings (`workflows/system_apps.rs:18–20`). Recording probes specifically require stream advancement and MCAP finalization (`workload-lifecycle/src/recording.rs:14–58`).

Have app assembly return identity-linked capabilities, declared artifacts and applicable lifecycle checks. Keep concrete CPU metrics and MCAP checks specialized. Add per-app results distinguishing unavailable/not applicable, completed and failed; keep global execution and boundary-clean facts independent. Existing CPU wire fields can remain projections for compatibility. Add another probe/output variant only when a real selected app needs it.

Validation: workload-only run; CPU run; non-recording app; two apps with distinct outputs; one finalization failure; no artifact filename collisions; unverified global cleanup still blocks admission.

### RE-APP-005 — Reuse backend constraints in host validation

**Confirmed mismatch; high confidence; proposed; small follow-up.**

`native/src/config.rs:58–66` accepts any positive stop_seconds; shared `preparation.rs:175–183` requires 1..300. Values above 300 pass configuration/review and fail preparation at Start. Centralize the existing backend constraint and apply it at host load. Preserve the established range and identify the invalid field. This also prevents future app-specific constraints drifting between Review and execution.

Validation: 1 and 300 accepted; 0 and 301 rejected before StateStore/Docker side effects. Not changed or executed in this review.

### RE-APP-006 — Coordinate with shared Dora ownership

**Architecture dependency; high confidence; deferred until the next concrete app.**

The agreed `libs/workload-docker/docs/102926-shared-dora-experiment.md:5–26` gives one experiment a shared coordinator/daemon and orders user, producer and recorder shutdown. Treat that plan as an input, not proof that its implementation exists on this branch. Select app participants before launch and let the shared workflow derive required runtime infrastructure. Do not create one Dora runtime per checkbox or introduce a second Console-specific workflow engine.

Validation when implemented: complete selected participant graph, one experiment runtime, dependency disclosure in Review, producer-stop/recorder-drain barriers and ordinary Compose apps without Dora. Routing and dependency-selection product policy remain unresolved.

## Coverage and continuation

| Target | Status / limits |
| --- | --- |
| Catalog, selection, app preparation | Reviewed statically; RE-APP-001/004 proposed. |
| Review → preflight → full Compose preparation | Both sides traced; RE-APP-002/005 proposed. |
| Startup, partial launch, stop, finalization, cleanup | Static journey traced; RE-APP-003 proposed. No new Docker/runtime evidence. |
| Shared Dora integration and persisted-schema migration | Follow-up; coordinate with the existing architecture plan and its current implementation owner. |

Next: agree the second concrete first-party app and dependency-selection behavior. Implement RE-APP-002 and RE-APP-001 as one vertical slice through the existing runner, then add only the capabilities needed by that app. Keep live app addition, arbitrary plugin loading and a new scheduler deferred.
