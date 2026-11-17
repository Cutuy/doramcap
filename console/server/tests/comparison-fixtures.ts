import { execution, wire } from '../src/native.js';
import type { SetupRecord } from '../src/records/setup.js';
import { analyzeRunSnapshot, compareRunAnalyses, type SetupSide } from '../src/analysis/modules/run/module.js';
import { analyzeMachineSnapshot, compareMachineAnalyses } from '../src/analysis/modules/machine/module.js';
import { historyRows, reviewRows, machineChangeRows } from '../src/analysis/comparisons.js';

export type Side = { id: string; detail: typeof wire.HistoryDetail.prototype; capture: typeof execution.RunCapture.prototype; setup: SetupRecord | null };
const setupSide = (side: Side) => ({ ...side, profiles: side.detail.entry?.profiles ?? [], apps: wire.SystemAppSelection.create(side.detail.entry?.systemApps ?? {}), writePolicy: side.detail.status?.writePolicy });
export function compareRuns(a: Side, b: Side, labels = ['A', 'B']) {
  const result = historyRows(compareRunAnalyses(analyzeRunSnapshot(setupSide(a)), analyzeRunSnapshot(setupSide(b))), compareMachineAnalyses(analyzeMachineSnapshot(a.capture), analyzeMachineSnapshot(b.capture)));
  return wire.HistoryComparison.create({ ...result, sides: [a,b].map((side,index) => ({ label: labels[index], runs: [{ entry: side.detail.entry, captured: !!side.capture.before }] })) });
}
export function compareReview(baseline: Side | null, current: SetupSide) {
  const left = baseline ? analyzeRunSnapshot(setupSide(baseline)) : null;
  const result = reviewRows(!!baseline, left ? compareRunAnalyses(left, analyzeRunSnapshot(current)) : [], compareMachineAnalyses(analyzeMachineSnapshot(baseline?.capture ?? execution.RunCapture.create()), analyzeMachineSnapshot(current.capture)));
  return wire.ReviewComparison.create({ ...result, baseline: baseline?.detail.entry });
}
export function machineChanges(before: typeof execution.RunCapture.prototype.before, after: typeof execution.RunCapture.prototype.before, beforeError = '', afterError = '') {
  return machineChangeRows(compareMachineAnalyses(analyzeMachineSnapshot(execution.RunCapture.create({ before, beforeError })), analyzeMachineSnapshot(execution.RunCapture.create({ before: after, beforeError: afterError }))));
}
