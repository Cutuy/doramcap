import type { doramcap } from '../../../generated/console.js';
import Long from 'long';
import { execution, systemConfig, wire } from '../../../native.js';
import { filesystem } from '../../../records/file-changes.js';
import { STOP_GRACE_PERIOD_VERSION, type SetupRecord } from '../../../records/setup.js';
import { comparedFactValue, factLabel } from './presentation.js';
import { boundRows } from '../../rows.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
type Row = InstanceType<typeof wire.ComparisonRow>;
type Capture = typeof execution.RunCapture.prototype;
type Fact = { value: string; hidden?: boolean; unavailable?: boolean };
export type Facts = Map<string, Fact>;
export type Side = { id: string; detail: typeof wire.HistoryDetail.prototype; capture: Capture; setup: SetupRecord | null };

/** Row and value budgets; encoded responses also enforce the UTF-8 transport byte limit. */
const MAX_VALUE_CHARS = 512;
const FINGERPRINT_CHARS = 12;
/** Names whose values are withheld: passwords, tokens, keys, credentials and session material. */
const SENSITIVE_NAME = /pass(word|wd|phrase)?|secret|token|credential|auth|cookie|session|private|(^|[_.-])(api|access|ssh|signing|encryption)?[_.-]?key($|[_.-])/i;
/** A URL carrying a username, a password or both. */
const URL_CREDENTIALS = /:\/\/[^\s/@]+@/;
/** Assigned by Console for each run, so a difference says nothing about the experiment. */
const GENERATED_ENVIRONMENT = new Set(['DORA_DAEMON_LOCAL_LISTEN_PORT']);
const FROZEN_FILE = /\/inputs\/([0-9a-f]{16,}-[0-7]+)$/;

export const isSensitive = (name: string, value: string) => SENSITIVE_NAME.test(name) || URL_CREDENTIALS.test(value);
const snake = (name: string) => name.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
const bounded = (value: string) => (value.length > MAX_VALUE_CHARS ? `${value.slice(0, MAX_VALUE_CHARS)}…` : value);
/** Withhold kernel parameters by the same name rules as environment values. */
const maskBootFlags = (flags: string) =>
  flags.split(/\s+/).map((flag) => {
    const [name, value = ''] = flag.split(/=(.*)/s);
    return isSensitive(name, value) ? `${name}=(hidden)` : flag;
  }).join(' ');
