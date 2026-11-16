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
| 1 | Lightweight real CLI, container and overlay probes | [#444](https://github.com/Cutuy/doramcap/pull/444), ready |
| 2 | Crate, protobuf contract and discovery | [#443](https://github.com/Cutuy/doramcap/pull/443), ready |
| 3 | Prepare and Review | Native Module defaults in progress |
| 4 | Build request | Pending |
| 5 | Application run | Pending |
| 6 | System apps | Pending |
| 7 | Recovery and acceptance | Pending |

## Agents

| Agent | Model | Assignment |
| --- | --- | --- |
| `holoscan_probe_audit` | GPT-6.1 Sol, xhigh | Milestone 1 probes and native Docker routing |
| `holoscan_contract_audit` | GPT-6.1 Sol, xhigh | Shared completion, cleanup retry and plan updates |
| `holoscan_discovery` | GPT-6.1 Sol, xhigh | Milestone 2 discovery and Prepare/Review audit |

## Decisions

The agreed decisions in #264 remain the baseline. The owner approved the
changes below during implementation.

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
per-command routing wrappers are unsupported. Add no environment overrides.

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
not replay capture or write-back. The existing inherited admission lock's effect
on orphan CLI recovery remains to be resolved in Build/Run implementation.

### 7. Module application compatibility — approved

Context: CLI 5.0 uses JSON metadata for application and Module projects. Only
Module projects read optional `pyproject.toml` defaults from `[tool.holoscan]`.
Their Docker options are hidden from dry-run output and must be accounted for
when reviewing mounts and conflicts. Application YAML remains native.

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

## Left for the owner

- Pending question: allow Unknown source freshness with an explicit unverified result.
- Later decisions: partial native dry-run interpretation, private shared memory and Build crash admission.
- Remaining implementation: Prepare, Build, Run, system apps and acceptance.
