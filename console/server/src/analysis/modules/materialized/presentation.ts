import { bytesLabel } from '../../../generated/presentation.js';

const ACRONYMS = /\b(cpu|gpu|numa|smt|rt|id|cpus)\b/g;
const UNIT_SUFFIX = /_(bytes|seconds|celsius|us)$/;
const BYTES_PER_KIB = 1024;
const SHORT_ID_CHARS = 12;
const SECONDS_PER_HOUR = 3600, SECONDS_PER_DAY = 86400;
/** A readable name for a compared fact. Service and input names stay exact. */
export function factLabel(name: string): string {
  const service = /^service\.(.+)\.exit$/.exec(name);
  if (service) return service[1] + ' · Exit';
  if (name === 'host_shm') return 'Host shared memory';
  if (name.startsWith('host_shm.')) return name.slice('host_shm.'.length);
  if (/^(environment|mount|image|config|secret|system_app_profile|command|entrypoint|stop_signal|stop_grace_period)\./.test(name)) return name;
  const words = name.replace(UNIT_SUFFIX, '').replace(/^docker\./, 'docker ').replaceAll('_', ' ').replace(ACRONYMS, word => word.toUpperCase());
  return words[0].toUpperCase() + words.slice(1);
}
/** A readable value for a compared fact, chosen by the unit in its name. */
export function factValue(name: string, value: string): string {
  if (name === 'write_policy') return value === 'WRITE_POLICY_KEEP' ? 'Keep with the run' : value === 'WRITE_POLICY_DISCARD' ? 'Discard' : value === 'WRITE_POLICY_WRITE_BACK' || value === 'WRITE_POLICY_UNSPECIFIED' ? 'Write back' : 'Unknown';
  if (name === 'source_commit') return value.slice(0, SHORT_ID_CHARS);
  if (name.startsWith('image.')) return value.replace(/^(sha256:.{12}).*/, '$1');
  const number = Number(value);
  if (!value || !Number.isFinite(number)) return value;
  if (name.endsWith('_bytes')) return number < 0 || (number < BYTES_PER_KIB && !Number.isInteger(number)) ? `${number} B` : bytesLabel(number);
  if (name.endsWith('_celsius')) return `${number} °C`;
  if (name === 'uptime_seconds') return number >= SECONDS_PER_DAY ? `${(number / SECONDS_PER_DAY).toFixed(1)} days` : `${(number / SECONDS_PER_HOUR).toFixed(1)} hours`;
  if (name.endsWith('_seconds')) return `${number.toFixed(1)} s`;
  if (name.endsWith('_us')) return `${number} µs`;
  return value;
}
/** Show exact native units when different numeric facts share a rounded label. */
export function comparedFactValue(name: string, value: string, others: string[]): string {
  const label = factValue(name, value);
  const unit = name.endsWith('_bytes') ? 'B' : name.endsWith('_seconds') ? 's' : '';
  if (unit && value && Number.isFinite(Number(value)) && others.some(other => other !== value && factValue(name, other) === label)) return `${value} ${unit}`;
  return label;
}
