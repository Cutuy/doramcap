import { presentFact, type ComparisonFact } from '../../presentation';
import { bytesLabel } from '../../../presentation';

const ACRONYMS = /\b(cpu|gpu|numa|smt|rt|id|cpus)\b/g;
const UNIT_SUFFIX = /_(bytes|seconds|celsius|us)$/;
const BYTES_PER_KIB = 1024, SHORT_ID_CHARS = 12;
const SECONDS_PER_HOUR = 3600, SECONDS_PER_DAY = 86400;
/** Machine labels preserve shared-memory paths and standard acronyms. */
export function machineFactLabel(name: string): string {
  if (name === 'host_shm') return 'Host shared memory';
  if (name.startsWith('host_shm.')) return name.slice('host_shm.'.length);
  const words = name.replace(UNIT_SUFFIX, '').replace(/^docker\./, 'docker ').replaceAll('_', ' ').replace(ACRONYMS, word => word.toUpperCase());
  return words.charAt(0).toUpperCase() + words.slice(1);
}
/** Display units and shortened identifiers are separate from measured values. */
export function machineFactValue(name: string, value: string): string {
  if (name === 'machine_fingerprint') return value.slice(0, SHORT_ID_CHARS);
  const number = Number(value);
  if (!value || !Number.isFinite(number)) return value;
  if (name.endsWith('_bytes')) return number < 0 || (number < BYTES_PER_KIB && !Number.isInteger(number)) ? `${number} B` : bytesLabel(number);
  if (name.endsWith('_celsius')) return `${number} °C`;
  if (name === 'uptime_seconds') return number >= SECONDS_PER_DAY ? `${(number / SECONDS_PER_DAY).toFixed(1)} days` : `${(number / SECONDS_PER_HOUR).toFixed(1)} hours`;
  if (name.endsWith('_seconds')) return `${number.toFixed(1)} s`;
  if (name.endsWith('_us')) return `${number} µs`;
  return value;
}
/** Show native units when different measurements would share the same rounded value. */
export function comparedMachineValue(name: string, value: string, others: string[]) {
  const label = machineFactValue(name, value), unit = name.endsWith('_bytes') ? 'B' : name.endsWith('_seconds') ? 's' : '';
  return unit && value && Number.isFinite(Number(value)) && others.some(other => other !== value && machineFactValue(name, other) === label) ? `${value} ${unit}` : label;
}
export const presentMachineRows = (rows: ComparisonFact[]) => rows.map(row => presentFact(row, machineFactLabel(row.name), (value, others) => comparedMachineValue(row.name, value, others), row.name === 'machine_capture_error'));