const display = (name: string, value: unknown): string => {
  if (name === 'machine_fingerprint') return String(value).slice(0, FINGERPRINT_CHARS);
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
    else facts.set(name, { value: display(name, value) });
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

/** Apply name masks to argument words, including assignment flags and shell fragments. */
const sensitiveInvocation = (value: unknown) => {
  const text = Array.isArray(value) ? value.join(' ') : String(value ?? '');
  return text.split(/[\s"'`=,:;()[\]{}]+/).some(name => isSensitive(name, text));
};
/** Compose null/missing inherits the image default; empty text/list clears it. */
const argumentValue = (value: doramcap.compose.v1.IInvocationValue | null | undefined): string | string[] | undefined => {
  if (!value) return undefined;
  if (value.arguments) return value.arguments.values ?? [];
  return value.text || [];
};
export function invocationFacts(record: SetupRecord | null): Facts {
  const facts: Facts = new Map();
  for (const [service, definition] of Object.entries(record?.services ?? {})) {
    const invocation = definition.normalized ?? definition.captured;
    for (const [field, value] of [['command', argumentValue(invocation?.command)], ['entrypoint', argumentValue(invocation?.entrypoint)], ['stop_signal', invocation?.stopSignal || undefined], ['stop_grace_period', invocation?.stopGracePeriod || undefined]] as const) {
      const text = value === undefined ? '' : JSON.stringify(value);
      facts.set(`${field}.${service}`, { value: text, hidden: (field === 'command' || field === 'entrypoint') && sensitiveInvocation(value) });
    }
  }
  return facts;
}
/** A frozen input's name carries its digest, so its contents compare without being read or shown. */
const file = (source: string): Fact => {
  const frozen = FROZEN_FILE.exec(source);
  return frozen ? { value: frozen[1], hidden: true } : { value: source };
};
export function setupFacts(record: SetupRecord | null, facts: Facts = new Map()): Facts {
  const path = (value: string) => record?.runId ? value.replaceAll(record.runId, '@run').replaceAll('reviews/@run/resolved', 'runs/@run') : value;
  for (const [service, definition] of Object.entries(record?.services ?? {})) {
    for (const [name, value] of Object.entries(definition.environment ?? {})) {
      const text = path(value);
      if (!GENERATED_ENVIRONMENT.has(name)) facts.set(`environment.${service}.${name}`, { value: text, hidden: isSensitive(name, text) });
    }
    for (const bind of definition.binds ?? []) facts.set(`mount.${service}.${path(bind.target ?? '')}`, file(path(bind.source ?? '')));
  }
  for (const [name, source] of Object.entries(record?.configs ?? {})) facts.set(`config.${name}`, file(path(source)));
  for (const [name, source] of Object.entries(record?.secrets ?? {})) facts.set(`secret.${name}`, { ...file(path(source)), hidden: true });
  return facts;
}
export type SetupSide = Pick<Side, 'capture' | 'setup'> & { profiles: string[]; apps?: typeof wire.SystemAppSelection.prototype | null; writePolicy?: number | null };
function selectionFacts({ profiles, apps, writePolicy }: SetupSide): Facts {
  const facts: Facts = new Map([['profiles', { value: profiles.join(', ') || 'Default' }]]);
  facts.set('system_apps', { value: apps?.ids?.join(', ') || 'None' });
  facts.set('write_policy', { value: filesystem.WritePolicy[writePolicy || filesystem.WritePolicy.WRITE_POLICY_WRITE_BACK] ?? 'Unknown' });
  for (const [app, profile] of Object.entries(apps?.profiles ?? {})) facts.set(`system_app_profile.${app}`, { value: profile });
  return facts;
}
function capturedSetupFacts({ capture: { setup } }: SetupSide): Facts {
  const facts: Facts = new Map();
  if (setup?.sourceCommit) facts.set('source_commit', { value: setup.sourceCommit });
  if (setup) facts.set('source_state', { value: setup.sourceState === execution.SourceState.SOURCE_STATE_CLEAN ? 'Clean'
    : setup.sourceState === execution.SourceState.SOURCE_STATE_DIRTY ? 'Dirty' : 'Unknown' });
  for (const { service, imageId } of setup?.images ?? []) facts.set(`image.${service}`, { value: imageId! });
  return facts;
}
function outcomeFacts({ detail }: Side): Facts {
  const entry = detail.entry!, outcome = detail.status?.outcome;
  const unfinished = entry.state === wire.HistoryState.HISTORY_RUNNING ? 'In progress' : 'Interrupted';
  const facts: Facts = new Map([['execution', { value: outcome?.execution || unfinished }]]);
  if (entry.finishedAtMs != null) facts.set('duration_seconds', { value: ((Number(entry.finishedAtMs) - Number(entry.startedAtMs)) / 1000).toFixed(1) });
  const sizes = (entry.artifacts ?? []).filter((artifact) => artifact.id !== 'report' && artifact.kind === wire.ArtifactKind.RECORDING && artifact.sizeBytes != null);
  if (sizes.length) facts.set('recording_bytes', { value: String(sizes.reduce((total, artifact) => total + Number(artifact.sizeBytes), 0)) });
  if (outcome?.recordingFinalized != null) facts.set('recording_saved', { value: String(outcome.recordingFinalized) });
  if (outcome?.boundaryClean != null) facts.set('cleanup_verified', { value: String(outcome.boundaryClean) });
  const exits = new Map<string, string[]>();
  for (const exit of outcome?.serviceExits ?? []) {
    const values = exits.get(exit.service!) ?? [];
    values.push('Exit ' + exit.exitCode + (exit.oomKilled ? '; OOM killed' : exit.killed ? '; killed' : ''));
    exits.set(exit.service!, values);
  }
  for (const [service, values] of exits) facts.set('service.' + service + '.exit', { value: values.sort().join(', ') });
  return facts;
}
/**
 * Compare one group by name. Values compare before masking, and hidden values never reach a row.
 * A fact set in one run only is a change; a side with no facts at all recorded nothing, so it is unavailable.
 */
export function diffFacts(group: number, a: Facts, b: Facts): Row[] {
  const recorded = a.size > 0 && b.size > 0;
  return [...new Set([...a.keys(), ...b.keys()])].sort().map((name) => {
    const left = a.get(name), right = b.get(name);
    const result = !recorded || left?.unavailable || right?.unavailable ? Result.COMPARISON_UNAVAILABLE : left?.value === right?.value ? Result.COMPARISON_SAME : Result.COMPARISON_CHANGED;
    const hidden = !!(left?.hidden || right?.hidden);
    return wire.ComparisonRow.create({
      group, name, result, hidden, alwaysVisible: name === 'machine_capture_error', label: bounded(factLabel(name)),
      cells: [left, right].map((fact) => {
        const raw = fact && !hidden ? name === 'boot_flags' ? maskBootFlags(fact.value) : fact.value : '';
        const others = [left, right].map(item => item?.value ?? '');
        const shown = bounded(hidden ? '' : comparedFactValue(name, raw, others));
        return { values: [bounded(raw)], displayValues: shown === bounded(raw) ? [] : [shown],
          evidence: hidden ? '' : fact?.unavailable ? 'Unavailable' : '' };
      }),
    });
  });
}

type Kind = 'history' | 'review' | 'machine';
/** Private materialized measurements; the shared runner never interprets these facts. */
export type MaterializedAnalysis = {
  kind: Kind; selection: Facts; captured: Facts; setup: Facts; invocations: Facts;
  normalized: boolean; setupVersion: number; configuration: Facts; condition: Facts;
  error: string; outcome: Facts;
};
const emptyFacts: Facts = new Map();

/** Extract one immutable observation before comparing it with another side. */
export function analyzeSnapshot(kind: Kind, side: SetupSide & { detail?: typeof wire.HistoryDetail.prototype | null }): MaterializedAnalysis {
  return {
    kind, selection: selectionFacts(side), captured: capturedSetupFacts(side), setup: setupFacts(side.setup),
    invocations: invocationFacts(side.setup),
    normalized: !!side.setup && Object.values(side.setup.services).every(service => service.normalized != null),
    setupVersion: side.setup?.version ?? 0,
    configuration: configurationFacts(side.capture.before), condition: conditionFacts(side.capture.before),
    error: side.capture.beforeError,
    outcome: side.detail?.entry ? outcomeFacts({ ...side, id: side.detail.entry.runId ?? '', detail: side.detail }) : new Map(),
  };
}
function invocationRows(a: MaterializedAnalysis, b: MaterializedAnalysis) {
  const captured = a.normalized && b.normalized;
  return diffFacts(Group.COMPARISON_SETUP, a.invocations, b.invocations).map(row => captured && (!row.name.startsWith('stop_grace_period.')
    || (a.setupVersion >= STOP_GRACE_PERIOD_VERSION && b.setupVersion >= STOP_GRACE_PERIOD_VERSION))
    ? row : wire.ComparisonRow.create({ ...row, result: Result.COMPARISON_UNAVAILABLE }));
}
const setupRows = (a: MaterializedAnalysis, b: MaterializedAnalysis) => [
  ...diffFacts(Group.COMPARISON_SETUP, a.selection, b.selection),
  ...diffFacts(Group.COMPARISON_SETUP, a.captured, b.captured),
  ...diffFacts(Group.COMPARISON_SETUP, a.setup, b.setup), ...invocationRows(a, b),
];
function outcomeRows(a: MaterializedAnalysis, b: MaterializedAnalysis) {
  const left = new Map(a.outcome), right = new Map(b.outcome);
  for (const name of new Set([...left.keys(), ...right.keys()])) {
    if (!name.startsWith('service.')) continue;
    if (!left.has(name)) left.set(name, { value: 'Unavailable', unavailable: true });
    if (!right.has(name)) right.set(name, { value: 'Unavailable', unavailable: true });
  }
  return diffFacts(Group.COMPARISON_OUTCOME, left, right);
}
function machineRows(a: MaterializedAnalysis, b: MaterializedAnalysis) {
  const errorFacts = (error: string): Facts => error ? new Map([['machine_capture_error', { value: error }]]) : emptyFacts;
  return [
    ...diffFacts(Group.COMPARISON_CONFIGURATION, a.error ? emptyFacts : a.configuration, b.error ? emptyFacts : b.configuration),
    ...diffFacts(Group.COMPARISON_CONDITION, a.error ? emptyFacts : a.condition, b.error ? emptyFacts : b.condition),
    ...diffFacts(Group.COMPARISON_CONDITION, errorFacts(a.error), errorFacts(b.error))
      .map(row => wire.ComparisonRow.create({ ...row, result: Result.COMPARISON_UNAVAILABLE })),
  ];
}
const emptyMachine = () => analyzeSnapshot('machine', { capture: execution.RunCapture.create(), setup: null, profiles: [] });
/** Compare private facts for saved History, a prepared Review, or a live machine observation. */
export function compareAnalyses(a: MaterializedAnalysis, b: MaterializedAnalysis): Row[] {
  if (b.kind === 'history') return [...outcomeRows(a, b), ...setupRows(a, b), ...machineRows(a, b)];
  if (b.kind === 'review') return [
    ...(a.kind === 'history' ? setupRows(a, b) : []),
    ...diffFacts(Group.COMPARISON_CONFIGURATION, a.kind === 'history' ? a.configuration : emptyFacts, b.error ? emptyFacts : b.configuration),
    ...machineRows(emptyMachine(), b).filter(row => row.group === Group.COMPARISON_CONDITION),
  ];
  const rows = machineRows(a, b).filter(row => row.result === Result.COMPARISON_CHANGED || row.name === 'machine_capture_error');
  return boundRows(rows).truncated ? machineRows(emptyMachine(), { ...emptyMachine(), error: 'Machine comparison is too large to display. Changes are unavailable.' }) : rows;
}
const setupSide = (side: Side): SetupSide & { detail: typeof wire.HistoryDetail.prototype } => ({ ...side, profiles: side.detail.entry?.profiles ?? [], apps: side.detail.entry?.systemApps as never, writePolicy: side.detail.status?.writePolicy });
/** Compare stored snapshots through the materialized module's fact extraction and comparer. */
export function compareRuns(a: Side, b: Side, [labelA, labelB]: string[] = ['A', 'B']) {
  const result = boundRows(compareAnalyses(analyzeSnapshot('history', setupSide(a)), analyzeSnapshot('history', setupSide(b))));
  const compared = (label: string, side: Side) => ({ label, runs: [{ entry: side.detail.entry, captured: !!side.capture.before }] });
  return wire.HistoryComparison.create({ sides: [compared(labelA, a), compared(labelB, b)], ...result });
}
/** Compare a prepared snapshot; machine condition displays the current observation. */
export function compareReview(baseline: Side | null, current: SetupSide) {
  const result = boundRows(compareAnalyses(baseline ? analyzeSnapshot('history', setupSide(baseline)) : emptyMachine(), analyzeSnapshot('review', current)));
  return wire.ReviewComparison.create({ baseline: baseline?.detail.entry, ...result });
}
/** Compare live observations; exhaustion remains an explicit unavailable result. */
export function machineChanges(before: Capture['before'], after: Capture['before'], beforeError = '', afterError = '') {
  const input = (before: Capture['before'], beforeError: string) => analyzeSnapshot('machine', { capture: execution.RunCapture.create({ before, beforeError }), setup: null, profiles: [] });
  return compareAnalyses(input(before, beforeError), input(after, afterError));
}
