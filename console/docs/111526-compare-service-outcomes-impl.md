# Compare analysis implementation

Created: 2026-11-15. [Architecture](111526-compare-service-outcomes.md).

Applicable rules: I-03/04/05/06/07/09; D-01/02/03/05/06/07/08/10/11/12/13/15.

## Module and per-run input

Keep the existing analyzer/comparer ownership:

```ts
interface AnalysisModule<Analysis> {
  readonly id: string;
  readonly label: string;
  analyze(run: AnalysisRun, signal: AbortSignal): Promise<Analysis>;
  compare(left: Analysis, right: Analysis): ComparisonFact[];
}
```

The approved input boundary is an explicit per-run protobuf. Reuse generated
saved metadata and capture types in `console.proto`; keep original evidence
references separate from comparison-normalized configuration. `analysis.proto`
owns shared comparison types and the module API. The existing service lookup
constructs each run input.

```proto
message AnalysisArtifact {
  string name = 1;
  string path = 2;
}
message AnalysisRun {
  string directory = 1;
  oneof snapshot {
    doramcap.console.v1.HistoryDetail snapshot_history = 2;
    doramcap.console.v1.Status snapshot_current = 5;
  }
  doramcap.console.execution.v1.RunCapture capture = 3;
  repeated AnalysisArtifact artifacts = 4;
}
```

`snapshot_history` supplies saved History; `snapshot_current` supplies resolved Review settings.
Capture-only inputs supply live machine observations. `directory` is a
server-authorized absolute evidence directory. Recording paths in
`snapshot_history.status.recordings` and artifact paths are relative to that directory.
The saved configuration and placement references preserve the original files.
Missing files remain explicit analyzer evidence. Browser input supplies run IDs;
it never supplies host paths. Native safe-open enforces the directory boundary.

The module adapter encodes one AnalysisRun for native execution. The Rust input
has no A/B selection or comparison policy. Native work must remain off the HTTP
event loop; cancellation is per invocation, and completion awaits reader cleanup.
Native analysis returns the module-owned Analysis protobuf after cleanup. The
module adapter decodes it for that module's comparer; shared orchestration does
not interpret private measurements.
The registry's private task adapter analyzes both runs concurrently and invokes
the module's comparer. Failure cancels the sibling; completion awaits both cleanups.

The framework PR introduces an empty production registry. Its follow-up registers
`run` for setup and outcomes and `machine` for configuration, condition and capture
errors. Console selects History, Review and live result groups before bounding
them. Review displays current condition; live changes retain differences and errors.

Both modules return `ComparisonFact` values, evidence and explicit withholding.
Module-owned TypeScript presenters supply labels, formatting and visibility.
Setup records retain original evidence; only the run module normalizes comparison
paths. The file-change report API keeps its existing owner.

The runner returns complete internal results. Console selects workflow results,
then applies `boundRows` and `comparisonResponse` at the transport boundary.
The row limit is 2,000, with nonmatching facts before matches and module order
preserved within each set. Modules order required error evidence first. A response
beyond the shared byte budget fails explicitly.

## Milestone 1: Design PR

**Work items**
- [x] Retain per-run analysis and module comparer ownership.
- [x] Simplify generic architecture and define explicit per-run inputs.
- [x] Place future Rust analysis at the per-run boundary.
- [x] Return module-owned protobuf analysis values after cleanup.

**Validation**
- [x] Architecture and contract consistency.
- [x] Documentation links and SVG rendering.

## Milestone 2: Framework PR

**Work items**
- [x] Add explicit per-run inputs and comparison facts.
- [x] Implement the registry, runner and cancellation ownership.
- [x] Integrate an empty production registry.
- [x] Keep existing comparison paths unchanged.

**Validation**
- [x] Empty registry and unknown module.
- [x] Parallel sides, failure and cleanup.
- [x] Snapshot wire compatibility.
- [x] Row and transport limits.
- [x] Existing comparisons and service shutdown.

## Milestone 3: Existing comparison migration

**Work items**
- [x] Add separate run and machine modules.
- [x] Route History, Review and live comparisons through the runner.
- [x] Move presentation into module-owned TS presenters.
- [x] Preserve masking, original evidence and file-change ownership.

**Validation**
- [x] History, Review and live behavior.
- [x] Missing evidence and limits.
- [x] Exact values and withholding.
- [x] Browser rendering and shutdown.

## Milestone 4: CPU recording module

**Work items**
- [ ] Add the CPU module with private per-run measurements.
- [ ] Preserve original recordings and coverage.
- [ ] Supply the CPU TypeScript presenter.

**Validation**
- [ ] Recording and attribution behavior.
- [ ] Missing and partial evidence.
- [ ] Worker responsiveness and cleanup.

## Milestone 5: Server selection

**Work items**
- [ ] Advertise optional recording analyses.
- [ ] Run the selected module with authorized per-run inputs.
- [ ] Bind disconnect and shutdown to cleanup.

**Validation**
- [ ] Selected module execution.
- [ ] Invalid selection and missing runs.
- [ ] Disconnect, shutdown and concurrent requests.

## Milestone 6: Frontend controls

**Work items**
- [ ] Add analysis selection with independent result sections.
- [ ] Support cancellation, retry and stale-response rejection.
- [ ] Use the shared renderer and module TS presenters.
- [ ] Update demo consumers.

**Validation**
- [ ] Saved comparisons during stalled analysis.
- [ ] Selection, cancellation and retry.
- [ ] History, Review and machine rendering.
- [ ] Packaged browser behavior.
