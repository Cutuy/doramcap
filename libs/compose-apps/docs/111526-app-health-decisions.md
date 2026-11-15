# App health implementation log

Created: 2026-11-15. [Plan](111526-exits-from-docker-events-impl.md).

## Work

| Scope | Agent | Branch | PR |
| --- | --- | --- | --- |
| Plan | Owner | `codex/exec-free-compose-health` | [387](https://github.com/Cutuy/doramcap/pull/387) |
| Milestone 1: remove probes | `pr387_implementation` | `codex/pr387-m1-remove-probes` | [389](https://github.com/Cutuy/doramcap/pull/389) |
| Milestone 2: report health | Same implementation agent | `codex/pr387-m2-bridge-health` | [394](https://github.com/Cutuy/doramcap/pull/394) |
| Typing fix review record | `pr387_runtime_prerequisite` | Included in runtime facts | [397](https://github.com/Cutuy/doramcap/pull/397) |
| Live Docker state prerequisite | `pr387_runtime_prerequisite` | `codex/pr387-live-runtime-state` | [391](https://github.com/Cutuy/doramcap/pull/391) |
| Milestone 3: live panel | Same implementation agent | `codex/pr387-m3-live-health-panel` | [399](https://github.com/Cutuy/doramcap/pull/399) |
| Qualification fixtures | `pr387_runtime_prerequisite` | `codex/pr387-demo-qualification-inputs` | [403](https://github.com/Cutuy/doramcap/pull/403) |
| Coordination and review | Root | `codex/pr387-decision-log` | This PR |

Open stack: 387 → 389 → 394 → 391 → 399 → 403 → this coordination PR. Main is unchanged. PR 397 is the typing-fix review record, included in PR 391.

## Decisions

### 1. Watchdog self-health — approved

Context: the watchdog already includes its ingress and output diagnostics in fault publications.
Problem: a separate watchdog `health` output would add a feedback path.

```mermaid
flowchart LR
  A[App health] --> W[Watchdog]
  W -->|Faults and local diagnostics| C[Console]
  D[Docker events] -->|Watchdog process status| C
```

- Option: show the watchdog's existing local diagnostics as app health.
- Option: show process/container/service status, keeping diagnostics in the faults view.
- Alternative: add a separate watchdog health output.
- Choice: the owner approved existing behavior or process status, without a new feedback path. The panel uses runtime status; existing diagnostics stay in the faults view.
- Reason: preserves current ownership and avoids a second reporting path.

### 2. Bridge outcomes — owner-directed

Context: the bridge accepts payloads within its existing size limit and skips oversized payloads.
Problem: define health reporting without adding bridge-specific suppression policy.

```mermaid
flowchart LR
  P[Relay result] -->|Every success or failure| H[Existing health reporter]
  H -->|Scheduled publication| W[Watchdog]
```

- Proposed: report relay outcomes with a bridge-specific transition filter.
- Alternative: report initialization only.
- Choice: the owner directed the bridge to always report, without special cases. Each relay outcome uses the existing reporter; its ordinary scheduled drain publishes reports.
- Reason: follows existing reporting semantics and keeps policy out of the bridge.

### 3. Native watchdog display — owner-directed

Context: reporters assign fresh process-incarnation IDs; watchdog publications already contain source connectivity, component labels and condition state.
Problem: a forced join from source incarnations to selected app IDs would require additional attribution metadata or ambiguous label matching.

```mermaid
flowchart LR
  R[Reporter facts] --> W[Watchdog publications]
  W --> H[Reported health rows]
  D[Docker events] --> P[Process status rows]
```

- Proposed: preserve graph attribution in watchdog publications through another prerequisite.
- Alternative: match existing component labels to app IDs.
- Choice: the owner asked to show what the watchdog received and reported as-is. Show its native source/condition facts alongside Docker process status, preserving source identity without adding attribution metadata.
- Reason: keeps the native fault model and avoids an unnecessary join.

### 4. Buildable layers and review record — owner-delegated

Context: the runtime observation adds a generated protobuf union. Its typing fix initially sat above it, leaving intermediate Node checks failing. The owner approved repairing the order.
Problem: during the branch rewrite, GitHub automatically marked PR 397 merged into its existing runtime feature-branch target. Its comments and code remain; main was unchanged.

```mermaid
flowchart LR
  A[Bridge health] --> B[Runtime facts and stable frontend types] --> C[Native health display]
  R[Typing review 397] -. review record .-> B
```

- Recommended: retain PR 397 as the review record and include the fix in PR 391.
- Alternative: open a replacement typing prerequisite.
- Choice: the owner delegated the remaining structure decision. Keep the fix with runtime facts and retain the existing review record.
- Reason: runtime checks pass independently; the final implementation is preserved without duplicating the review unit.

## Events

- 2026-11-15: inspected plan `33d155fd` against main `9e18b12a` and reused the suggested partial milestone 1 commit.
- 2026-11-15: real Docker evidence established normal stop-before-die ordering; runtime observations retain unknown state until exit evidence arrives.
- 2026-11-15: isolated demo qualification gained its missing shared inputs and waits for output before checking HTTP pages. Fresh acceptance-image builds use an ephemeral host-network selection.
- 2026-11-15: local Rust/Node/MCAP checks, ARM64 worker 721, real Console and Dora/GPU gallery acceptance, and light/dark/narrow Chromium component checks passed.
- 2026-11-15: the approved restack preserves the qualified source. The runtime layer now includes its typing prerequisite and passes hosted Node checks on both architectures; final local checks also pass. The review record for the contained fix remains in PR 397.

## Probe CPU comparison

On 2026-11-15, ARM64 worker job 720 completed three alternating pairs of four idle containers. Each trial warmed up for 2 seconds, then sampled for 10 seconds. The same cached image ran `sleep` in each case; the baseline added `test -s /tmp/app.ready` every 200 ms. The test removed its own containers.

| Mean core CPU | With probes | Without probes |
| --- | ---: | ---: |
| Whole host | 77.5% | 3.4% |
| Container cgroups combined | 27.5% | 0.0% |

The worker has 12 ARM64 vCPUs. Whole-host values use `/proc/stat` deltas in core percent; container values use cgroup `usage_usec`. This isolates probe overhead; it is not an end-to-end measurement of the new health pipeline. Source and raw results remain in worker job directory `/home/doramcap-ci/jobs/pr387-probe-cpu-20261115`.

## Left for the owner

- Review the completed milestone stack before integration.
