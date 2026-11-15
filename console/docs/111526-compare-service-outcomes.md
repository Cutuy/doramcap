![Compare architecture](111526-compare-service-outcomes.svg)

# Compare saved facts and recording analyses

Draft. Created: 2026-11-15. [Implementation](111526-compare-service-outcomes-impl.md).

Compare has two layers. The materialized layer compares saved launch configuration,
machine snapshots and outcomes, including service exits. Its response renders first,
as soon as those facts are compared. The analytics layer reads recordings after
the user selects
**Run analysis**. Each analysis adds its own section when its task completes.

## Components

| Component | Responsibility |
| --- | --- |
| Materialized comparer | Compare saved facts and prepare display rows. |
| Analysis module | Analyze each selected run, then compare those domain results and prepare display rows. |
| Task runner | Invoke one module for the selected pair; own cancellation and task completion. |
| Shared row renderer | Display supplied labels, values, comparison outcomes and evidence. |

An analysis module owns its analyzer and comparer. CPU is the first module. Its
measurement types, attribution rules, units, precision and coverage stay together.
The shared runner and renderer consume a common contract. Module registration
supplies the module identity and label used by the generic request and UI.

![Analysis module](111526-analysis-module.svg)

## Delivery and lifetime

The materialized response and each analysis response have independent state. An
analysis shows **Running** until its completed rows or task error arrive. Completion
and retry are independent for each module. Materialized facts stay visible during
analysis, failure, cancellation and retry.

Each Analyze+Compare invocation is a self-contained task in the Console process.
It binds one module and the two selected runs. Its request signal reaches recording
reads and computation checkpoints. The task owns its readers and in-flight work
through verified cleanup. Leaving Compare or changing the selection cancels its
outstanding tasks; stale completions cannot replace the current selection.

Rows are returned after comparison completes. Task completion and cancellation use
the request lifecycle. Returning to Compare permits another explicit invocation.
Completed rows belong to the active view; saved runs and recording formats retain
their existing authority. Analysis results are recomputed on a fresh invocation.

## Results and evidence

Both layers evolve the existing comparison row contract. Comparers supply stable
keys, labels, display values, comparison outcomes and per-side evidence. Exact
values are compared before formatting. Shared rendering uses these fields.

Evidence describes the scope and quality of each side's observations. Partial and
unavailable evidence stays explicit beside the affected measurement. A task error
belongs to its module section; useful materialized facts and other completed
analysis sections remain available. Cancellation ends the task and releases its
resources.

## CPU adoption

CPU analysis reads the original saved placement evidence and completed MCAP
recordings. It retains valid interval reduction, session and identity boundaries,
rotation traversal and explicit unavailable attribution. CPU comparison prepares
service measurements and coverage for the shared renderer. CPU coverage describes
valid recorded intervals.

## Resource boundaries

The TypeScript server hosts the tasks. Existing native file capabilities own safe
file opening; CPU retains the official sequential MCAP reader. Recording traversal
is incremental. Memory scales with active readers, the largest MCAP record,
dictionaries, counter endpoints, declared services and displayed rows.

Large-recording checks must establish cooperative cancellation and responsiveness
of concurrent materialized requests. Row and value limits must expose truncation
and preserve access to supported results. Processing and storage formats retain
the existing telemetry and saved-input limits.
