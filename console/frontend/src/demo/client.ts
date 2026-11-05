import { ApiError, Phase, StepState, wire, workflow, type HistoryEntry, type Review, type Snapshot, type createClient } from '../api';
import { LIMITS, PREPARING_RUN } from '../generated/transport';
import { doramcap } from '../generated/console';
import { createRequestKey } from '../model';
import type { Frame } from '../streams';
import { RUN_SAMPLES, SAMPLE_SECONDS, cpuPacket } from './telemetry';

type Client = ReturnType<typeof createClient>;
type Facts = Record<string, string>;
/** Named facts per comparison group; real rows are built from these. */
type RunFacts = { outcome: Facts; setup: Facts; configuration: Facts; condition: Facts };
type Selection = { profiles: string[]; apps: string[]; appProfiles: Record<string, string> };
type StoredRun = { entry: HistoryEntry; status: Snapshot; facts: RunFacts };
type Subscriber = { consume: (status: Snapshot) => void; frame: (frame: Frame) => void; closed: () => void };

const { ComparisonGroup: Group, ComparisonResult: Result, HistoryState: State } = wire;
const { Kind } = doramcap.console.live.v1;
const SAMPLE_MS = SAMPLE_SECONDS * 1000;
const PREPARE_MS = 500;
const NS_PER_SAMPLE = SAMPLE_SECONDS * 1e9;
/** A run starts for this many samples and takes this many to stop. */
const START_SAMPLES = 3, STOP_SAMPLES = 2;
const STOP_SECONDS_REMAINING = 30;
const PROJECT_DIRECTORY = '/demo/activity-lab';
const HEAVY_PROFILE = 'load', RECORD_PROFILE = 'record', LIVE_PROFILE = 'live';
const CPU_APP = 'cpu-monitor', BRIDGE_APP = 'live-bridge';
const CPU_RECORDING = 'cpu', REPORT = 'report', CPU_STREAM = 'cpu_0';
const COMPLETED = 'Completed';
const JSON_DATA_URL = 'data:application/json;charset=utf-8,';

/** The built-in run; `BUILTIN_RECORDING_BYTES` matches the bundled file. */
export const BUILTIN_RUN_ID = 'a1b2c3d4e5f60718293a4b5c6d7e8f90';
export const BUILTIN_STARTED_AT_MS = Date.UTC(2026, 10, 3, 9, 30);
export const BUILTIN_RECORDING_BYTES = 8416;
const builtinRecordingUrl = () => new URL('./builtin-cpu.mcap', import.meta.url).href;

const SERVICES = { app: { name: 'activity-lab', imageId: 'sha256:5c1f0a7e92b4d3866f1e2a7c40b9d15e3f8a6c72' }, stress: { name: 'load-generator', imageId: 'sha256:9e4d7b21c08f6a35d1b72e9046ca8f13b5d07e64' } };
/** The machine captured with the built-in run, then the one every visitor run sees. */
const CONFIGURATION: Record<'builtin' | 'current', Facts> = {
  builtin: { architecture: 'x86_64', cpu_model: 'Demo Xeon 8-core', kernel_release: '6.8.0-demo', governors: 'powersave: 0-3', turbo_enabled: 'true', smt_active: 'true', swappiness: '60' },
  current: { architecture: 'x86_64', cpu_model: 'Demo Xeon 8-core', kernel_release: '6.8.0-demo', governors: 'performance: 0-3', turbo_enabled: 'true', smt_active: 'true', swappiness: '10' },
};
const condition = (run: number): Facts => ({ load_1m: (0.4 + run * 0.1).toFixed(1), max_temperature_celsius: String(47 + run), uptime_seconds: String(86_400 + run * 900) });

const report = (status: Snapshot) => JSON.stringify({
  execution: status.outcome?.execution, error: status.outcome?.error || null, writers_stopped: status.outcome?.writersStopped,
  recording_finalized: status.outcome?.recordingFinalized,
  service_exits: (status.outcome?.serviceExits ?? []).map(exit => ({ service: exit.service, exit_code: exit.exitCode, oom_killed: exit.oomKilled, killed: exit.killed })),
  boundary_clean: status.outcome?.boundaryClean, event_stream_healthy: status.outcome?.eventStreamHealthy, system_apps: status.systemApps?.ids ?? [],
});

