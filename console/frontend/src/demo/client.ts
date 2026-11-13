import { ApiError, Phase, StepState, wire, workflow, type HistoryEntry, type Review, type Snapshot, type createClient } from '../api';
import { FILE_CHANGES_ARTIFACT, LIMITS, PREPARING_RUN } from '../generated/transport';
import { doramcap } from '../generated/console';
import { createRequestKey } from '../model';
import type { Frame } from '../streams';
import { RUN_SAMPLES, SAMPLE_MS, WORKLOAD_SERVICES, createWorkload, digest, faultPacket, noise } from './telemetry';
import { demoFileChanges, demoFileChangesJsonl } from './file-changes';

type Client = ReturnType<typeof createClient>;
type Facts = Record<string, string>;
/** Named facts per comparison group; real rows are built from these. */
type RunFacts = { outcome: Facts; setup: Facts; configuration: Facts; condition: Facts };
type Selection = { apps: string[]; appProfiles: Record<string, string> };
type StoredRun = { entry: HistoryEntry; status: Snapshot; facts: RunFacts };
type Subscriber = { consume: (status: Snapshot) => void; frame: (frame: Frame) => void; closed: () => void };

const { ComparisonGroup: Group, ComparisonResult: Result, HistoryState: State } = wire;
const { Kind, Frame: LiveFrame } = doramcap.console.live.v1;
const PREPARE_MS = 500;
const NS_PER_SAMPLE = SAMPLE_MS * 1e6;
const RUN_SECONDS = RUN_SAMPLES * SAMPLE_MS / 1000;
const REPORT = 'report', COMPLETED = 'Completed', IN_PROGRESS = 'In progress';
const JSON_DATA_URL = 'data:application/json;charset=utf-8,';
const JSONL_DATA_URL = 'data:application/x-ndjson;charset=utf-8,';
const EXIT_ON_SIGTERM = 143, STOP_SECONDS = 30;
const RECORDING_RATE_BYTES_PER_SECOND = 10 * 1024 * 1024, RECORDING_RATE_VARIATION = 0.3;

/** A sample robotics project with several Compose services. */
const PROJECT_DIRECTORY = '/home/robot/perception-stack';
const SERVICES = WORKLOAD_SERVICES.map(({ name }) => ({ name, imageId: `sha256:${digest(`image:${name}`)}` }));
const SOURCE_COMMIT = '431ea4c6d0f2b8e17a94c35d60e8f1b27c9d3a56';
const BRIDGE = 'live-bridge', CPU_MONITOR = 'cpu-monitor', DATA_RECORDER = 'data-recorder', WATCHDOG = 'fault-watchdog';
const CPU_RECORDING = 'cpu', MESSAGES_RECORDING = 'messages', FAULTS_RECORDING = 'faults';
/** System apps as `console/release/console.pbtxt` declares them; the first profile is the default. */
const SYSTEM_APPS = [
  { id: BRIDGE, label: 'Live telemetry', selected: true, profiles: [] },
  { id: CPU_MONITOR, label: 'CPU monitoring', selected: true, profiles: [{ id: 'record', label: 'Live panels and recording' }, { id: 'live', label: 'Live panels only' }] },
  { id: DATA_RECORDER, label: 'Data recorder', selected: false, profiles: [{ id: 'cpu', label: 'CPU topics (requires CPU monitoring)' }, { id: 'faults', label: 'Watchdog publications (requires watchdog)' }] },
  { id: WATCHDOG, label: 'Fault Watchdog', selected: false, profiles: [{ id: 'live', label: 'Live only' }, { id: 'mcap', label: 'Live and MCAP' }, { id: 'otel', label: 'Live and OTLP' }] },
];
const DEFAULT_SELECTION: Selection = { apps: SYSTEM_APPS.filter(app => app.selected).map(app => app.id), appProfiles: {} };

