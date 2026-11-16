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
  compare(left: Analysis, right: Analysis): ComparisonRow[];
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
    doramcap.console.v1.HistoryDetail detail = 2;
    doramcap.console.v1.Status current = 5;
  }
  doramcap.console.execution.v1.RunCapture capture = 3;
  repeated AnalysisArtifact artifacts = 4;
}
```

`detail` supplies saved History; `current` supplies resolved Review settings.
Capture-only inputs supply live machine observations. `directory` is a
server-authorized absolute evidence directory. Recording paths in
`detail.status.recordings` and artifact paths are relative to that directory.
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

The built-in `materialized` module owns saved facts, Review and live machine
comparison, including normalization, masking and presentation. Records retain
original evidence. Common row bounds retain producer-marked context before
other differences and matches. The browser displays supplied labels and values;
legacy rows use raw fields. The demo reuses module presentation when producing rows.
The final shutdown observation completes through the runner before it closes.

## Milestone 1: Design PR

**Work items**
- [x] Retain per-run analysis and module comparer ownership.
- [x] Simplify generic architecture and define explicit per-run inputs.
- [x] Place future Rust analysis at the per-run boundary.
- [x] Return module-owned protobuf analysis values after cleanup.

**Validation**
- [x] Architecture and contract consistency.
- [x] Documentation links and SVG rendering.

## Milestone 2: Framework and default materialized module

**Work items**
- [x] Add generated per-run inputs and common presentation fields.
- [x] Implement registry, runner and cancellation ownership.
- [x] Register the default materialized module.
- [x] Route History, Review and live changes through the shared runner.
- [x] Move materialized presentation out of the renderer.

**Validation**
- [x] Default module and unknown identity.
- [x] Original evidence and module normalization.
- [ ] Service, History and disconnect integration.
- [x] Independent tasks and cleanup.
- [x] Existing comparison and saved-row compatibility.

## Milestone 3: First recording module PR

**Work items**
- [ ] Add a dedicated CPU module directory.
- [ ] Separate per-run analyzer, private measurements and comparer.
- [ ] Preserve original evidence, rotation traversal and coverage.

**Validation**
- [ ] Recording and attribution behavior.
- [ ] Missing and partial evidence.
- [ ] Worker responsiveness and cleanup.

## Milestone 4: Server selection PR

**Work items**
- [ ] Advertise registered analyzers and validate selected identity.
- [ ] Construct explicit run inputs in the existing service lookup.
- [ ] Bind HTTP cancellation and shutdown to task cleanup.

**Validation**
- [ ] Selected analyzer execution.
- [ ] Invalid selections and missing runs.
- [ ] Disconnect, shutdown and concurrent requests.

## Milestone 5: Frontend PR

**Work items**
- [ ] Add analyzer selection to Compare.
- [ ] Render independent running, completed and failed sections.
- [ ] Support cancel, retry and stale-response rejection.
- [ ] Update demos and shared presentation consumers.

**Validation**
- [ ] Saved facts during stalled analysis.
- [ ] Selection, cancellation and retry.
- [ ] Review and machine-change rendering.
- [ ] Packaged browser behavior.