/** Same when equal, Changed when not, Unavailable when a side recorded nothing; matches Console's comparison rows. */
function diff(group: number, a: Facts, b: Facts) {
  const recorded = Object.keys(a).length > 0 && Object.keys(b).length > 0;
  return [...new Set([...Object.keys(a), ...Object.keys(b)])].sort().map(name => wire.ComparisonRow.create({
    group, name,
    result: !recorded ? Result.COMPARISON_UNAVAILABLE : a[name] === b[name] ? Result.COMPARISON_SAME : Result.COMPARISON_CHANGED,
    cells: [a, b].map(side => wire.ComparisonCell.create({ values: [side[name] ?? ''] })),
  }));
}

const appList = (selection: Selection) => [
  wire.SystemApp.create({ id: BRIDGE_APP, label: 'Live telemetry', selected: selection.apps.includes(BRIDGE_APP) }),
  wire.SystemApp.create({
    id: CPU_APP, label: 'CPU monitoring', selected: selection.apps.includes(CPU_APP), profile: selection.appProfiles[CPU_APP] ?? RECORD_PROFILE,
    profiles: [wire.ProfileOption.create({ id: RECORD_PROFILE, label: 'Live panels and recording' }), wire.ProfileOption.create({ id: LIVE_PROFILE, label: 'Live panels only' })],
  }),
];
const records = (selection: Selection) => selection.apps.includes(CPU_APP) && (selection.appProfiles[CPU_APP] ?? RECORD_PROFILE) === RECORD_PROFILE;
const recordings = (selection: Selection) => records(selection) ? [wire.RecordingOutput.create({ id: CPU_RECORDING, path: `${CPU_RECORDING}.mcap`, cpu: true })] : [];
const streamsLive = (selection: Selection) => selection.apps.includes(CPU_APP) && selection.apps.includes(BRIDGE_APP);
const liveStreams = (selection: Selection) => streamsLive(selection) ? [wire.LiveStream.create({ id: CPU_STREAM, appId: CPU_APP, kind: Kind.CPU_ACCOUNTING })] : [];
const heavy = (selection: Selection) => selection.profiles.includes(HEAVY_PROFILE);
const services = (selection: Selection) => (heavy(selection) ? [SERVICES.app, SERVICES.stress] : [SERVICES.app]).map(service => wire.Service.create(service));
const appSelection = (selection: Selection) => wire.SystemAppSelection.create({ ids: selection.apps, profiles: selection.appProfiles });
const setupFacts = (selection: Selection): Facts => ({
  profiles: selection.profiles.join(', ') || 'Default', system_apps: selection.apps.join(', ') || 'None',
  ...(selection.apps.includes(CPU_APP) ? { [`system_app_profile.${CPU_APP}`]: selection.appProfiles[CPU_APP] ?? RECORD_PROFILE } : {}),
  ...Object.fromEntries(services(selection).map(service => [`image.${service.name}`, service.imageId])),
  source_state: 'Clean',
});

/** Workflow rows at `sample`; a stop cuts the current step short and begins stopping. */
function stepsAt(sample: number, stopAt: number | null, finished: boolean) {
  const stopping = stopAt ?? Infinity;
  const stages = [
    { step: 'compose.images', displayName: 'Check images', from: 0, to: 1 },
    { step: 'compose.launch', displayName: 'Start project', from: 1, to: START_SAMPLES },
    { step: 'experiment.capture', displayName: 'Capture activity', from: START_SAMPLES, to: Infinity },
  ].filter(stage => stage.from < stopping && stage.from <= sample).map(stage => ({ ...stage, to: Math.min(stage.to, stopping) }));
  if (stopAt !== null) stages.push({ step: 'compose.stop', displayName: 'Stop project', from: stopAt, to: stopAt + STOP_SAMPLES });
  return stages.map((stage, index) => {
    const done = finished || stage.to <= sample;
    return workflow.Event.create({
      version: 1, sequence: index + 1, session: 'demo', executor: 'local', invocation: index + 1, step: stage.step, displayName: stage.displayName,
      state: done ? StepState.SUCCEEDED : StepState.STARTED,
      elapsedNs: (done ? stage.to : stage.from) * NS_PER_SAMPLE, durationNs: done ? (stage.to - stage.from) * NS_PER_SAMPLE : 0,
    });
  });
}
const metricsAt = (sample: number) => Array.from({ length: sample }, (_, index) => wire.RunMetric.create({
  elapsedNs: (index + 1) * NS_PER_SAMPLE,
  recordingBytes: Math.floor(BUILTIN_RECORDING_BYTES * (index + 1) / RUN_SAMPLES),
  writeBytesPerSecond: BUILTIN_RECORDING_BYTES / RUN_SAMPLES / SAMPLE_SECONDS,
}));

