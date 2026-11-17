import { presentFact, type ComparisonFact, type ComparisonRowView } from '../../presentation';
import { wire } from '../../../api';

const serviceFact = /^service\.(.+)\.(cpu_seconds|cpu_mean_percent|cpu_observed_seconds|cpu_coverage)$/;
const labels: Record<string, string> = { cpu_seconds: 'Measured CPU time', cpu_mean_percent: 'Mean CPU · core %',
  cpu_observed_seconds: 'Measured duration', cpu_coverage: 'CPU coverage' };
/** Round an exact percent ratio to one decimal place, half up, without Number precision loss. */
const percent = (value: string) => {
  const ratio = /^(\d+)\/(\d+)$/.exec(value), denominator = BigInt(ratio?.[2] ?? 0);
  if (!ratio || !denominator) return value;
  const tenths = (BigInt(ratio[1]) * 10n + denominator / 2n) / denominator;
  return String(tenths / 10n) + '.' + String(tenths % 10n);
};
/** CPU labels and units are browser view state; raw exact values and evidence stay unchanged. */
export function presentCpuRows(rows: ComparisonFact[]): ComparisonRowView[] {
  return rows.map(row => {
    const match = serviceFact.exec(row.name), metric = match?.[2];
    const shown = (value: string, values: string[]) => {
      if (row.name === 'cpu_capture') return value ? 'Unavailable: ' + value : 'Recorded';
      if (!value || value === 'Unavailable') return value;
      if (metric?.endsWith('_seconds')) return value + ' s';
      if (metric === 'cpu_mean_percent') {
        const rounded = percent(value);
        const collision = row.result === wire.ComparisonResult.COMPARISON_RESULT_CHANGED && values.some(other => other !== value && percent(other) === rounded);
        return (collision ? value + ' ' : rounded) + '%';
      }
      return value;
    };
    const presented = presentFact(row,
      match ? match[1] + ' · ' + labels[metric!] : row.name === 'cpu_capture' ? 'CPU recording' : row.name,
      shown, row.name === 'cpu_capture' || metric === 'cpu_coverage' || metric === 'cpu_observed_seconds');
    if (metric === 'cpu_coverage') for (const cell of presented.cells) if (cell.values.includes(cell.evidence)) cell.evidence = '';
    return presented;
  });
}