/** The built-in run; `BUILTIN_RECORDING_BYTES` matches the published file. */
export const BUILTIN_RUN_ID = 'a1b2c3d4e5f60718293a4b5c6d7e8f90';
export const BUILTIN_STARTED_AT_MS = Date.UTC(2026, 10, 3, 9, 30);
export const BUILTIN_RECORDING_BYTES = 22329;
/** Published as a release asset; `scripts/generate-demo-recording.ts` produces it. */
const RECORDING_RELEASE_TAG = 'console-demo-recording-2026-11-04';
const BUILTIN_RECORDING_URL = `https://github.com/Cutuy/doramcap/releases/download/${RECORDING_RELEASE_TAG}/${CPU_RECORDING}.mcap`;

/** Machine settings captured with the built-in run, then the ones every visitor run sees. */
const SHARED_CONFIGURATION: Facts = {
  machine_fingerprint: '9d41c7e2ab05', architecture: 'x86_64', cpu_model: 'Intel(R) Xeon(R) Silver 4314 CPU @ 2.40GHz', online_cpus: '0-7', numa_nodes: '1',
  memory_total_bytes: '68719476736', kernel_release: '6.8.0-52-generic', frequency_driver: 'intel_pstate: 0-7', turbo_enabled: 'true', smt_active: 'true',
  transparent_hugepages: 'madvise', cgroup_controllers: 'cpuset cpu io memory pids', 'docker.server_version': '27.5.1', 'docker.compose_version': '2.32.4',
  'docker.cgroup_driver': 'systemd', 'docker.cgroup_version': '2', 'docker.storage_driver': 'overlay2',
};
const CONFIGURATION: Record<'builtin' | 'current', Facts> = {
  builtin: { ...SHARED_CONFIGURATION, governor: 'powersave: 0-7', energy_preference: 'balance_power: 0-7', swappiness: '60' },
  current: { ...SHARED_CONFIGURATION, governor: 'performance: 0-7', energy_preference: 'performance: 0-7', swappiness: '10' },
};
const condition = (run: number): Facts => ({
  load_1m: (0.4 + run * 0.1).toFixed(1), memory_available_bytes: String(51_539_607_552 - run * 104_857_600), disk_free_bytes: String(412_316_860_416 - run * 26_214_400),
  max_temperature_celsius: String(47 + run), uptime_seconds: String(486_400 + run * 900),
});

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

const profileOf = (selection: Selection, id: string) => selection.appProfiles[id] ?? SYSTEM_APPS.find(app => app.id === id)!.profiles[0]?.id ?? '';
const selected = (selection: Selection, id: string) => selection.apps.includes(id);
const appList = (selection: Selection) => SYSTEM_APPS.map(app => wire.SystemApp.create({
  id: app.id, label: app.label, selected: selected(selection, app.id), profile: profileOf(selection, app.id),
  profiles: app.profiles.map(profile => wire.ProfileOption.create(profile)),
}));
const recordings = (selection: Selection) => [
  ...(selected(selection, CPU_MONITOR) && profileOf(selection, CPU_MONITOR) === 'record' ? [{ id: CPU_RECORDING, path: 'cpu.mcap', stream: 'cpu.pb', cpu: true }] : []),
  ...(selected(selection, DATA_RECORDER) ? [{ id: MESSAGES_RECORDING, path: 'messages.mcap' }] : []),
  ...(selected(selection, WATCHDOG) && profileOf(selection, WATCHDOG) === 'mcap' ? [{ id: FAULTS_RECORDING, path: 'faults.mcap' }] : []),
].map(output => wire.RecordingOutput.create(output));
const records = (selection: Selection) => recordings(selection).length > 0;
const CPU_SNAPSHOTS = `${CPU_MONITOR}_0`, CPU_DICTIONARY = `${CPU_MONITOR}_1`, FAULTS = `${WATCHDOG}_0`;
const relaysCpu = (selection: Selection) => selected(selection, BRIDGE) && selected(selection, CPU_MONITOR);
const relaysFaults = (selection: Selection) => selected(selection, BRIDGE) && selected(selection, WATCHDOG);
/** Stream IDs follow the server: the app ID and the output's index. */
const liveStreams = (selection: Selection) => [
  ...(relaysCpu(selection) ? [CPU_SNAPSHOTS, CPU_DICTIONARY].map(id => wire.LiveStream.create({ id, appId: CPU_MONITOR, kind: Kind.CPU_ACCOUNTING })) : []),
  ...(relaysFaults(selection) ? [wire.LiveStream.create({ id: FAULTS, appId: WATCHDOG, kind: Kind.FAULT_PUBLICATION })] : []),
];
const services = () => SERVICES.map(service => wire.Service.create(service));
const appSelection = (selection: Selection) => wire.SystemAppSelection.create({ ids: selection.apps, profiles: Object.fromEntries(selection.apps.filter(id => profileOf(selection, id)).map(id => [id, profileOf(selection, id)])) });
const setupFacts = (selection: Selection): Facts => ({
  profiles: 'Default', system_apps: selection.apps.join(', ') || 'None',
  ...Object.fromEntries(selection.apps.filter(id => profileOf(selection, id)).map(id => [`system_app_profile.${id}`, profileOf(selection, id)])),
  ...Object.fromEntries(SERVICES.map(service => [`image.${service.name}`, service.imageId])), source_commit: SOURCE_COMMIT, source_state: 'Clean',
});