function runStatus(runId: string, selection: Selection, phase: number, sample: number, stopAt: number | null, sequence: number) {
  const finished = phase === Phase.FINISHED, saving = records(selection);
  return wire.Status.create({
    phase, runId, sequence, projectDirectory: PROJECT_DIRECTORY, profiles: selection.profiles, systemApps: appSelection(selection),
    steps: stepsAt(sample, stopAt, finished), activityAvailable: true, metrics: saving ? metricsAt(sample) : [],
    recordings: recordings(selection), liveStreams: liveStreams(selection), cpuAvailable: streamsLive(selection), reportAvailable: finished,
    stoppingServices: phase === Phase.STOPPING ? services(selection).map(service => wire.StoppingService.create({ service: service.name, forceStopInSeconds: STOP_SECONDS_REMAINING })) : [],
    outcome: finished ? wire.Outcome.create({ execution: COMPLETED, writersStopped: saving, recordingFinalized: saving, boundaryClean: true, eventStreamHealthy: true }) : null,
  });
}
function finishedEntry(runId: string, selection: Selection, startedAtMs: number, finishedAtMs: number, status: Snapshot, recordingBytes?: number) {
  return wire.HistoryEntry.create({
    runId, profiles: selection.profiles, systemApps: appSelection(selection), startedAtMs, finishedAtMs, state: State.HISTORY_COMPLETED,
    artifacts: [
      ...(recordingBytes === undefined ? [] : [wire.HistoryArtifact.create({ id: CPU_RECORDING, sizeBytes: recordingBytes, downloadable: true })]),
      wire.HistoryArtifact.create({ id: REPORT, sizeBytes: new TextEncoder().encode(report(status)).length, downloadable: true }),
    ],
  });
}
const outcomeFacts = (status: Snapshot, seconds: number): Facts => ({
  execution: status.outcome?.execution ?? '', duration_seconds: seconds.toFixed(1),
  recording_saved: String(status.outcome?.recordingFinalized ?? false), cleanup_verified: String(status.outcome?.boundaryClean ?? false),
});

function builtinRun(): StoredRun {
  const selection: Selection = { profiles: [], apps: [BRIDGE_APP, CPU_APP], appProfiles: { [CPU_APP]: RECORD_PROFILE } };
  const status = runStatus(BUILTIN_RUN_ID, selection, Phase.FINISHED, RUN_SAMPLES, RUN_SAMPLES - STOP_SAMPLES, RUN_SAMPLES);
  const duration = RUN_SAMPLES * SAMPLE_SECONDS;
  return {
    entry: finishedEntry(BUILTIN_RUN_ID, selection, BUILTIN_STARTED_AT_MS, BUILTIN_STARTED_AT_MS + duration * 1000, status, BUILTIN_RECORDING_BYTES), status,
    facts: { setup: setupFacts(selection), configuration: CONFIGURATION.builtin, condition: condition(0),
      outcome: { ...outcomeFacts(status, duration), recording_bytes: String(BUILTIN_RECORDING_BYTES) } },
  };
}

/**
 * A browser-only Console client over one in-memory Activity lab project.
 * Starts with the built-in run; visitor runs live until the page refreshes.
 * The result also has `dispose()`, which cancels pending work and ends every `live` call.
 */
