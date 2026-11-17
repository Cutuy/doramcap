# Holoscan runner stack

Created November 16, 2026.
Plan: [#264](https://github.com/Cutuy/doramcap/pull/264).

Implement the seven runner milestones as stacked pull requests. The web app gets
its implementation plan after the application-only run works. Nothing is merged
by this task.

## Work

| Milestone | Scope | PR |
| --- | --- | --- |
| Prerequisite | Shared normal completion | [#442](https://github.com/Cutuy/doramcap/pull/442), ready |
| Prerequisite | Native Docker socket check | [#446](https://github.com/Cutuy/doramcap/pull/446), ready |
| Prerequisite | Supervisor removal retry | [#445](https://github.com/Cutuy/doramcap/pull/445), ready |
| Prerequisite | Raw Module Docker inputs | [#447](https://github.com/Cutuy/doramcap/pull/447), ready |
| Prerequisite | Empty-slice cleanup race | [#455](https://github.com/Cutuy/doramcap/pull/455), ready |
| Prerequisite | Container-only CLI environment | [#460](https://github.com/Cutuy/doramcap/pull/460), ready |
| Prerequisite | Application-slice CLI containment | [#461](https://github.com/Cutuy/doramcap/pull/461), ready |
| Prerequisite | Shared Compose filesystem preparation | [#463](https://github.com/Cutuy/doramcap/pull/463), ready |
| 1 | Lightweight real CLI, container and overlay probes | [#444](https://github.com/Cutuy/doramcap/pull/444), ready |
| 2 | Crate, protobuf contract and discovery | [#443](https://github.com/Cutuy/doramcap/pull/443), ready |
| 3 | Limited Prepare and Review | [#448](https://github.com/Cutuy/doramcap/pull/448), ready |
| 4 | Build request | [#458](https://github.com/Cutuy/doramcap/pull/458), ready |
| 5 | Application run | [#465](https://github.com/Cutuy/doramcap/pull/465), draft |
| 6 | System apps | [#464](https://github.com/Cutuy/doramcap/pull/464), draft |
| 7 | Recovery and acceptance | Controller-loss and admission harness in implementation |

## Agents

| Agent | Model | Assignment |
| --- | --- | --- |
| `holoscan_probe_audit` | GPT-6.1 Sol, xhigh | Native process containment and shared cleanup retry |
| `holoscan_contract_audit` | GPT-6.1 Sol, xhigh | Build complete; milestone 7 recovery and admission |
| `holoscan_discovery` | GPT-6.1 Sol, xhigh | Milestone 3 limited Prepare/Review, completed |
| `holoscan_container_environment` | GPT-6.1 Sol, xhigh | Environment complete; milestone 5 application run |
| `holoscan_process_scope` | GPT-6.1 Sol, xhigh | Shared filesystem reuse reviewed; milestone 6 system apps |

## Decisions

The agreed decisions in #264 remain the baseline. Each implementation decision
below records its approval or pending status.

### 1. Normal application completion — approved

Context: Holoscan must preserve the application's exit code and allow a successful
normal exit. The shared lifecycle currently treats every natural end as an error.

```mermaid
flowchart LR
  H[Holoscan exits] --> A[Shared completion result]
  H --> B[Holoscan-specific run loop]
  A --> C[Shared stop and cleanup]
  B --> D[Separate stop and cleanup]
```

| Option | Effect |
| --- | --- |
| Shared completion result, recommended | Change `Running::ended` to return success or failure; preserve existing backends' behavior and shared teardown. |
| Holoscan-specific run loop | Handle normal completion in a separate lifecycle sequence owned by Holoscan. |

Choice: the owner approved the shared completion result on November 16. It keeps
lifecycle policy with its current owner and avoids duplicating teardown.

### 2. Lightweight lifecycle application — approved

Context: the owner asked for a lightweight project for lifecycle testing. The two
applications in #264 require replay data and exercise additional application
behavior. Their full acceptance remains later work.

```mermaid
flowchart LR
  P[Lifecycle tests] --> S[first_pipeline]
  P --> R[CLI smoke_app]
  S --> G[Real SDK graph]
  R --> C[Python print command]
```

| Option | Effect |
| --- | --- |
| Official SDK first_pipeline, recommended | Runs three CPU operators for ten values, requests stop and checks termination. |
| Official CLI smoke_app | Exercises CLI command execution without an SDK graph. |

Choice: the owner approved the unchanged SDK example with native metadata and
a small test image. It uses CUDA runtime libraries, supplied by the CUDA base and
NVIDIA runtime, without GPU computation, a dataset, model or display. The two
larger applications remain later acceptance work.

Source: [SDK first_pipeline](https://github.com/nvidia-holoscan/holoscan-sdk/tree/358c772f2d2e88d117651a961d6f6bf9b650cf8d/examples/first_pipeline).
The earlier synthetic-mode proposal was superseded by this investigation; it was
not approved or executed.

### 3. Console configuration — approved

Context: milestone 2 adds the configuration already planned in #264. Automatic
approval review required direct confirmation of those additions.

```mermaid
flowchart LR
  C[Console configuration] --> H[HOLOSCAN form]
  H --> P[Absolute CLI path]
  C --> O[Other forms]
  O --> R[Reject Holoscan block]
```

| Option | Effect |
| --- | --- |
| Planned configuration, recommended | Add `form: HOLOSCAN` and required `holoscan { cli }`; reject the block for other forms. |
| Defer configuration | Leave discovery disconnected from Console's configured backend. |

Choice: the owner explicitly approved the planned configuration on November 16.

### 4. Docker routing — approved

Context: Holoscan, system containers and Console's Engine API must reach the same
Docker daemon. Compose and Holoscan workflows do not need to run together. CLI
5.0 has no daemon host/context flag or project setting; a child `DOCKER_HOST`
override also disables its automatic local CPU-limit forwarding.

```mermaid
flowchart LR
  H[Native Docker configuration] --> A[Daemon ID]
  C[Console docker_socket] --> B[Daemon ID]
  A --> V{Equal?}
  B --> V
  V -->|Yes| R[Build or Start]
  V -->|No or unknown| F[Reject]
```

| Option | Effect |
| --- | --- |
| Matching native setup, recommended | Preserve native routing and CPU behavior; compare daemon identity before Build and Start. |
| Child DOCKER_HOST override | Select Console's socket explicitly; lose automatic local CPU-limit forwarding. |

Choice: the owner approved native setup when the daemon matches and requested
a socket-based check without a Docker subprocess. Resolve the native endpoint
from Docker configuration, then query the Engine API directly. The owner approved
implementation on November 16. Routing must remain stable during each operation;
per-command routing wrappers are unsupported. Docker routing values remain unchanged;
decision 10 records the later approved container-execution selector.

### 5. Catalog process ownership — approved

Context: closing Console currently kills an active catalog CLI as its runtime
shuts down, without reaping it. The cancellation check exposes a persistent zombie.

```mermaid
flowchart LR
  C[Console closes] --> O[Existing Rust execution thread]
  O --> S[Signal query shutdown]
  S --> K[Kill and reap CLI]
  K --> J[Join catalog tasks]
  J --> R[Close runtime]
```

| Option | Effect |
| --- | --- |
| Existing Rust execution thread, recommended | Track catalog tasks and await child cleanup before closing their runtime. |
| Separate process supervisor | Add a dedicated component for child ownership. |

Choice: the owner approved the existing execution thread waiting for catalog
cancellation and process reaping. This adds no separate process supervisor.

### 6. Process containment and cleanup — reuse existing slices

Context: the owner asked whether detached processes and cgroups suffice, and
required idempotent pre-run and post-run cleanup across workflows. A proposed
separate CLI cgroup was superseded by the owner's instruction to reuse the
existing application slices.

```mermaid
flowchart LR
  P[Repository] --> R[Run]
  R --> S[System apps]
  R --> U[User apps]
  U --> H[Holoscan application]
```

| Option | Effect |
| --- | --- |
| Existing application hierarchy, owner choice | Reuse repository/run/user/application placement and cleanup. |
| Separate CLI hierarchy | Introduce another containment boundary and recovery path. |

Keep the actual `tokio::process::Child` returned by spawn until `wait()` reaps it.
Cgroup emptiness does not establish reaping. Docker owns the containers and Engine
cleanup verifies them separately. Catalog remains usable without cgroup access.

Cleanup accepts missing owned resources and resumes partial removal. It verifies
writers have stopped and preserves unrelated resources. Repeating teardown must
not replay capture or write-back. Console retains the admission lock; Holoscan
children use its default close-on-exec behavior so the next controller can acquire
admission and clean surviving owned resources.

### 7. Module application compatibility — approved

Context: CLI 5.0 uses JSON metadata for application and Module projects. Only
Module projects read optional `pyproject.toml` defaults from `[tool.holoscan]`.
Their Docker options are hidden from dry-run output. Read the raw files for
provenance; the limited Review does not reconstruct their effective options.
Application YAML remains native.

```mermaid
flowchart LR
  A[Application JSON] --> R[Build and Run]
  M[Module JSON] --> T[Optional TOML defaults]
  T --> R
```

| Option | Effect |
| --- | --- |
| Run applications in both project types, recommended | Preserve Module project defaults; module creation and packaging remain outside Console. |
| Application projects only | Defer application execution inside Module projects. |

NVIDIA documents the Module workflow and lists Deltacast and RTI community
modules; that establishes real use, without establishing broad adoption. After
reviewing this evidence, the owner approved application Build/Run in both project
types and the existing locked `toml` crate on November 16.

For read-only TOML support, `toml` and `toml_edit` are reputable candidates from
[toml-rs](https://github.com/toml-rs/toml), both already in Cargo.lock.
`toml` is recommended because formatting-preserving editing is unnecessary.

Sources: [Module tutorial](https://nvidia-holoscan.github.io/tutorials/create-a-module/),
[community catalog](https://nvidia-holoscan.github.io/modules/).


### 8. Source metadata — follow existing workflows

Context: the owner directed Holoscan to follow Compose/systemd source capture.
Both workflows use the same Console helper. It reads current HEAD and status
independently; local changes are Dirty, failed status is Unknown, and an unreadable
commit stays absent. There is no cached-commit fallback.

```mermaid
flowchart LR
  B[Build source check] --> K[Known commit and status]
  B --> U[Unknown]
  K --> V[Verify freshness at Review]
  U --> A[Allow with unverified freshness]
  U --> R[Require Git metadata]
```

| Option | Effect |
| --- | --- |
| Allow Unknown, recommended | Keep copied projects usable; explicitly mark source freshness unverified. |
| Require known Git metadata | Refuse Build/Run until commit and status are available. |

Choice: follow the existing workflows, as the owner directed on November 16.
Dirty, Unknown or absent Git metadata alone do not gate execution. Retain the
readable current commit with Clean, Dirty or Unknown. Their freshness remains
unverified; known changes relative to a build still require rebuilding as #264
specifies.
Unknown never becomes Dirty merely because a Git command failed.


### 9. Limited Review — approved

Context: CLI 5.0 hides configured Docker options in dry-run output. Its human
command formatter also loses quoting information. A complete settings screen
would require additional interpretation or interception. The owner rejected
that complexity and chose limited Review.

```mermaid
flowchart LR
  A[Verified selection and source facts] --> R[Console Review]
  I[Verified image facts] --> R
  D[CLI dry-run text] --> P[Partial diagnostic evidence]
  P --> R
  U[Unavailable launch settings] --> N[Unknown]
  N --> R
```

| Option | Effect |
| --- | --- |
| A: limited Review, owner choice | Show verified facts and explicitly identify unavailable launch details. Preserve CLI execution. |
| B: interpret native configuration | Duplicate CLI parsing and expansion without a complete resolved launch. Rejected. |
| C: intercept Docker invocation | Add an executable and paused-launch lifecycle; structured settings still need interpretation. Rejected. |

Choice: on November 16 the owner instructed, "Just follow A." Remove the
comprehensive pre-run inspection promise from #264. Keep raw native input
fingerprints and approved source freshness checks. Do not add a shell tokenizer,
Docker-option parser, interception executable or new Docker-option environment
reads. CLI display text is partial evidence, never exact argv or proof that
mounts/conflicts are absent. Existing mandatory constraints still require proof;
unknown inputs cannot satisfy them. Container inspection after launch does not
provide a pre-start guarantee.

### 10. Container execution selector — approved

Context: this workflow supports Docker execution. CLI 5.0 can select host
execution through inherited `HOLOSCAN_CLI_BUILD_LOCAL` or native mode defaults;
Build and Run have no flag that forces container execution.

```mermaid
flowchart LR
  C[Build and Run] --> A[Child BUILD_LOCAL=0]
  A --> D[Native container workflow]
  C --> B[Unchanged environment]
  B --> H[Host execution remains possible]
```

| Option | Effect |
| --- | --- |
| Child-only `HOLOSCAN_CLI_BUILD_LOCAL=0`, recommended | Force native container execution for Build, Run and Prepare's dry run; preserve Docker routing. |
| Keep environment unchanged | Defer Build/Run because this backend cannot establish container execution. |

Choice: the owner explicitly approved child-only `HOLOSCAN_CLI_BUILD_LOCAL=0`
for Build, Run and Prepare's dry run. Native source and an isolated dry-run matrix
establish the selector's behavior. Catalog queries and Docker routing retain their
existing environment. Implementation now proceeds through one named child-command
interface, shared by these callers.

### 11. Encode Console arguments — approved

Context: the CLI accepts Console's Docker additions as one string. Paths containing
spaces or quotes must survive its native tokenization.

```mermaid
flowchart LR
  V[Console argument values] --> A[Locked shlex encoder]
  V --> B[Custom quoting]
  A --> H[Holoscan CLI]
  B --> H
```

| Option | Effect |
| --- | --- |
| Locked `shlex` 2.0.1, recommended | Encode Console's own values with `try_join`; reject NUL. |
| Custom quoting | Maintain equivalent encoding code and edge cases in this backend. |

Choice: the owner approved the existing locked crate. This encodes generated
arguments; project Docker settings retain their native interpretation.

### 12. Current helper recovery — keep one record

Context: restart cleanup must find the preceding CLI helper before another Build
or Run. Attempt-local records require separate Build and Run history traversal.

```mermaid
flowchart LR
  B[Build] --> R[One project helper record]
  U[Run] --> R
  R --> C[Shared cleanup]
  C --> D[Clear after cleanup]
```

| Option | Effect |
| --- | --- |
| One record in the existing project recovery directory, recommended | Build and Run recover the current helper directly. |
| Attempt-local records | Search stored Build and Run attempts; recovery depends on the chosen data folder. |

Choice: the owner delegated the choice with “Keep it simple and stupid.” Use one
authoritative current-helper record and one cleanup path. Replace attempt-history
traversal; retain logs and results in their existing artifact folders.

## Events

- November 16: read #264 at `635cd8fd` and both repository guides. Verified the
  listed shared prerequisites, including #364, in `origin/main` at `e3fc8911`.
- November 16: started two independent read-only audits. Acquired a separate
  worktree lease for this log and the local review report; published this log in
  [#441](https://github.com/Cutuy/doramcap/pull/441).
- November 16: backend audit completed. Prepare and Build can proceed independently
  after the shared contracts exist. Later decisions concern Docker daemon identity,
  host IPC and shared overlay preparation for system apps.

- November 16: #442 is ready after scoped checks and parent review; its lease is
  released. No PR has been merged.

- November 16: published #443 for discovery. #444 is ready and its lease is
  released. Discovery remains draft during shutdown-fix review.
- November 16: the owner approved socket-only daemon checks and catalog cleanup,
  required repeatable cleanup, and directed reuse of existing application slices.
  Started the routing prerequisite and a narrow Supervisor removal retry fix.

- November 16: #443 and #445 are ready and their leases released. Parent reviewed
  #446, including its native routing contract; it is ready and its lease released.
  The plan update and Prepare/Review audit continue.
- November 16: the owner approved application execution in Module projects and
  read-only native defaults through the existing locked `toml` crate.
- November 16: updated #264 at `8cc2ec65` for approved scope and milestones 1/2.
  Prepare/Review audit found that native dry-run display is lossy and configured
  fragments can reference CLI-derived environment values. Interpretation must
  preserve this uncertainty; the detailed contract remains under discussion.
- November 16: the existing shared-memory contract resolves host IPC: native host
  IPC requires explicit PASSTHROUGH; BLOCKED rejects it. Apply that rule to
  Holoscan without another configuration input or policy.
- November 16: #447 is ready at `41ba6f9a`, stacked on #446, after parent and
  independent review. It reads declared Module Docker inputs; effective native
  resolution and Prepare/Review remain later work. The source-freshness question
  was resolved by the owner’s subsequent instruction to follow existing workflows.

- November 16: traced shared source capture, its callers and presentation for
  Compose/systemd; the existing regression tests confirm independent commit/status
  observations. Holoscan follows that behavior.
- November 16: #264 reached `bd6175aa` with source/IPC wording and the #447
  prerequisite. Published draft #448 at `d588fbc7` with shared source contracts and
  Build freshness assessment; native Prepare remains incomplete.
- November 16: the owner chose limited Review A. Resumed #448 implementation and
  the #264 scope update; removed the parser/interception proposal from active work.

- November 16: #264 is ready at `c223119e`; its author released the lease after
  parent review. Limited Review is the current implementation contract.
- November 16: a real systemd scope probe established direct-child reaping,
  descendant containment and repeated owned cleanup in the existing application
  hierarchy. A shared GetUnit/StopUnit disappearance race is being fixed separately.
- November 16: Build work started in its own lease. Rust owns the latest attempt
  record under admission; a failed or interrupted rebuild cannot expose an older
  successful Build as current. Container execution selection awaits approval.

- November 16: limited Review #448 is ready at `6b3b09b5` after parent review
  and scoped validation. The ready cleanup fix #455 is stacked on that final head
  at `e2ce7bab`; both code leases are released.
- November 16: published partial Build draft #458. Its latest-attempt storage is
  under parent and independent review; native Build invocation and Console delivery
  remain open. The application-run reuse map uses existing overlays, admission and
  lifecycle/finalization APIs.

- November 16: #264 now records milestones 1–3 ready at `96a19979`. Build draft
  #458 is aligned with the final stack at `752a2cb5`; parent and independent review
  resolved publication cancellation and durability-retry findings. Store checks
  are complete; Build invocation, output delivery and Console integration remain open.

- November 16: the owner approved the container execution selector. Resumed
  Build #458 and delegated the environment/Prepare prerequisite and existing-slice
  process helper to parallel agents with separate leases.

- November 16: #460 is ready at `c2da392d` after scoped parent review; its code
  lease is released. The plan correction uses a fresh lease. Build integration and
  process containment continue in their own checkouts.

- November 16: #264 at `b89cc5db` records the approved selector; its exact
  three-document correction was reviewed and the fresh plan lease released.
  Process containment is in draft #461. Application-run work uses a fresh lease
  and will stack on completed Build #458.

- November 16: #461 is ready at `182cdc30` and its lease is released. Parent
  review covered the single current-helper record and the failed-scope cleanup fix.
  Draft #458 now includes native Build invocation. GitHub stack links were rebuilt
  to insert #460/#461 below Build; PRs and branches were retained.

- November 16: reviewed #463 at `1edd3b72`. Compose keeps its existing rollback
  and readiness behavior; Holoscan can combine its folders with system-app binds
  in one filesystem plan. The author reports focused and real OverlayFS checks
  passing. Build output delivery and application-run integration continue.

- November 16: the owner waived ARM64 validation for this stack after the
  documented worker was unreachable. Local integration and real-container
  qualification remain required.

- November 16: published application Run #465 and system apps #464, appended in
  that order to stack #462. Build #458 now observes builder completion before
  cleanup; the real CLI rerun continues. The filesystem prerequisite #463 was
  restacked without changing its implementation.
- November 16: a real Compose probe reproduced late container creation after
  controller loss and replacement admission. Reusing the existing helper cgroup
  for finite system-app commands is awaiting the owner’s decision. Large
  application acceptance also awaits a storage/scope decision.

- November 16: Build #458 is ready at `b12d85b7`; its lease is released. The
  filesystem prerequisite #463 is aligned at `6d24703f`, also released. The Build
  author acquired a fresh recovery lease on system-app draft #464.
- November 16: updated #264 at `bebebee8` with implemented and demonstrated
  milestone items. Actual Build/Run contention, wrong-container rejection,
  system runtime and final acceptance remain open. Run #465 and system apps
  #464 remain drafts while their authors complete qualification.

## Left for the owner

- Remaining work: Run and system-app qualification, recovery and final integration.
- Pending choices: finite Compose helper containment and large-application acceptance storage/scope.
