import { closeSync, readFileSync } from 'node:fs';
import Long from 'long';
import { capabilities, execution, systemConfig, wire } from './native.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
type Row = InstanceType<typeof wire.ComparisonRow>;
type Capture = typeof execution.RunCapture.prototype;
type Fact = { value: string; hidden?: boolean };
export type Facts = Map<string, Fact>;
/** The configuration a run or review resolved; absent when its file cannot be read safely. */
export type Manifest = Record<string, any> | null;
export type Side = { id: string; detail: typeof wire.HistoryDetail.prototype; capture: Capture; manifest: Manifest };

const MANIFEST_FILE = 'experiment.input.json';
const MAX_MANIFEST_BYTES = 4 * 1024 * 1024;
/** Bounds together keep one comparison well below the transport's frame limit. */
const MAX_ROWS = 2000, MAX_VALUE_CHARS = 512;
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
  if (state?.hostShmError) {
    facts.set('host_shm', { value: '' });
    facts.set('host_shm_error', { value: state.hostShmError });
    return facts;
  }
  if (!state?.hostShm) return facts;
  const snapshot = systemConfig.HostSharedMemory.toObject(systemConfig.HostSharedMemory.fromObject(state.hostShm), { longs: String, defaults: true });
  facts.set('host_shm', { value: `${snapshot.posix.length} POSIX entries, ${snapshot.systemV.length} System V segments` });
  for (const object of snapshot.posix) {
    const { path, ...metadata } = object;
    facts.set(`host_shm.posix.${path}`, { value: JSON.stringify(metadata) });
  }
  for (const object of snapshot.systemV)
    facts.set(`host_shm.system_v.${object.id}`, { value: JSON.stringify(object) });
  return facts;
}

