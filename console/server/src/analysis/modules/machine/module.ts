import Long from 'long';
import { execution, systemConfig, wire } from '../../../native.js';
import type { AnalysisModule } from '../../module.js';
import { diffFacts, isSensitive, type Facts } from '../facts.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
type Capture = typeof execution.RunCapture.prototype;
const snake = (name: string) => name.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
/** Withhold kernel parameters by the same name rules as environment values. */
const maskBootFlags = (flags: string) =>
  flags.split(/\s+/).map((flag) => {
    const [name, value = ''] = flag.split(/=(.*)/s);
    return isSensitive(name, value) ? `${name}=(hidden)` : flag;
  }).join(' ');
const display = (name: string, value: unknown): string => {
  if (Array.isArray(value)) return value.map((item) => (typeof item === 'object' ? `${item.value}: ${item.cpus}` : String(item))).join(', ');
  return String(value);
};
/** Numeric zero, false and empty fields mean unset; string values retain their meaning. */
const isUnset = (value: unknown) => value == null || value === '' || value === 0 || (Long.isLong(value) && value.isZero()) || value === false || (Array.isArray(value) && !value.length);
/** Flatten decoded protobuf fields into named facts; unset fields have none. */
function flatten(object: Record<string, unknown>, prefix = '', facts: Facts = new Map()): Facts {
  for (const [key, value] of Object.entries(object)) {
    if (isUnset(value)) continue;
    const name = prefix + snake(key);
    if (value && typeof value === 'object' && !Array.isArray(value) && !Long.isLong(value)) flatten(value as Record<string, unknown>, `${name}.`, facts);
    else { const text = display(name, value); facts.set(name, { value: text, ...(name === 'boot_flags' ? { publicValue: maskBootFlags(text) } : {}) }); }
  }
  return facts;
}
const options = { enums: String, arrays: false } as const;
const { MachineConfiguration, MachineCondition } = systemConfig;
export const configurationFacts = (state: Capture['before']) =>
  state?.configuration ? flatten(MachineConfiguration.toObject(MachineConfiguration.fromObject(state.configuration), options)) : new Map() as Facts;
export function conditionFacts(state: Capture['before']): Facts {
  const facts = state?.condition ? flatten(MachineCondition.toObject(MachineCondition.fromObject(state.condition), options)) : new Map() as Facts;
  if (!state) return facts;
  const entries = state.hostShm ?? [];
  facts.set('host_shm', { value: `${entries.length} entries` });
  for (const entry of entries) {
    const { path, ...metadata } = systemConfig.FileMetadata.toObject(systemConfig.FileMetadata.fromObject(entry), { longs: String, defaults: true });
    facts.set(`host_shm.${path}`, { value: JSON.stringify(metadata) });
  }
  return facts;
}


/** Machine facts and observation failure from one capture. */
export type MachineAnalysis = { configuration: Facts; condition: Facts; error: string };
/** Extract a capture without choosing a History, Review or live presentation. */
export function analyzeMachineSnapshot(capture: Capture): MachineAnalysis {
  return { configuration: configurationFacts(capture.before), condition: conditionFacts(capture.before), error: capture.beforeError };
}
/** Capture errors precede normal facts so transport bounds preserve required evidence. */
export function compareMachineAnalyses(a: MachineAnalysis, b: MachineAnalysis) {
  const empty: Facts = new Map();
  const errors = (error: string): Facts => error ? new Map([['machine_capture_error', { value: error }]]) : empty;
  return [
    ...diffFacts(Group.COMPARISON_GROUP_CONDITION, errors(a.error), errors(b.error))
      .map(row => wire.ComparisonFact.create({ ...row, result: Result.COMPARISON_RESULT_UNAVAILABLE })),
    ...diffFacts(Group.COMPARISON_GROUP_CONFIGURATION, a.error ? empty : a.configuration, b.error ? empty : b.configuration),
    ...diffFacts(Group.COMPARISON_GROUP_CONDITION, a.error ? empty : a.condition, b.error ? empty : b.condition),
  ];
}
export const MACHINE_MODULE_ID = 'machine';
/** Compare machine configuration, condition and capture evidence independently of run metadata. */
export const machineModule: AnalysisModule<MachineAnalysis> = {
  id: MACHINE_MODULE_ID, label: 'Machine',
  async analyze(run, signal) {
    signal.throwIfAborted();
    const analysis = analyzeMachineSnapshot(execution.RunCapture.create(run.capture ?? {}));
    signal.throwIfAborted();
    return analysis;
  },
  compare: compareMachineAnalyses,
};
