import { strict as assert } from 'node:assert';
import { afterEach, beforeEach, mock, test } from 'node:test';
import { McapStreamReader } from '@mcap/core';
import protobuf from 'protobufjs';
import 'protobufjs/ext/descriptor/index.js';
import { ApiError, Phase, StepState, wire, type Snapshot } from './api';
import { generateRecording } from '../scripts/generate-demo-recording';
import { BUILTIN_RECORDING_BYTES, BUILTIN_RUN_ID, BUILTIN_STARTED_AT_MS, createDemoClient } from './demo/client';
import { RUN_SAMPLES, SAMPLE_MS } from './demo/telemetry';
import { PREPARING_RUN } from './generated/transport';
import { LiveModel } from './streams';

const { ComparisonGroup: Group, ComparisonResult: Result, HistoryState: State } = wire;
const PREPARE_MS = 500, SECOND_MS = 1000, RUN_MS = RUN_SAMPLES * SAMPLE_MS;
const NS_PER_MS = 1_000_000n, SAMPLE_NS = BigInt(SAMPLE_MS) * NS_PER_MS;
const LIVE_ONLY = { 'cpu-monitor': 'live' };
const PRODUCTION_STEPS = ['compose.reset', 'compose.prepare', 'compose.inspect', 'compose.finalize', 'compose.report'];
const SERVER_REPORT_FIELDS = ['execution', 'error', 'writers_stopped', 'recording_finalized', 'service_exits', 'boundary_clean', 'event_stream_healthy', 'system_apps'];

const realFetch = globalThis.fetch;
beforeEach(() => {
  mock.timers.enable({ apis: ['setTimeout', 'Date'] });
  // The demo must complete with every network request blocked.
  globalThis.fetch = () => { throw new Error('The demo must not use the network'); };
});
afterEach(() => { mock.timers.reset(); globalThis.fetch = realFetch; });

const settle = async () => { for (let turn = 0; turn < 5; turn++) await Promise.resolve(); };
const advance = async (ms: number) => { for (let left = ms; left > 0; left -= 100) { mock.timers.tick(100); await settle(); } };
function watch(client: ReturnType<typeof createDemoClient>) {
  const seen: Snapshot[] = [], controller = new AbortController(), model = new LiveModel();
  const ended = client.live(status => { seen.push(status); model.configure(status.runId, status.liveStreams); }, controller.signal, frame => model.receive(frame));
  return { seen, model, controller, ended, last: () => seen.at(-1)! };
}
async function review(client: ReturnType<typeof createDemoClient>, profiles: string[] = [], apps?: string[], appProfiles?: Record<string, string>, baseline?: string) {
  const pending = client.prepare(profiles, apps, appProfiles, baseline);
  await advance(PREPARE_MS);
  return pending;
}
async function finishedRun(client: ReturnType<typeof createDemoClient>, profiles: string[] = [], appProfiles?: Record<string, string>) {
  const prepared = await review(client, profiles, undefined, appProfiles);
  const { runId } = await client.start(prepared.preparedId, `key-${prepared.runId}`);
  await advance(RUN_MS);
  return runId;
}

test('the demo starts idle with one completed built-in run', async () => {
  // Arrange
  const client = createDemoClient();
  // Act
  const page = await client.history({});
  const detail = await client.historyRun(BUILTIN_RUN_ID);
  // Assert
  assert.equal((await client.current()).phase, Phase.IDLE);
  assert.deepEqual(page.entries.map(entry => entry.runId), [BUILTIN_RUN_ID]);
  assert.equal(detail.entry!.state, State.HISTORY_COMPLETED);
  assert.equal(detail.status!.outcome!.recordingFinalized, true);
  assert.equal(Number(detail.entry!.artifacts[0].sizeBytes), BUILTIN_RECORDING_BYTES);
  assert.equal(detail.status!.metrics.length, RUN_SAMPLES);
  assert.equal(Number(detail.status!.metrics.at(-1)!.recordingBytes), BUILTIN_RECORDING_BYTES);
});

