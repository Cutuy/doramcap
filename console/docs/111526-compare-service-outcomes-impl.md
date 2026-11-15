# Compare saved facts and recording analyses implementation

Draft. Created: 2026-11-15. [Architecture](111526-compare-service-outcomes.md).

Redesign of [PR #404](https://github.com/Cutuy/doramcap/pull/404).

Applicable rules: I-03/04/05/06/07/09; D-01/02/03/04/05/06/07/08/10/11/12/13/15.

## Module and task contracts

Each registered module has an identity and label and owns these operations:

```ts
interface AnalysisModule<Analysis> {
  readonly id: string;
  readonly label: string;
  analyze(run: AnalysisRun, signal: AbortSignal): Promise<Analysis>;
  compare(left: Analysis, right: Analysis): ComparisonRow[];
}
```

Analysis is private to the module. Its input contains immutable saved run metadata
and recording access scoped to that run. The runner resolves the selected module
and run pair, invokes both per-run analyses, calls that module's comparer, and
returns its completed rows. The runner checks cancellation around each stage;
long-running analyzer work checks it during traversal and computation. Request
completion follows cleanup of every task-owned reader and operation.

Shared orchestration and row rendering use module identity, labels and common
results. CPU-specific contracts belong in the CPU module. The registration site
binds that module to the runner. The initial module remains inside the Console
server and uses its existing native capabilities.

## Protocol and rendering

Keep materialized comparison on its own fast request. Introduce a generic analysis
request identifying a registered module and the selected run pair. Validate the
module, selection and saved run references before starting recording reads. Return
one completed response per module; request cancellation propagates to its task.

Evolve the existing protobuf comparison rows additively: retain field numbers and
raw-value semantics, and add explicit labels, display values and per-cell evidence.
New enum values use the complete enum-name prefix. Hidden values remain withheld
from both raw and display fields. Audit saved rows and current callers before
changing producer or renderer assumptions.

Update materialized row producers and the analysis comparer together. Compare,
Review and machine-change views consume the explicit presentation fields through
the shared renderer. Source-specific formatting and evidence wording stay with
the comparer that understands them.

Keep materialized state separate from a module's running, completed and failed
states. An explicit user action starts each task. Render each section when its
own response arrives. Selection changes and navigation abort outstanding work;
late responses cannot overwrite another selection. Per-side missing evidence
stays a result the comparer can display alongside available observations.

## Milestone 1 Shared rows and materialized delivery

**Work items**
- [ ] Extend protobuf rows with explicit presentation and evidence fields.
- [ ] Audit current and saved comparison-row consumers.
- [ ] Update materialized producers and shared rendering together.
- [ ] Keep materialized comparison independent of recording analysis.
- [ ] Preserve withholding, row limits, truncation and supported-result access.

**Validation**
- [ ] Existing saved runs and row compatibility.
- [ ] Review and machine-change rendering.
- [ ] Exact comparison before formatting.
- [ ] Row and value boundaries.
- [ ] Sensitive-value withholding.

## Milestone 2 Module task and explicit invocation

**Work items**
- [ ] Add the module contract and registration boundary.
- [ ] Add the generic analysis request and completed-result response.
- [ ] Bind one task to one module and run pair.
- [ ] Propagate cancellation and await resource cleanup.
- [ ] Add explicit invocation and independent section state.
- [ ] Reject stale completions after selection changes.

**Validation**
- [ ] Materialized rendering with stalled analytics.
- [ ] Independent module completion and retry.
- [ ] Cancellation before and during each stage.
- [ ] Navigation and selection replacement.
- [ ] Reader cleanup on success, failure and cancellation.

## Milestone 3 CPU module adoption

**Work items**
- [ ] Replace CPU comparison facts with a private per-run analysis result.
- [ ] Move attribution, exact measurements, formatting and coverage into the module.
- [ ] Keep original placement evidence and incremental rotation traversal.
- [ ] Preserve measured zero, partial evidence and unavailable attribution.
- [ ] Remove CPU fields and key checks from shared comparison, requests and rendering.
- [ ] Replace the earlier CPU-only design text with this architecture.

**Validation**
- [ ] Index-free recordings and consecutive rotations.
- [ ] Session, identity, reset and counter gaps.
- [ ] Shared and unsupported service attribution.
- [ ] Missing, unfinished and malformed sources.
- [ ] Per-side evidence and measured zero.
- [ ] Generic rendering of CPU rows.

## Milestone 4 Responsiveness and integration

**Work items**
- [ ] Measure materialized response and rendering during large recording analysis.
- [ ] Measure cancellation latency, resource release and memory growth.
- [ ] Exercise independent module tasks with controlled completion and failure.
- [ ] Finish focused server, frontend, contract and packaged browser checks.

**Validation**
- [ ] Large-recording request responsiveness.
- [ ] Large-recording cancellation.
- [ ] Concurrent task resource ownership.
- [ ] Materialized-first browser rendering.
- [ ] Focused suites and production builds.
- [ ] x86_64 and ARM64 Node integration.
