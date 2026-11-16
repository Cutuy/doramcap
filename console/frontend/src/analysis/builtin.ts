import { wire } from '../protocol';
import { presentRunRows } from './modules/run/presenter';
import { presentMachineRows } from './modules/machine/presenter';
import type { ComparisonFact } from './presentation';

/** Console composes the two automatic modules into its existing comparison sections. */
export function presentBuiltinRows(rows: ComparisonFact[]) {
  return rows.map(row => row.group === wire.ComparisonGroup.COMPARISON_OUTCOME || row.group === wire.ComparisonGroup.COMPARISON_SETUP
    ? presentRunRows([row])[0] : presentMachineRows([row])[0]);
}