export function createDemoClient() {
  const runs = new Map<string, StoredRun>([[BUILTIN_RUN_ID, builtinRun()]]);
  const subscribers = new Set<Subscriber>();
  let status = wire.Status.create({ phase: Phase.IDLE, projectDirectory: PROJECT_DIRECTORY });
  let sequence = 0, runCount = 0;
  let frames: Frame[] = [];
  let prepared: { id: string; runId: string; selection: Selection; facts: RunFacts } | null = null;
  let preparing: { timer: ReturnType<typeof setTimeout>; reject: (error: Error) => void } | null = null;
  let active: { runId: string; requestKey: string; selection: Selection; startedAtMs: number; facts: RunFacts; sample: number; stopAt: number | null; timer: ReturnType<typeof setTimeout> } | null = null;
  let lastKey: { key: string; runId: string } | null = null;

  const publish = (next: Snapshot) => {
    status = next;
    for (const subscriber of [...subscribers]) subscriber.consume(next);
  };
  const stageStatus = (phase: number, runId = '') => wire.Status.create({ phase, runId, sequence: ++sequence, projectDirectory: PROJECT_DIRECTORY });
  const publishRun = (phase: number) => {
    const run = active!;
    publish(runStatus(run.runId, run.selection, phase, run.sample, run.stopAt, ++sequence));
    const stored = runs.get(run.runId)!;
    stored.status = status;
  };
  const finish = () => {
    const run = active!;
    active = null;
    publish(runStatus(run.runId, run.selection, Phase.FINISHED, run.sample, run.stopAt, ++sequence));
    const seconds = run.sample * SAMPLE_SECONDS;
    runs.set(run.runId, {
      status, facts: { ...run.facts, outcome: outcomeFacts(status, seconds) },
      entry: finishedEntry(run.runId, run.selection, run.startedAtMs, run.startedAtMs + seconds * 1000, status),
    });
  };
  const tick = () => {
    const run = active!;
    run.sample++;
    if (streamsLive(run.selection)) {
      const frame = doramcap.console.live.v1.Frame.create({ stream: CPU_STREAM, sequence: run.sample, payload: cpuPacket(run.sample, heavy(run.selection), run.startedAtMs) });
      frames.push(frame);
      for (const subscriber of [...subscribers]) subscriber.frame(frame);
    }
    if (run.stopAt === null && run.sample >= RUN_SAMPLES - STOP_SAMPLES) run.stopAt = run.sample;
    if (run.stopAt !== null && run.sample >= run.stopAt + STOP_SAMPLES) return finish();
    publishRun(run.stopAt !== null ? Phase.STOPPING : run.sample >= START_SAMPLES ? Phase.RUNNING : Phase.STARTING);
    run.timer = setTimeout(tick, SAMPLE_MS);
  };

  /** The run a review compares with: the named one, else the latest completed. */
  const baselineFor = (id: string) => {
    const named = runs.get(id);
    if (named && named.entry.state === State.HISTORY_COMPLETED) return named;
    return [...runs.values()].filter(run => run.entry.state === State.HISTORY_COMPLETED).at(-1)!;
  };
  const buildReview = (selection: Selection, baselineId: string): Review => {
    const baseline = baselineFor(baselineId), runId = createRequestKey(), preparedId = createRequestKey();
    const setup = setupFacts(selection), current = condition(++runCount);
    prepared = { id: preparedId, runId, selection, facts: { outcome: {}, setup, configuration: CONFIGURATION.current, condition: current } };
    return wire.Review.create({
      preparedId, runId, availableProfiles: [HEAVY_PROFILE], selectedProfiles: selection.profiles, services: services(selection), systemApps: appList(selection),
      recordings: recordings(selection), liveStreams: liveStreams(selection),
      comparison: wire.ReviewComparison.create({ baseline: baseline.entry, rows: [
        ...diff(Group.COMPARISON_SETUP, baseline.facts.setup, setup),
        ...diff(Group.COMPARISON_CONFIGURATION, baseline.facts.configuration, CONFIGURATION.current),
        ...diff(Group.COMPARISON_CONDITION, {}, current),
      ] }),
    });
  };

  const client = {
    prepare: (profiles: string[], systemApps?: string[], appProfiles: Record<string, string> = {}, baselineRunId = '') => new Promise<Review>((resolve, reject) => {
      if (preparing || active) { reject(new ApiError('Conflict')); return; }
      const selection: Selection = { profiles, apps: systemApps ?? [BRIDGE_APP, CPU_APP], appProfiles };
      publish(stageStatus(Phase.PREPARING));
      const timer = setTimeout(() => {
        preparing = null;
        const review = buildReview(selection, baselineRunId);
        publish(stageStatus(Phase.REVIEWED, review.runId));
        resolve(review);
      }, PREPARE_MS);
      preparing = { timer, reject };
    }),
    start: async (preparedId: string, requestKey: string) => {
      if (lastKey?.key === requestKey) return wire.StartResponse.create({ runId: lastKey.runId });
      if (preparing || active) throw new ApiError('Conflict');
      if (prepared?.id !== preparedId) throw new ApiError('StaleReview');
      const { runId, selection, facts } = prepared;
      prepared = null;
      lastKey = { key: requestKey, runId };
      frames = [];
      const startedAtMs = Date.now();
      runs.set(runId, { status, facts, entry: wire.HistoryEntry.create({ runId, profiles: selection.profiles, systemApps: appSelection(selection), startedAtMs, state: State.HISTORY_RUNNING }) });
      active = { runId, requestKey, selection, startedAtMs, facts, sample: 0, stopAt: null, timer: setTimeout(tick, SAMPLE_MS) };
      publishRun(Phase.STARTING);
      return wire.StartResponse.create({ runId });
    },
    stop: async (runId: string) => {
      if (runId === PREPARING_RUN && preparing) {
        clearTimeout(preparing.timer);
        preparing.reject(new ApiError('Stopped'));
        preparing = null;
        publish(stageStatus(Phase.IDLE));
        return wire.StopResponse.create({ accepted: true });
      }
      if (active?.runId !== runId) throw new ApiError('Conflict');
      if (active.stopAt === null) { active.stopAt = active.sample; publishRun(Phase.STOPPING); }
      return wire.StopResponse.create({ accepted: true });
    },
    current: async () => status,
    history: async (query: { state?: number; cursor?: string; limit?: number }) => {
      const entries = [...runs.values()].map(run => run.entry).filter(entry => !query.state || entry.state === query.state).reverse();
      const from = query.cursor ? entries.findIndex(entry => entry.runId === query.cursor) + 1 : 0;
      const limit = query.limit || LIMITS.historyPageDefault, page = entries.slice(from, from + limit);
      return wire.HistoryPage.create({ entries: page, nextCursor: from + limit < entries.length ? page.at(-1)!.runId : '' });
    },
    historyRun: async (runId: string) => {
      const run = runs.get(runId);
      if (!run) throw new ApiError('Missing');
      return wire.HistoryDetail.create({ entry: run.entry, status: run.status });
    },
    historyCompare: async (a: string, b: string) => {
      const left = runs.get(a), right = runs.get(b);
      if (!left || !right) throw new ApiError('Missing');
      const compared = (label: string, run: StoredRun) => wire.ComparisonSide.create({ label, runs: [wire.ComparedRun.create({ entry: run.entry, captured: true })] });
      return wire.HistoryComparison.create({ sides: [compared('A', left), compared('B', right)], rows: [
        ...diff(Group.COMPARISON_OUTCOME, left.facts.outcome, right.facts.outcome),
        ...diff(Group.COMPARISON_SETUP, left.facts.setup, right.facts.setup),
        ...diff(Group.COMPARISON_CONFIGURATION, left.facts.configuration, right.facts.configuration),
        ...diff(Group.COMPARISON_CONDITION, left.facts.condition, right.facts.condition),
      ] });
    },
    artifactUrl: (runId: string, id: string): string | undefined => {
      const run = runs.get(runId);
      if (id === REPORT) return run?.status.reportAvailable ? JSON_DATA_URL + encodeURIComponent(report(run.status)) : undefined;
      return runId === BUILTIN_RUN_ID && id === CPU_RECORDING ? builtinRecordingUrl() : undefined;
    },
    live: (consume: (status: Snapshot) => void, signal: AbortSignal, frame: (frame: Frame) => void = () => {}) => new Promise<void>(resolve => {
      if (signal.aborted) { resolve(); return; }
      const subscriber: Subscriber = { consume, frame, closed: () => { subscribers.delete(subscriber); signal.removeEventListener('abort', subscriber.closed); resolve(); } };
      subscribers.add(subscriber);
      signal.addEventListener('abort', subscriber.closed);
      // A returning screen rebuilds its panels from the current run's frames.
      consume(status);
      for (const retained of frames) frame(retained);
    }),
    dispose: () => {
      if (active) clearTimeout(active.timer);
      if (preparing) { clearTimeout(preparing.timer); preparing.reject(new ApiError('Stopped')); }
      active = preparing = null;
      for (const subscriber of [...subscribers]) subscriber.closed();
    },
  } satisfies Client & { dispose: () => void };
  return client;
}
