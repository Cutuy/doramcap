# Compare saved facts and recording analyses implementation

Created: 2026-11-15. [Architecture](111526-compare-service-outcomes.md).

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
binds that module to the runner. CPU analyzers run in request-owned worker threads inside the Console process.
The server sends immutable saved metadata; the worker opens original evidence
beneath that run directory using existing native capabilities. Cancellation sends
an abort message and awaits worker exit after reader cleanup. The production
build includes the worker entry point. This worker boundary was approved on
2026-11-15 after a large-packet decode blocked the HTTP event loop.

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
- [x] Extend protobuf rows with explicit presentation and evidence fields.
- [x] Audit current and saved comparison-row consumers.
- [x] Update materialized producers and shared rendering together.
- [x] Keep materialized comparison independent of recording analysis.
- [x] Preserve withholding, row limits, truncation and supported-result access.

**Validation**
- [x] Existing saved runs and row compatibility.
- [x] Review and machine-change rendering.
- [x] Exact comparison before formatting.
- [x] Row and value boundaries.
- [x] Sensitive-value withholding.

## Milestone 2 Module task and explicit invocation

**Work items**
- [x] Add the module contract and registration boundary.
- [x] Add the generic analysis request and completed-result response.
- [x] Bind one task to one module and run pair.
- [x] Propagate cancellation and await resource cleanup.
- [x] Add explicit invocation and independent section state.
- [x] Reject stale completions after selection changes.

**Validation**
- [x] Materialized rendering with stalled analytics.
- [x] Independent module completion and retry.
- [x] Cancellation before and during each stage.
- [x] Navigation and selection replacement.
- [x] Reader cleanup on success, failure and cancellation.

## Milestone 3 CPU module adoption

**Work items**
- [x] Replace CPU comparison facts with a private per-run analysis result.
- [x] Move attribution, exact measurements, formatting and coverage into the module.
- [x] Keep original placement evidence and incremental rotation traversal.
- [x] Preserve measured zero, partial evidence and unavailable attribution.
- [x] Remove CPU fields and key checks from shared comparison, requests and rendering.
- [x] Replace the earlier CPU-only design text with this architecture.

**Validation**
- [x] Index-free recordings and consecutive rotations.
- [x] Session, identity, reset and counter gaps.
- [x] Shared and unsupported service attribution.
- [x] Missing, unfinished and malformed sources.
- [x] Per-side evidence and measured zero.
- [x] Generic rendering of CPU rows.

## Milestone 4 Responsiveness and integration

**Work items**
- [x] Measure materialized response and rendering during large recording analysis.
- [x] Measure cancellation latency, resource release and memory growth.
- [x] Exercise independent module tasks with controlled completion and failure.
- [x] Finish focused server, frontend, contract and packaged browser checks.

**Validation**
- [x] Large-recording request responsiveness.
- [x] Large-recording cancellation.
- [x] Concurrent task resource ownership.
- [x] Materialized-first browser rendering.
- [x] Focused suites and production builds.
- [x] x86_64 and ARM64 Node integration.

The large-packet acceptance check uses the production server and worker with an
index-free recording containing four 500,000-record packets. It measures concurrent
materialized HTTP delivery, event-loop delay, cancellation, descriptor closure and
process memory. Build the server, then run `tsx tests/analysis-responsiveness.ts`
from `console/server`. The packaged browser check is `tsx tests/analysis.browser.ts
PLAYWRIGHT_MODULE CHROMIUM_EXECUTABLE OUTPUT_DIRECTORY` after both production builds.
