import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, mock, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { McapStreamReader } from '@mcap/core';
import protobuf from 'protobufjs';
import 'protobufjs/ext/descriptor/index.js';
import { ApiError, Phase, wire, type Snapshot } from './api';
import { BUILTIN_RECORDING_BYTES, BUILTIN_RUN_ID, BUILTIN_STARTED_AT_MS, createDemoClient } from './demo/client';
import { RUN_SAMPLES } from './demo/telemetry';
import { PREPARING_RUN } from './generated/transport';
import { LiveModel } from './streams';

const { ComparisonGroup: Group, ComparisonResult: Result, HistoryState: State } = wire;
const PREPARE_MS = 500, SAMPLE_MS = 1000, RUN_MS = RUN_SAMPLES * SAMPLE_MS;
const LIVE_ONLY = { 'cpu-monitor': 'live' };
const SERVER_REPORT_FIELDS = ['execution', 'error', 'writers_stopped', 'recording_finalized', 'service_exits', 'boundary_clean', 'event_stream_healthy', 'system_apps'];

const realFetch = globalThis.fetch;
beforeEach(() => {
  mock.timers.enable({ apis: ['setTimeout'] });
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

test('the bundled recording is a valid capture matching the built-in run', () => {
  // Arrange
  const file = readFileSync(fileURLToPath(new URL('./demo/builtin-cpu.mcap', import.meta.url)));
  const reader = new McapStreamReader({ includeChunks: true, validateCrcs: true });
  let root: protobuf.Root | undefined, name = '';
  const messages: { logTime: bigint; data: Uint8Array }[] = [];
  let footer = false;
  // Act
  reader.append(file);
  for (let record; (record = reader.nextRecord());) {
    if (record.type === 'Schema') { root = (protobuf.Root as unknown as { fromDescriptor(data: Uint8Array): protobuf.Root }).fromDescriptor(record.data); name = record.name; }
    else if (record.type === 'Message') messages.push(record);
    else if (record.type === 'Footer') footer = true;
  }
  const Packet = root!.lookupType(name);
  const first = Packet.toObject(Packet.decode(messages[0].data), { longs: String }) as { snapshot: { dictionary: { path: string }[] } };
  // Assert
  assert.ok(footer);
  assert.equal(file.length, BUILTIN_RECORDING_BYTES);
  assert.equal(name, 'doramcap.telemetry.v1.Packet');
  assert.equal(messages.length, RUN_SAMPLES);
  assert.equal(messages[0].logTime, BigInt(BUILTIN_STARTED_AT_MS) * 1_000_000n + 1_000_000_000n);
  assert.equal(messages.at(-1)!.logTime - messages[0].logTime, BigInt(RUN_SAMPLES - 1) * 1_000_000_000n);
  assert.deepEqual(first.snapshot.dictionary.map(entry => entry.path), ['/docker/activity-lab']);
});

test('a run progresses through its phases to Completed and joins History', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  // Act
  const { runId } = await client.start(prepared.preparedId, 'key');
  await advance(SAMPLE_MS);
  const starting = screen.last().phase;
  await advance(5 * SAMPLE_MS);
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
  assert.equal(screen.model.view().cpu!.groups[0].name, 'activity-lab');
  assert.deepEqual((await client.history({})).entries.map(entry => entry.runId), [runId, BUILTIN_RUN_ID]);
});

test('Stop ends a run early as Completed', async () => {
  // Arrange
  const client = createDemoClient(), screen = watch(client);
  const prepared = await review(client);
  const { runId } = await client.start(prepared.preparedId, 'key');
  await advance(5 * SAMPLE_MS);
  // Act
  await client.stop(runId);
  const stopping = screen.last().phase;
  await advance(2 * SAMPLE_MS);
  // Assert
  assert.equal(stopping, Phase.STOPPING);
  assert.equal(screen.last().phase, Phase.FINISHED);
  assert.equal(screen.last().outcome!.execution, 'Completed');
  const detail = await client.historyRun(runId);
  assert.equal(Number(detail.entry!.finishedAtMs) - Number(detail.entry!.startedAtMs), 7 * SAMPLE_MS);
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
  assert.match(recordingUrl, /builtin-cpu\.mcap$/);
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
  const runId = await finishedRun(client, ['load']);
  // Act
  const comparison = await client.historyCompare(BUILTIN_RUN_ID, runId);
  const rerun = await review(client, ['load'], undefined, undefined, BUILTIN_RUN_ID);
  const row = (rows: typeof comparison.rows, name: string) => rows.find(item => item.name === name)!;
  // Assert
  assert.deepEqual(comparison.sides.map(side => side.runs[0].entry!.runId), [BUILTIN_RUN_ID, runId]);
  assert.equal(row(comparison.rows, 'profiles').result, Result.COMPARISON_CHANGED);
  assert.deepEqual(row(comparison.rows, 'profiles').cells.map(cell => cell.values[0]), ['Default', 'load']);
  assert.equal(row(comparison.rows, 'execution').result, Result.COMPARISON_SAME);
  assert.equal(row(comparison.rows, 'governors').group, Group.COMPARISON_CONFIGURATION);
  assert.equal(row(comparison.rows, 'governors').result, Result.COMPARISON_CHANGED);
  assert.equal(rerun.comparison!.baseline!.runId, BUILTIN_RUN_ID);
  const condition = rerun.comparison!.rows.filter(item => item.group === Group.COMPARISON_CONDITION);
  assert.ok(condition.length > 0);
  assert.ok(condition.every(item => item.result === Result.COMPARISON_UNAVAILABLE && item.cells[1].values[0]));
});

test('a run continues without a screen, and a returning screen rebuilds its panels', async () => {
  // Arrange
  const client = createDemoClient(), first = watch(client);
  const prepared = await review(client);
  await client.start(prepared.preparedId, 'key');
  await advance(4 * SAMPLE_MS);
  // Act: leave the Console section, then come back.
  first.controller.abort();
  await first.ended;
  const delivered = first.seen.length;
  await advance(4 * SAMPLE_MS);
  const returning = watch(client);
  // Assert
  assert.equal(first.seen.length, delivered);
  assert.equal(returning.last().phase, Phase.RUNNING);
  assert.equal(returning.last().runId, prepared.runId);
  assert.equal(returning.model.view().cpu!.history.length, 7);
  assert.equal(returning.model.view().gaps, 0);
  await advance(SAMPLE_MS);
  assert.equal(returning.model.view().cpu!.history.length, 8);
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