/** Production's step IDs and display names, in workflow order; durations are in samples. */
const type = (path: string) => `workload_docker::${path}`;
const lifecycle = (name: string) => `workload_lifecycle::recording::${name}`;
const STEP = {
  reset: { step: 'compose.reset', displayName: 'Reset experiment', samples: 2 },
  prepare: { step: 'compose.prepare', displayName: 'Prepare project', samples: 5 },
  baseline: { step: type('compose::filesystem::CaptureFilesystemBaseline'), samples: 2 },
  dora: { step: type('workflows::experiment::StartDoraRuntime'), samples: 3 },
  graph: { step: type('workflows::experiment::RegisterDoraGraph'), samples: 2 },
  start: { step: type('compose::steps::StartCompose'), samples: 3 },
  wait: { step: lifecycle('WaitForRecordings'), samples: 2 },
  inspect: { step: 'compose.inspect', displayName: 'Inspect project', samples: 1 },
  apps: { step: type('compose::steps::StartCompose'), samples: 2 },
  quiesceApps: { step: type('compose::steps::QuiesceCompose'), samples: 2 },
  stopDora: { step: type('workflows::experiment::StopDoraRuntime'), samples: 2 },
  quiesceSystem: { step: type('compose::steps::QuiesceCompose'), samples: 2 },
  finalize: { step: 'compose.finalize', displayName: 'Finalize recording', samples: 2 },
  finalizeRecordings: { step: lifecycle('FinalizeRecordings'), samples: 1 },
  remove: { step: type('compose::steps::RemoveCompose'), samples: 1 },
  report: { step: 'compose.report', displayName: 'Write run report', samples: 1 },
};
const INSPECT_EVERY_SAMPLES = 10;
const startup = (saving: boolean) => [STEP.reset, STEP.prepare, STEP.dora, STEP.graph, STEP.start, ...(saving ? [STEP.wait] : []), STEP.inspect, STEP.apps];
const SHUTDOWN = [STEP.quiesceApps, STEP.stopDora, STEP.quiesceSystem, STEP.finalize, STEP.remove, STEP.report];
const total = (stages: { samples: number }[]) => stages.reduce((sum, stage) => sum + stage.samples, 0);
const STOP_SAMPLES = total(SHUTDOWN);

