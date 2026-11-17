import { presentFact, type ComparisonFact } from '../../presentation';
import { bytesLabel } from '../../../presentation';

const SHORT_ID_CHARS = 12, BYTES_PER_KIB = 1024;
const UNIT_SUFFIX = /_(bytes|seconds)$/;
/** Keep service/input names exact and humanize the run's stable fact names. */
export function runFactLabel(name: string): string {
  const service = /^service\.(.+)\.exit$/.exec(name);
  if (service) return service[1] + ' · Exit';
  if (/^(environment|mount|image|config|secret|system_app_profile|command|entrypoint|stop_signal|stop_grace_period)\./.test(name)) return name;
  const words = name.replace(UNIT_SUFFIX, '').replaceAll('_', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}
/** Format run values after exact comparison; leave unrecognized values intact. */
export function runFactValue(name: string, value: string): string {
  if (name === 'write_policy') return value === 'WRITE_POLICY_KEEP' ? 'Keep with the run' : value === 'WRITE_POLICY_DISCARD' ? 'Discard' : value === 'WRITE_POLICY_WRITE_BACK' || value === 'WRITE_POLICY_UNSPECIFIED' ? 'Write back' : 'Unknown';
  if (name === 'source_commit') return value.slice(0, SHORT_ID_CHARS);
  if (name.startsWith('image.')) return value.replace(/^(sha256:.{12}).*/, '$1');
  const number = Number(value);
  if (!value || !Number.isFinite(number)) return value;
  if (name.endsWith('_bytes')) return number < 0 || (number < BYTES_PER_KIB && !Number.isInteger(number)) ? `${number} B` : bytesLabel(number);
  if (name.endsWith('_seconds')) return `${number.toFixed(1)} s`;
  return value;
}
/** Preserve exact digits when rounding would make different measurements look equal. */
export function comparedRunValue(name: string, value: string, others: string[]) {
  const label = runFactValue(name, value), unit = name.endsWith('_bytes') ? 'B' : name.endsWith('_seconds') ? 's' : '';
  if (value && others.some(other => other !== value && runFactValue(name, other) === label)) {
    if (unit && Number.isFinite(Number(value))) return `${value} ${unit}`;
    if (name === 'source_commit' || name.startsWith('image.')) return value;
  }
  return label;
}
export const presentRunRows = (rows: ComparisonFact[]) => rows.map(row => presentFact(row, runFactLabel(row.name), (value, others) => comparedRunValue(row.name, value, others)));
