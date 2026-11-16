import type { wire } from '../native.js';
export type AnalysisRun = typeof wire.AnalysisRun.prototype;

type Row = typeof wire.ComparisonFact.prototype;
/** One module owns its per-run analyzer and pairwise comparer; analysis values stay private to it. */
export interface AnalysisModule<Analysis> {
  readonly id: string;
  readonly label: string;
  /** A/B invocations may overlap; each settles after its owned resources close. */
  analyze(run: Readonly<AnalysisRun>, signal: AbortSignal): Promise<Analysis>;
  compare(left: Analysis, right: Analysis): Row[];
}