/** Workflow rows at `sample`, as the orchestrator reports them; a stop cuts the current step short. */
function stepsAt(sample: number, stopAt: number | null, finished: boolean, saving: boolean) {
  const stopping = stopAt ?? Infinity;
  const rows: { step: string; displayName?: string; from: number; to: number; parent?: number; after?: number }[] = [];
  const add = (stage: { step: string; displayName?: string }, from: number, to: number, parent?: number, after?: number) => {
    rows.push({ step: stage.step, displayName: stage.displayName, from, to, parent, after });
    return rows.length;
  };
  let at = 0, previous: number | undefined;
  for (const stage of startup(saving)) {
    if (at >= stopping) break;
    const end = Math.min(at + stage.samples, stopping);
    previous = add(stage, at, end, undefined, previous);
    // Preparation captures the filesystem baseline as a child step.
    if (stage === STEP.prepare && at + 1 < end) add(STEP.baseline, at + 1, Math.min(at + 1 + STEP.baseline.samples, end), previous);
    at = end;
  }
  for (; at < stopping && at <= sample; at += INSPECT_EVERY_SAMPLES) add(STEP.inspect, at, Math.min(at + STEP.inspect.samples, stopping), undefined, previous);
  if (stopAt !== null) {
    at = stopAt;
    for (const stage of SHUTDOWN) {
      previous = add(stage, at, at + stage.samples, undefined, previous);
      if (stage === STEP.finalize && saving) add(STEP.finalizeRecordings, at + 1, at + 1 + STEP.finalizeRecordings.samples, previous);
      at += stage.samples;
    }
  }
  return rows.map((row, index) => ({ row, invocation: index + 1 })).filter(({ row }) => row.from <= sample || finished).map(({ row, invocation }) => {
    const done = finished || row.to <= sample;
    return workflow.Event.create({
      version: 1, sequence: invocation, session: 'console', executor: 'local', invocation, step: row.step, displayName: row.displayName ?? '',
      parent: row.parent ?? null, dependencies: row.after ? [row.after] : [],
      state: done ? StepState.SUCCEEDED : StepState.STARTED,
      elapsedNs: (done ? row.to : row.from) * NS_PER_SAMPLE, durationNs: done ? (row.to - row.from) * NS_PER_SAMPLE : 0,
    });
  });
}
/** The built-in run's write history, matching its published file. */
const builtinMetricsAt = (sample: number) => Array.from({ length: sample }, (_, index) => wire.RunMetric.create({
  elapsedNs: (index + 1) * NS_PER_SAMPLE,
  recordingBytes: Math.floor(BUILTIN_RECORDING_BYTES * (index + 1) / RUN_SAMPLES),
  writeBytesPerSecond: BUILTIN_RECORDING_BYTES / RUN_SECONDS,
}));

/** A visitor run's write history: at least the base rate while services run, nothing otherwise. */
function liveMetricsAt(sample: number, stopAt: number | null, runningFrom: number, seed: number) {
  let bytes = 0;
  return Array.from({ length: sample }, (_, index) => {
    const at = index + 1;
    const rate = at >= runningFrom && (stopAt === null || at <= stopAt) ? Math.round(RECORDING_RATE_BYTES_PER_SECOND * (1 + RECORDING_RATE_VARIATION * noise(seed, at))) : 0;
    bytes += rate * SAMPLE_MS / 1000;
    return wire.RunMetric.create({ elapsedNs: at * NS_PER_SAMPLE, recordingBytes: Math.floor(bytes), writeBytesPerSecond: rate });
  });
}