test('the generated recording is a valid capture matching the built-in run', async () => {
  // Arrange
  const file = await generateRecording();
  const reader = new McapStreamReader({ includeChunks: true, validateCrcs: true });
  let root: protobuf.Root | undefined, name = '';
  const messages: { channelId: number; logTime: bigint; data: Uint8Array }[] = [];
  const topics = new Map<number, string>();
  let footer = false;
  // Act
  reader.append(file);
  for (let record; (record = reader.nextRecord());) {
    if (record.type === 'Schema') { root = (protobuf.Root as unknown as { fromDescriptor(data: Uint8Array): protobuf.Root }).fromDescriptor(record.data); name = record.name; }
    else if (record.type === 'Channel') topics.set(record.id, record.topic);
    else if (record.type === 'Message') messages.push(record);
    else if (record.type === 'Footer') footer = true;
  }
  const Packet = root!.lookupType(name);
  const samples = messages.filter(message => topics.get(message.channelId) === 'doramcap/cpuacct');
  const dictionary = messages.filter(message => topics.get(message.channelId) === 'doramcap/cpuacct/dictionary');
  const names = Packet.toObject(Packet.decode(dictionary[0].data), { longs: String }) as { dictionary: { entries: { path: string }[] } };
  // Assert
  assert.ok(footer);
  assert.equal(file.length, BUILTIN_RECORDING_BYTES);
  assert.equal(name, 'doramcap.telemetry.v1.Packet');
  assert.equal(samples.length, RUN_SAMPLES);
  assert.equal(dictionary.length, 1);
  assert.equal(samples[0].logTime, BigInt(BUILTIN_STARTED_AT_MS) * NS_PER_MS + SAMPLE_NS);
  assert.equal(samples.at(-1)!.logTime - samples[0].logTime, BigInt(RUN_SAMPLES - 1) * SAMPLE_NS);
  assert.ok(names.dictionary.entries.every(entry => /^\/system\.slice\/docker-[0-9a-f]{64}\.scope$/.test(entry.path)));
});

test('a run progresses through its phases to Completed and joins History', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  // Act
  const { runId } = await client.start(prepared.preparedId, 'key');
  await advance(SECOND_MS);
  const starting = screen.last().phase;
  await advance(3 * SECOND_MS);
  const running = screen.last().phase;
  await advance(RUN_MS);
  // Assert
  assert.equal(runId, prepared.runId);
  assert.equal(starting, Phase.STARTING);
  assert.equal(running, Phase.RUNNING);
  assert.equal(screen.last().phase, Phase.FINISHED);
  assert.equal(screen.last().outcome!.execution, 'Completed');
  assert.ok(screen.seen.some(status => status.phase === Phase.STOPPING));
  assert.equal(screen.model.view().cpu!.history.length, RUN_SAMPLES - 1);
  assert.match(screen.model.view().cpu!.groups[0].name, /^docker-[0-9a-f]{64}\.scope$/);
  assert.ok(screen.model.view().cpu!.groups[0].percent > 90);
  assert.deepEqual((await client.history({})).entries.map(entry => entry.runId), [runId, BUILTIN_RUN_ID]);
});

test('a run samples at 10 Hz for 15 s and advances its sequence only on workflow events', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  const before = screen.seen.length;
  // Act
  await client.start(prepared.preparedId, 'key');
  await advance(RUN_MS);
  const run = screen.seen.slice(before);
  const sequences = run.map(status => Number(status.sequence));
  // Assert
  assert.equal(SAMPLE_MS, 100);
  assert.equal(RUN_MS, 15 * SECOND_MS);
  assert.equal(run.length, RUN_SAMPLES + 1);
  assert.equal(run.at(-1)!.phase, Phase.FINISHED);
  assert.deepEqual(sequences, [...sequences].sort((a, b) => a - b));
  assert.ok(new Set(sequences).size < run.length / 2, 'most samples leave the sequence unchanged');
});

test('the demo offers the release system apps and emits production workflow steps', async () => {
  // Arrange
  const client = createDemoClient();
  // Act
  const prepared = await review(client);
  const runId = await finishedRun(client);
  const { status } = await client.historyRun(runId);
  const steps = status!.steps.map(step => step.step);
  // Assert
  assert.deepEqual(prepared.systemApps.map(app => [app.id, app.label, app.selected]), [
    ['live-bridge', 'Live telemetry', true], ['cpu-monitor', 'CPU monitoring', true], ['data-recorder', 'Data recorder', false], ['fault-watchdog', 'Fault Watchdog', false],
  ]);
  assert.deepEqual(prepared.systemApps[3].profiles.map(profile => profile.id), ['live', 'mcap', 'otel']);
  assert.deepEqual(prepared.liveStreams.map(stream => stream.id), ['cpu-monitor_0', 'cpu-monitor_1']);
  assert.deepEqual(prepared.services.map(service => service.name), ['worker']);
  for (const step of PRODUCTION_STEPS) assert.ok(steps.includes(step), step);
  assert.ok(steps.includes('workload_docker::compose::steps::StartCompose'));
  const child = status!.steps.find(step => step.step.endsWith('CaptureFilesystemBaseline'))!;
  assert.equal(status!.steps[Number(child.parent) - 1].step, 'compose.prepare');
  assert.ok(status!.steps.every(step => step.state === StepState.SUCCEEDED));
  assert.ok(status!.machineChanges.length > 0);
});

