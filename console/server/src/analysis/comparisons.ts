import { wire } from '../native.js';
import type { doramcap } from '../generated/console.js';
import { LIMITS } from '../generated/transport.js';
import { boundRows } from './rows.js';

type Fact = doramcap.console.v1.ComparisonFact.$Properties;
const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const captureError = (row: Fact) => row.name === 'machine_capture_error';
/** The History workflow keeps all run and machine results within one response budget. */
export function historyRows(run: Fact[], machine: Fact[]) {
  return boundRows([...machine.filter(captureError), ...run, ...machine.filter(row => !captureError(row))]);
}
/** Review compares setup/configuration and shows only the current machine condition. */
export function reviewRows(hasBaseline: boolean, run: Fact[], machine: Fact[]) {
  const condition = machine.filter(row => row.group === Group.COMPARISON_CONDITION && row.cells?.[1]?.values?.some(Boolean))
    .map(row => wire.ComparisonFact.create({ ...row, result: Result.COMPARISON_UNAVAILABLE,
      cells: [wire.ComparisonFactCell.create({ values: [''] }), wire.ComparisonFactCell.create(row.cells?.[1] ?? {})],
    }));
  return boundRows([
    ...condition.filter(captureError),
    ...(hasBaseline ? run.filter(row => row.group === Group.COMPARISON_SETUP) : []),
    ...machine.filter(row => row.group === Group.COMPARISON_CONFIGURATION),
    ...condition.filter(row => !captureError(row)),
  ]);
}
/** Build a visible unavailable result when live machine comparison cannot be represented. */
export function machineFailure(message: string) {
  return [wire.ComparisonFact.create({ group: Group.COMPARISON_CONDITION, name: 'machine_capture_error', result: Result.COMPARISON_UNAVAILABLE,
    cells: [{ values: [] }, { values: [message] }],
  })];
}
/** Live changes apply their difference selection before the row budget. */
export function machineChangeRows(rows: Fact[]) {
  const result = boundRows(rows.filter(row => row.result === Result.COMPARISON_CHANGED || captureError(row)));
  return result.truncated ? machineFailure('Machine comparison is too large to display. Changes are unavailable.') : result.rows;
}
/** Preserve Review when its optional comparison exceeds the complete encoded envelope. */
export function boundReviewComparison(review: typeof wire.Review.prototype) {
  if (review.comparison && wire.Review.encode(review).finish().length > LIMITS.frameBytes) {
    review.comparison.rows = []; review.comparison.truncated = true;
  }
}
/** Preserve the run result when live comparison exceeds the complete encoded envelope. */
export function boundMachineChanges(status: typeof wire.Status.prototype) {
  if (status.machineChanges.length && wire.LiveMessage.encode({ status: wire.Status.encode(status).finish() }).finish().length > LIMITS.frameBytes)
    status.machineChanges = machineFailure('Machine comparison is too large to display. Changes are unavailable.');
}
