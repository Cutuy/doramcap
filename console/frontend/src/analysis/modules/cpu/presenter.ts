import type { ComparisonFact, ComparisonRowView } from '../../presentation';
import { wire } from '../../../api';

const serviceFact = /^service\.(.+)\.(cpu_seconds|cpu_mean_percent|cpu_observed_seconds|cpu_coverage)$/;
const labels: Record<string, string> = { cpu_seconds: 'Measured CPU time', cpu_mean_percent: 'Mean CPU · core %',
  cpu_observed_seconds: 'Measured duration', cpu_coverage: 'CPU coverage' };
const percent = (value: string) => {
  const ratio = /^(\d+)\/(\d+)$/.exec(value);
  return ratio && BigInt(ratio[2]) > 0n ? String(Number(ratio[1]) / Number(ratio[2])) : value;
};
/** CPU labels and units are browser view state; raw exact values and evidence stay unchanged. */
export function presentCpuRows(rows: ComparisonFact[]): ComparisonRowView[] {
  return rows.map(row => {
    const match = serviceFact.exec(row.name), metric = match?.[2], values = row.cells.flatMap(cell => cell.values);
    const hidden = row.cells.some(cell => cell.withheld);
    const shown = (value: string) => {
      if (row.name === 'cpu_capture') return value ? 'Unavailable: ' + value : 'Recorded';
      if (!value || value === 'Unavailable') return value;
      if (metric?.endsWith('_seconds')) return value + ' s';
      if (metric === 'cpu_mean_percent') {
        const rounded = percent(value);
        const collision = row.result === wire.ComparisonResult.COMPARISON_CHANGED && values.some(other => other !== value && percent(other) === rounded);
        return (collision ? value + ' ' : rounded) + '%';
      }
      return value;
    };
    return { group: row.group, name: row.name, result: row.result,
      label: match ? match[1] + ' · ' + labels[metric!] : row.name === 'cpu_capture' ? 'CPU recording' : row.name,
      hidden, alwaysVisible: row.name === 'cpu_capture' || metric === 'cpu_coverage' || metric === 'cpu_observed_seconds',
      cells: row.cells.map(cell => ({ values: hidden ? [] : cell.values.map(shown), evidence: hidden ? '' : cell.evidence })),
    };
  });
}