test('selecting the watchdog relays faults and adds its recording', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const apps = ['live-bridge', 'cpu-monitor', 'fault-watchdog'];
  // Act
  const prepared = await review(client, [], apps, { 'fault-watchdog': 'mcap' });
  await client.start(prepared.preparedId, 'key');
  await advance(SAMPLE_MS);
  // Assert
  assert.deepEqual(prepared.recordings.map(output => output.id), ['cpu', 'faults']);
  assert.ok(prepared.liveStreams.some(stream => stream.id === 'fault-watchdog_0'));
  assert.equal(screen.model.view().faults!.connected, 1);
  assert.equal(screen.model.view().faults!.stale, false);
});

test('Stop ends a run early as Completed', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  const { runId } = await client.start(prepared.preparedId, 'key');
  await advance(5 * SECOND_MS);
  // Act
  await client.stop(runId);
  const stopping = screen.last().phase;
  await advance(2 * SECOND_MS);
  // Assert
  assert.equal(stopping, Phase.STOPPING);
  assert.equal(screen.last().phase, Phase.FINISHED);
  assert.equal(screen.last().outcome!.execution, 'Completed');
  const detail = await client.historyRun(runId);
  assert.equal(Number(detail.entry!.finishedAtMs) - Number(detail.entry!.startedAtMs), 7 * SECOND_MS);
});

test('preparation can be cancelled and allows another review', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const pending = client.prepare([]);
  const outcome = assert.rejects(pending, (error: ApiError) => error.code === 'Stopped');
  // Act
  await client.stop(PREPARING_RUN);
  await outcome;
  // Assert
  assert.equal(screen.last().phase, Phase.IDLE);
  assert.ok((await review(client)).preparedId);
});

test('start reuses its request key and refuses stale reviews and a second active run', async () => {
  // Arrange
  const client = createDemoClient();
  const stale = await review(client), fresh = await review(client);
  // Act / Assert
  await assert.rejects(client.start(stale.preparedId, 'old'), (error: ApiError) => error.code === 'StaleReview');
  const { runId } = await client.start(fresh.preparedId, 'key');
  assert.equal((await client.start(fresh.preparedId, 'key')).runId, runId);
  await assert.rejects(client.start(fresh.preparedId, 'other'), (error: ApiError) => error.code === 'Conflict');
  await assert.rejects(client.prepare([]), (error: ApiError) => error.code === 'Conflict');
  await assert.rejects(client.stop('f'.repeat(32)), (error: ApiError) => error.code === 'Conflict');
  assert.equal((await client.history({})).entries.filter(entry => entry.state === State.HISTORY_RUNNING).length, 1);
});

test('a live-only selection records nothing and still reports', async () => {
  // Arrange
  const client = createDemoClient();
  // Act
  const prepared = await review(client, [], undefined, LIVE_ONLY);
  const runId = await finishedRun(client, [], LIVE_ONLY);
  const { entry, status } = await client.historyRun(runId);
  // Assert
  assert.equal(prepared.recordings.length, 0);
  assert.equal(status!.recordings.length, 0);
  assert.equal(status!.outcome!.recordingFinalized, false);
  assert.deepEqual(entry!.artifacts.map(artifact => artifact.id), ['report']);
});

test('only the built-in run offers a recording; every finished run offers a matching report', async () => {
  // Arrange
  const client = createDemoClient();
  const runId = await finishedRun(client);
  // Act
  const recordingUrl = client.artifactUrl(BUILTIN_RUN_ID, 'cpu')!;
  const reportUrl = client.artifactUrl(runId, 'report')!;
  const report = JSON.parse(decodeURIComponent(reportUrl.split(',').slice(1).join(',')));
  const { entry } = await client.historyRun(runId);
  // Assert
  assert.match(recordingUrl, /^https:\/\/github\.com\/Cutuy\/doramcap\/releases\/download\/[^/]+\/cpu\.mcap$/);
  assert.equal(client.artifactUrl(runId, 'cpu'), undefined);
  assert.equal(client.artifactUrl('f'.repeat(32), 'report'), undefined);
  assert.deepEqual(entry!.artifacts.map(artifact => artifact.id), ['report']);
  assert.deepEqual(Object.keys(report), SERVER_REPORT_FIELDS);
  assert.equal(report.execution, 'Completed');
  assert.equal(report.recording_finalized, true);
});