/** Read one saved configuration through the safe open, with its own run ID removed. */
export function readManifest(root: string, id: string, relative = `${id}/${MANIFEST_FILE}`): Manifest {
  try {
    const fd = capabilities.openFile(root, relative);
    try {
      const bytes = readFileSync(fd);
      if (bytes.length > MAX_MANIFEST_BYTES) return null;
      return JSON.parse(bytes.toString('utf8').replaceAll(id, '@run').replaceAll('reviews/@run/resolved', 'runs/@run'));
    } finally {
      closeSync(fd);
    }
  } catch {
    return null;
  }
}
/** A frozen input's name carries its digest, so its contents compare without being read or shown. */
const file = (source: string): Fact => {
  const frozen = FROZEN_FILE.exec(source);
  return frozen ? { value: frozen[1], hidden: true } : { value: source };
};
export function manifestFacts(manifest: Manifest, facts: Facts = new Map()): Facts {
  for (const [service, definition] of Object.entries<any>(manifest?.services ?? {})) {
    for (const [name, value] of Object.entries(definition.environment ?? {})) {
      if (GENERATED_ENVIRONMENT.has(name)) continue;
      const text = String(value ?? '');
      facts.set(`environment.${service}.${name}`, { value: text, hidden: isSensitive(name, text) });
    }
    for (const volume of definition.volumes ?? [])
      if (volume.type === 'bind') facts.set(`mount.${service}.${volume.target}`, file(volume.source));
  }
  for (const [name, config] of Object.entries<any>(manifest?.configs ?? {}))
    if (config.file) facts.set(`config.${name}`, file(config.file));
  // Compose declares these as secrets, so their contents always stay hidden.
  for (const [name, secret] of Object.entries<any>(manifest?.secrets ?? {}))
    if (secret.file) facts.set(`secret.${name}`, { ...file(secret.file), hidden: true });
  return facts;
}
export type SetupSide = Pick<Side, 'capture' | 'manifest'> & { profiles: string[]; apps?: typeof wire.SystemAppSelection.prototype | null };
function selectionFacts({ profiles, apps }: SetupSide): Facts {
  const facts: Facts = new Map([['profiles', { value: profiles.join(', ') || 'Default' }]]);
  facts.set('system_apps', { value: apps?.ids?.join(', ') || 'None' });
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
/** Compare each source apart, so a run missing one source reads as unavailable for that source only. */
const setupRows = (a: SetupSide, b: SetupSide) => [selectionFacts, capturedSetupFacts, ({ manifest }: SetupSide) => manifestFacts(manifest)]
  .flatMap((facts) => diffFacts(Group.COMPARISON_SETUP, facts(a), facts(b)));
function outcomeFacts({ detail }: Side): Facts {
  const entry = detail.entry!, outcome = detail.status?.outcome;
  const unfinished = entry.state === wire.HistoryState.HISTORY_RUNNING ? 'In progress' : 'Interrupted';
  const facts: Facts = new Map([['execution', { value: outcome?.execution || unfinished }]]);
  if (entry.finishedAtMs != null) facts.set('duration_seconds', { value: ((Number(entry.finishedAtMs) - Number(entry.startedAtMs)) / 1000).toFixed(1) });
  const sizes = (entry.artifacts ?? []).filter((artifact) => artifact.id !== 'report' && artifact.sizeBytes != null);
  if (sizes.length) facts.set('recording_bytes', { value: String(sizes.reduce((total, artifact) => total + Number(artifact.sizeBytes), 0)) });
  if (outcome?.recordingFinalized != null) facts.set('recording_saved', { value: String(outcome.recordingFinalized) });
  if (outcome?.boundaryClean != null) facts.set('cleanup_verified', { value: String(outcome.boundaryClean) });
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
    const result = !recorded ? Result.COMPARISON_UNAVAILABLE : left?.value === right?.value ? Result.COMPARISON_SAME : Result.COMPARISON_CHANGED;
    const hidden = !!(left?.hidden || right?.hidden);
    return wire.ComparisonRow.create({
      group, name, result, hidden,
      cells: [left, right].map((fact) => ({ values: [fact && !hidden ? bounded(name === 'boot_flags' ? maskBootFlags(fact.value) : fact.value) : ''] })),
    });
  });
}
const differs = (row: Row) => row.result !== Result.COMPARISON_SAME;
/** Prioritize capture errors, differences, then matching facts within the row budget. */
function boundRows(rows: Row[]) {
  if (rows.length <= MAX_ROWS) return { rows, truncated: false };
  const errors = rows.filter(row => row.name === 'host_shm_error');
  const facts = rows.filter(row => row.name !== 'host_shm_error');
  const kept = [...errors, ...facts.filter(differs), ...facts.filter((row) => !differs(row))].slice(0, MAX_ROWS);
  return { rows: kept, truncated: true };
}
/** Changes between Prepare and post-run captures; capture errors remain visible with either side missing. */
export function machineChanges(before: Capture['before'], after: Capture['before']) {
  return [
    ...diffFacts(Group.COMPARISON_CONFIGURATION, configurationFacts(before), configurationFacts(after)),
    ...diffFacts(Group.COMPARISON_CONDITION, conditionFacts(before), conditionFacts(after)),
  ].filter((row) => row.result === Result.COMPARISON_CHANGED || row.name === 'host_shm_error');
}
const setupSide = (side: Side): SetupSide => ({ ...side, profiles: side.detail.entry?.profiles ?? [], apps: side.detail.entry?.systemApps as never });

/** Compare two stored runs by what they ran, how they ended and the machine they ran on. */
export function compareRuns(a: Side, b: Side, [labelA, labelB]: string[] = ['A', 'B']) {
  const { rows, truncated } = boundRows([
    ...diffFacts(Group.COMPARISON_OUTCOME, outcomeFacts(a), outcomeFacts(b)),
    ...setupRows(setupSide(a), setupSide(b)),
    ...diffFacts(Group.COMPARISON_CONFIGURATION, configurationFacts(a.capture.before), configurationFacts(b.capture.before)),
    ...diffFacts(Group.COMPARISON_CONDITION, conditionFacts(a.capture.before), conditionFacts(b.capture.before)),
  ]);
  const compared = (label: string, side: Side) => ({ label, runs: [{ entry: side.detail.entry, captured: !!side.capture.before }] });
  return wire.HistoryComparison.create({ sides: [compared(labelA, a), compared(labelB, b)], rows, truncated });
}

const emptyFacts: Facts = new Map();
/** Compare what a review would run with a baseline run. Machine condition shows current values only. */
export function compareReview(baseline: Side | null, current: SetupSide) {
  const state = current.capture.before;
  const { rows, truncated } = boundRows([
    ...(baseline ? setupRows(setupSide(baseline), current) : []),
    ...diffFacts(Group.COMPARISON_CONFIGURATION, baseline ? configurationFacts(baseline.capture.before) : emptyFacts, configurationFacts(state)),
    ...diffFacts(Group.COMPARISON_CONDITION, emptyFacts, conditionFacts(state)),
  ]);
  return wire.ReviewComparison.create({ baseline: baseline?.detail.entry, rows, truncated });
}