/** `seed` is set for a visitor run; the built-in run omits it and replays its published file. */
function runStatus(runId: string, selection: Selection, phase: number, sample: number, stopAt: number | null, conditions: Facts, after: Facts, seed?: number) {
  const finished = phase === Phase.FINISHED, saving = records(selection);
  const steps = stepsAt(sample, stopAt, finished, saving);
  // Like the server, `sequence` counts workflow events; other samples leave it unchanged.
  const sequence = steps.reduce((events, step) => events + (step.state === StepState.STARTED ? 1 : 2), 0);
  return wire.Status.create({
    phase, runId, sequence, projectDirectory: PROJECT_DIRECTORY, profiles: [], systemApps: appSelection(selection),
    steps, activityAvailable: true, metrics: !saving ? [] : seed === undefined ? builtinMetricsAt(sample) : liveMetricsAt(sample, stopAt, total(startup(saving)), seed),
    recordings: recordings(selection), liveStreams: liveStreams(selection), cpuAvailable: relaysCpu(selection), reportAvailable: finished,
    stoppingServices: phase === Phase.STOPPING ? SERVICES.map(({ name }) => wire.StoppingService.create({ service: name, forceStopInSeconds: STOP_SECONDS })) : [],
    machineChanges: finished ? diff(Group.COMPARISON_CONDITION, conditions, after) : [],
    outcome: finished ? wire.Outcome.create({
      execution: COMPLETED, writersStopped: saving, recordingFinalized: saving, boundaryClean: true, eventStreamHealthy: true,
      serviceExits: SERVICES.map(({ name }) => wire.ServiceExit.create({ service: name, exitCode: EXIT_ON_SIGTERM })),
    }) : null,
  });
}
function finishedEntry(runId: string, selection: Selection, startedAtMs: number, finishedAtMs: number, status: Snapshot, recordingBytes?: number) {
  return wire.HistoryEntry.create({
    runId, profiles: [], systemApps: appSelection(selection), startedAtMs, finishedAtMs, state: State.HISTORY_COMPLETED,
    artifacts: [
      ...(recordingBytes === undefined ? [] : [wire.HistoryArtifact.create({ id: CPU_RECORDING, sizeBytes: recordingBytes, downloadable: true })]),
      wire.HistoryArtifact.create({ id: REPORT, sizeBytes: new TextEncoder().encode(report(status)).length, downloadable: true }),
      wire.HistoryArtifact.create({ id: FILE_CHANGES_ARTIFACT, sizeBytes: new TextEncoder().encode(demoFileChangesJsonl(status.fileChanges!)).length, downloadable: true, kind: wire.ArtifactKind.FILE_CHANGES }),
    ],
  });
}
const outcomeFacts = (status: Snapshot, seconds: number): Facts => ({
  execution: status.outcome?.execution ?? '', duration_seconds: seconds.toFixed(1),
  recording_saved: String(status.outcome?.recordingFinalized ?? false), cleanup_verified: String(status.outcome?.boundaryClean ?? false),
});