test('history filters, pages and reports missing runs from the same stored facts', async () => {
  // Arrange
  const client = createDemoClient();
  const runId = await finishedRun(client);
  // Act
  const first = await client.history({ limit: 1 });
  const second = await client.history({ limit: 1, cursor: first.nextCursor });
  const failed = await client.history({ state: State.HISTORY_FAILED });
  // Assert
  assert.deepEqual(first.entries.map(entry => entry.runId), [runId]);
  assert.deepEqual(second.entries.map(entry => entry.runId), [BUILTIN_RUN_ID]);
  assert.equal(second.nextCursor, '');
  assert.equal(failed.entries.length, 0);
  await assert.rejects(client.historyRun('f'.repeat(32)), (error: ApiError) => error.code === 'Missing');
  await assert.rejects(client.historyCompare(runId, 'f'.repeat(32)), (error: ApiError) => error.code === 'Missing');
});

test('comparing runs and reviewing a selection give Same, Changed and Unavailable rows', async () => {
  // Arrange
  const client = createDemoClient();
  const runId = await finishedRun(client, [], LIVE_ONLY);
  // Act
  const comparison = await client.historyCompare(BUILTIN_RUN_ID, runId);
  const rerun = await review(client, [], undefined, LIVE_ONLY, BUILTIN_RUN_ID);
  const row = (rows: typeof comparison.rows, name: string) => rows.find(item => item.name === name)!;
  // Assert
  assert.deepEqual(comparison.sides.map(side => side.runs[0].entry!.runId), [BUILTIN_RUN_ID, runId]);
  assert.equal(row(comparison.rows, 'system_app_profile.cpu-monitor').result, Result.COMPARISON_CHANGED);
  assert.deepEqual(row(comparison.rows, 'system_app_profile.cpu-monitor').cells.map(cell => cell.values[0]), ['record', 'live']);
  assert.equal(row(comparison.rows, 'execution').result, Result.COMPARISON_SAME);
  assert.equal(row(comparison.rows, 'governor').group, Group.COMPARISON_CONFIGURATION);
  assert.equal(row(comparison.rows, 'governor').result, Result.COMPARISON_CHANGED);
  assert.equal(rerun.comparison!.baseline!.runId, BUILTIN_RUN_ID);
  const condition = rerun.comparison!.rows.filter(item => item.group === Group.COMPARISON_CONDITION);
  assert.ok(condition.length > 0);
  assert.ok(condition.every(item => item.result === Result.COMPARISON_UNAVAILABLE && item.cells[1].values[0]));
});

test('comparing an in-progress run reports it as In progress', async () => {
  // Arrange
  const client = createDemoClient();
  const prepared = await review(client);
  const { runId } = await client.start(prepared.preparedId, 'key');
  // Act
  const { rows } = await client.historyCompare(BUILTIN_RUN_ID, runId);
  const row = (name: string) => rows.find(item => item.name === name)!;
  // Assert
  assert.equal(row('execution').result, Result.COMPARISON_CHANGED);
  assert.deepEqual(row('execution').cells.map(cell => cell.values[0]), ['Completed', 'In progress']);
  assert.equal(row('duration_seconds').result, Result.COMPARISON_CHANGED);
  assert.deepEqual(row('duration_seconds').cells.map(cell => cell.values[0]), ['15.0', '']);
  assert.ok(rows.every(item => item.result !== Result.COMPARISON_UNAVAILABLE));
});

test('a run continues without a screen, and a returning screen rebuilds its panels', async () => {
  // Arrange
  const client = createDemoClient(), first = watch(client);
  const prepared = await review(client);
  await client.start(prepared.preparedId, 'key');
  await advance(4 * SECOND_MS);
  // Act: leave the Console section, then come back.
  first.controller.abort();
  await first.ended;
  const delivered = first.seen.length;
  await advance(4 * SECOND_MS);
  const returning = watch(client);
  // Assert
  assert.equal(first.seen.length, delivered);
  assert.equal(returning.last().phase, Phase.RUNNING);
  assert.equal(returning.last().runId, prepared.runId);
  assert.equal(returning.model.view().cpu!.history.length, 79);
  assert.equal(returning.model.view().gaps, 0);
  await advance(SAMPLE_MS);
  assert.equal(returning.model.view().cpu!.history.length, 80);
  assert.equal(first.seen.length, delivered);
});

test('dispose cancels pending work and ends live subscriptions', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  await client.start(prepared.preparedId, 'key');
  const delivered = () => screen.seen.length;
  // Act
  client.dispose();
  await screen.ended;
  const after = delivered();
  await advance(RUN_MS);
  // Assert
  assert.equal(delivered(), after);
});
