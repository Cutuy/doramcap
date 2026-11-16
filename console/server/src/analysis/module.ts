import { analysis, wire } from '../native.js';
import { boundRows } from '../records/compare.js';
export type AnalysisRun = typeof analysis.AnalysisRun.prototype;

type Row = typeof wire.ComparisonRow.prototype;
/** One module owns its per-run analyzer and pairwise comparer; analysis values stay private to it. */
export interface AnalysisModule<Analysis> {
  readonly id: string;
  readonly label: string;
  analyze(run: Readonly<AnalysisRun>, signal: AbortSignal): Promise<Analysis>;
  compare(left: Analysis, right: Analysis): Row[];
}
/** Bind one module and pair to a task; completion follows the analyzer's awaited cleanup. */
export function createAnalysisTask<Analysis>(module: AnalysisModule<Analysis>) {
  return {
    id: module.id, label: module.label,
    async run(left: AnalysisRun, right: AnalysisRun, signal: AbortSignal) {
      signal.throwIfAborted();
      const a = await module.analyze(left, signal);
      signal.throwIfAborted();
      const b = await module.analyze(right, signal);
      signal.throwIfAborted();
      const result = boundRows(module.compare(a, b));
      signal.throwIfAborted();
      return wire.AnalysisResponse.create({ moduleId: module.id, label: module.label, ...result });
    },
  };
}