function builtinRun(): StoredRun {
  const selection = DEFAULT_SELECTION;
  const status = runStatus(BUILTIN_RUN_ID, selection, Phase.FINISHED, RUN_SAMPLES, RUN_SAMPLES - STOP_SAMPLES, condition(0), condition(1));
  const duration = RUN_SECONDS;
  status.fileChanges = demoFileChanges(status, BUILTIN_STARTED_AT_MS, BUILTIN_STARTED_AT_MS + duration * 1000);
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
  let runCount = 0;
  let frames: Frame[] = [];
  let prepared: { id: string; runId: string; selection: Selection; facts: RunFacts } | null = null;
  let preparing: { timer: ReturnType<typeof setTimeout>; reject: (error: Error) => void } | null = null;
  let active: { runId: string; requestKey: string; selection: Selection; startedAtMs: number; facts: RunFacts; after: Facts; sample: number; stopAt: number | null; seed: number; workload: ReturnType<typeof createWorkload>; timer: ReturnType<typeof setTimeout> } | null = null;
  let lastKey: { key: string; runId: string } | null = null;

  const publish = (next: Snapshot) => {
    status = next;
    for (const subscriber of [...subscribers]) subscriber.consume(next);
  };
  const stageStatus = (phase: number, runId = '') => wire.Status.create({ phase, runId, projectDirectory: PROJECT_DIRECTORY });
  const publishRun = (phase: number) => {
    const run = active!;
    publish(runStatus(run.runId, run.selection, phase, run.sample, run.stopAt, run.facts.condition, run.after, run.seed));
    const stored = runs.get(run.runId)!;
    stored.status = status;
  };
  const finish = () => {
    const run = active!;
    active = null;
    const seconds = run.sample * SAMPLE_MS / 1000;
    const finished = runStatus(run.runId, run.selection, Phase.FINISHED, run.sample, run.stopAt, run.facts.condition, run.after, run.seed);
    finished.fileChanges = demoFileChanges(finished, run.startedAtMs, run.startedAtMs + seconds * 1000);
    publish(finished);
    runs.set(run.runId, {
      status, facts: { ...run.facts, outcome: { ...outcomeFacts(status, seconds), ...(status.metrics.length ? { recording_bytes: String(status.metrics.at(-1)!.recordingBytes) } : {}) } },
      entry: finishedEntry(run.runId, run.selection, run.startedAtMs, run.startedAtMs + seconds * 1000, status),
    });
  };
  const relay = (stream: string, sequence: number, payload: Uint8Array) => {
    const frame = LiveFrame.create({ stream, sequence, payload });
    frames.push(frame);
    for (const subscriber of [...subscribers]) subscriber.frame(frame);
  };
  const tick = () => {
    const run = active!;
    run.sample++;
    if (relaysCpu(run.selection)) {
      if (run.sample === 1) relay(CPU_DICTIONARY, 1, run.workload.dictionary(run.startedAtMs));
      const running = run.stopAt === null && run.sample >= total(startup(records(run.selection)));
      relay(CPU_SNAPSHOTS, run.sample, run.workload.packet(run.sample, run.startedAtMs, running));
    }
    if (run.sample === 1 && relaysFaults(run.selection)) relay(FAULTS, 1, faultPacket());
    if (run.stopAt === null && run.sample >= RUN_SAMPLES - STOP_SAMPLES) run.stopAt = run.sample;
    if (run.stopAt !== null && run.sample >= run.stopAt + STOP_SAMPLES) return finish();
    publishRun(run.stopAt !== null ? Phase.STOPPING : run.sample >= total(startup(records(run.selection))) ? Phase.RUNNING : Phase.STARTING);
    // Pace against the start time so a long run does not drift.
    run.timer = setTimeout(tick, Math.max(0, run.startedAtMs + (run.sample + 1) * SAMPLE_MS - Date.now()));
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
    prepared = { id: preparedId, runId, selection, facts: { outcome: { execution: IN_PROGRESS }, setup, configuration: CONFIGURATION.current, condition: current } };
    return wire.Review.create({
      preparedId, runId, availableProfiles: [], selectedProfiles: [], services: services(), systemApps: appList(selection),
      recordings: recordings(selection), liveStreams: liveStreams(selection),
      comparison: wire.ReviewComparison.create({ baseline: baseline.entry, rows: [
        ...diff(Group.COMPARISON_SETUP, baseline.facts.setup, setup),
        ...diff(Group.COMPARISON_CONFIGURATION, baseline.facts.configuration, CONFIGURATION.current),
        ...diff(Group.COMPARISON_CONDITION, {}, current),
      ] }),
    });
  };

  const client = {
    prepare: (_profiles: string[], systemApps?: string[], appProfiles: Record<string, string> = {}, baselineRunId = '') => new Promise<Review>((resolve, reject) => {
      if (preparing || active) { reject(new ApiError('Conflict')); return; }
      const selection: Selection = { apps: systemApps ?? DEFAULT_SELECTION.apps, appProfiles };
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
      const startedAtMs = Date.now(), seed = Math.floor(Math.random() * 2 ** 32);
      runs.set(runId, { status, facts, entry: wire.HistoryEntry.create({ runId, profiles: [], systemApps: appSelection(selection), startedAtMs, state: State.HISTORY_RUNNING }) });
      active = { runId, requestKey, selection, startedAtMs, facts, after: condition(++runCount), sample: 0, stopAt: null, seed, workload: createWorkload(seed, PROJECT_DIRECTORY, runId), timer: setTimeout(tick, SAMPLE_MS) };
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
      if (id === FILE_CHANGES_ARTIFACT) return run?.status.fileChanges?.downloadable ? JSONL_DATA_URL + encodeURIComponent(demoFileChangesJsonl(run.status.fileChanges)) : undefined;
      return runId === BUILTIN_RUN_ID && id === CPU_RECORDING ? BUILTIN_RECORDING_URL : undefined;
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
