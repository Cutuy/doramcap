import { doramcap } from '../../../generated/console.js';
import { execution, wire } from '../../../native.js';
import { readSetupRecord, STOP_GRACE_PERIOD_VERSION, type SetupRecord } from '../../../records/setup.js';
import type { AnalysisModule } from '../../module.js';
import { diffFacts, type Fact, type Facts } from '../facts.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const filesystem = doramcap.filesystem.v1;
type Capture = typeof execution.RunCapture.prototype;
const SENSITIVE_NAME = /pass(word|wd|phrase)?|secret|token|credential|auth|cookie|session|private|(^|[_.-])(api|access|ssh|signing|encryption)?[_.-]?key($|[_.-])/i;
const URL_CREDENTIALS = /:\/\/[^\s/@]+@/;
const GENERATED_ENVIRONMENT = new Set(['DORA_DAEMON_LOCAL_LISTEN_PORT']);
const FROZEN_FILE = /\/inputs\/([0-9a-f]{16,}-[0-7]+)$/;
const isSensitive = (name: string, value: string) => SENSITIVE_NAME.test(name) || URL_CREDENTIALS.test(value);

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
      facts.set(`${field}.${service}`, { value: text, withheld: (field === 'command' || field === 'entrypoint') && sensitiveInvocation(value) });
    }
  }
  return facts;
}
/** A frozen input's name carries its digest, so its contents compare without being read or shown. */
const file = (source: string): Fact => {
  const frozen = FROZEN_FILE.exec(source);
  return frozen ? { value: frozen[1], withheld: true } : { value: source };
};
export function setupFacts(record: SetupRecord | null, facts: Facts = new Map()): Facts {
  const path = (value: string) => record?.runId ? value.replaceAll(record.runId, '@run').replaceAll('reviews/@run/resolved', 'runs/@run') : value;
  for (const [service, definition] of Object.entries(record?.services ?? {})) {
    for (const [name, value] of Object.entries(definition.environment ?? {})) {
      const text = path(value);
      if (!GENERATED_ENVIRONMENT.has(name)) facts.set(`environment.${service}.${name}`, { value: text, withheld: isSensitive(name, text) });
    }
    for (const bind of definition.binds ?? []) facts.set(`mount.${service}.${path(bind.target ?? '')}`, file(path(bind.source ?? '')));
  }
  for (const [name, source] of Object.entries(record?.configs ?? {})) facts.set(`config.${name}`, file(path(source)));
  for (const [name, source] of Object.entries(record?.secrets ?? {})) facts.set(`secret.${name}`, { ...file(path(source)), withheld: true });
  return facts;
}
export type SetupSide = { capture: Capture; setup: SetupRecord | null; profiles: string[]; apps?: typeof wire.SystemAppSelection.prototype | null; writePolicy?: number | null };
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
function outcomeFacts(detail: doramcap.console.v1.HistoryDetail.$Properties): Facts {
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

/** Run-owned measurements; extraction has no History/Review presentation policy. */
export type RunAnalysis = {
  selection: Facts; captured: Facts; setup: Facts; invocations: Facts;
  normalized: boolean; setupVersion: number; outcome: Facts;
};
/** Extract original setup and recorded outcomes into exact run measurements. */
export function analyzeRunSnapshot(side: SetupSide & { detail?: doramcap.console.v1.HistoryDetail.$Properties | null }): RunAnalysis {
  return {
    selection: selectionFacts(side), captured: capturedSetupFacts(side), setup: setupFacts(side.setup),
    invocations: invocationFacts(side.setup),
    normalized: !!side.setup && Object.values(side.setup.services).every(service => service.normalized != null),
    setupVersion: side.setup?.version ?? 0,
    outcome: side.detail?.entry ? outcomeFacts(side.detail) : new Map(),
  };
}
/** Compare run setup and outcomes; the caller chooses the workflow's result groups. */
export function compareRunAnalyses(a: RunAnalysis, b: RunAnalysis) {
  const left = new Map(a.outcome), right = new Map(b.outcome);
  for (const name of new Set([...left.keys(), ...right.keys()])) {
    if (!name.startsWith('service.')) continue;
    if (!left.has(name)) left.set(name, { value: 'Unavailable', unavailable: true });
    if (!right.has(name)) right.set(name, { value: 'Unavailable', unavailable: true });
  }
  const invocations = diffFacts(Group.COMPARISON_SETUP, a.invocations, b.invocations).map(row => a.normalized && b.normalized
    && (!row.name.startsWith('stop_grace_period.') || (a.setupVersion >= STOP_GRACE_PERIOD_VERSION && b.setupVersion >= STOP_GRACE_PERIOD_VERSION))
    ? row : wire.ComparisonFact.create({ ...row, result: Result.COMPARISON_UNAVAILABLE }));
  return [
    ...diffFacts(Group.COMPARISON_OUTCOME, left, right),
    ...diffFacts(Group.COMPARISON_SETUP, a.selection, b.selection),
    ...diffFacts(Group.COMPARISON_SETUP, a.captured, b.captured),
    ...diffFacts(Group.COMPARISON_SETUP, a.setup, b.setup), ...invocations,
  ];
}
export const RUN_MODULE_ID = 'run';
/** Compare run configuration and outcomes using original run-scoped evidence. */
export const runModule: AnalysisModule<RunAnalysis> = {
  id: RUN_MODULE_ID, label: 'Run',
  async analyze(run, signal) {
    signal.throwIfAborted();
    const history = run.snapshotHistory, current = run.snapshotCurrent;
    if (current && (history || current.phase !== wire.Phase.REVIEWED)) throw new Error('InvalidInput');
    const id = history?.entry?.runId ?? current?.runId ?? '';
    const setup = run.directory && id ? readSetupRecord(run.directory, id, '') : null;
    const analysis = analyzeRunSnapshot({ setup, capture: execution.RunCapture.create(run.capture ?? {}), detail: history,
      profiles: current?.profiles ?? history?.entry?.profiles ?? [],
      apps: wire.SystemAppSelection.create(current?.systemApps ?? history?.entry?.systemApps ?? {}),
      writePolicy: current?.writePolicy ?? history?.status?.writePolicy,
    });
    signal.throwIfAborted();
    return analysis;
  },
  compare: compareRunAnalyses,
};
