import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createClient, routes, wire, workflow, Phase, StepState, type HistoryPage } from './api';
import { MEDIA_TYPE, artifactPath } from './generated/transport';
import { HistoryRun, HistoryTable } from './History';
import { durationLabel, entryRecordingBytes, historyHash, historyRoute } from './model';
import { RunMetrics } from './RunMetrics';

const { HistoryState: State } = wire;
const runId = 'a'.repeat(32);
const entry = (fields: object = {}) => wire.HistoryEntry.fromObject({
  runId, projectDirectory: '/projects/robot', profiles: ['gpu'], systemApps: { ids: ['cpu-monitor'] },
  startedAtMs: 1_000_000, finishedAtMs: 1_125_000, state: State.HISTORY_COMPLETED,
  artifacts: [{ id: 'cpu', sizeBytes: 2 * 1024 * 1024, downloadable: true }, { id: 'report', sizeBytes: 100, downloadable: true }],
  ...fields,
});
const table = (page: HistoryPage | null, fields: Partial<Parameters<typeof HistoryTable>[0]> = {}) => renderToStaticMarkup(
  <HistoryTable page={page} loading={false} error="" state={0} project="" setState={() => {}} setProject={() => {}} more={() => {}} {...fields}/>);
const status = (fields: object = {}) => wire.Status.fromObject({
  phase: Phase.FINISHED, runId, recordings: [{ id: 'cpu', path: 'cpu.mcap', cpu: true }],
  outcome: { execution: 'Completed', recordingFinalized: true, boundaryClean: true }, reportAvailable: true,
  steps: [workflow.Event.toObject(workflow.Event.fromObject({ session: 's', executor: 'local', invocation: '1', step: 'compose.launch', displayName: 'Start project', state: StepState.SUCCEEDED, elapsedNs: '2000000000', durationNs: '1000000000' }))],
  ...fields,
});

test('the table shows loading, first-use, filtered-empty and failure states', () => {
  // Arrange / Act
  const loading = table(null, { loading: true });
  const empty = table(wire.HistoryPage.create());
  const filtered = table(wire.HistoryPage.create(), { state: State.HISTORY_FAILED });
  const failed = table(null, { error: 'History could not be loaded. Try again.' });
  // Assert
  assert.match(loading, /Loading history/);
  assert.match(empty, /No runs yet/);
  assert.match(filtered, /No runs match these filters/);
  assert.doesNotMatch(filtered, /No runs yet/);
  assert.match(failed, /role="alert"[^>]*>History could not be loaded/);
  assert.doesNotMatch(failed, /No runs/);
});

test('a degraded index says runs are unaffected and does not claim the history is empty', () => {
  // Arrange / Act
  const html = table(wire.HistoryPage.create({ degraded: 'database disk image is malformed', unreadable: 2 }));
  // Assert
  assert.match(html, /History is degraded: database disk image is malformed/);
  assert.match(html, /Runs still start, stop and recover/);
  assert.match(html, /2 saved runs could not be read/);
  assert.doesNotMatch(html, /No runs yet/);
});

test('rows show state, project, duration, recording size, a detail link and paging', () => {
  // Arrange
  const page = wire.HistoryPage.create({ entries: [entry(), entry({ runId: 'b'.repeat(32), state: State.HISTORY_INTERRUPTED, finishedAtMs: null, errorSummary: 'Console stopped', artifacts: [] })], nextCursor: `1:${runId}`, projects: ['/projects/robot'] });
  // Act
  const html = table(page);
  // Assert
  assert.match(html, /Completed/);
  assert.match(html, /Interrupted/);
  assert.match(html, /2 min 5 s/);
  assert.match(html, /Unknown/);
  assert.match(html, /2\.0 MiB/);
  assert.match(html, /Console stopped/);
  assert.ok(html.includes(`href="${historyHash(runId)}"`));
  assert.match(html, /<option value="\/projects\/robot">/);
  assert.match(html, /Load more/);
});

test('a completed run reuses timeline and result views with sized downloads and run again', () => {
  // Arrange
  const detail = wire.HistoryDetail.create({ entry: entry(), status: status() });
  // Act
  const html = renderToStaticMarkup(<HistoryRun detail={detail} runAgain={() => {}} runActive={false}/>);
  // Assert
  assert.match(html, /Run completed/);
  assert.match(html, /Run outcomes/);
  assert.match(html, /Activity timeline/);
  assert.match(html, /Start project/);
  assert.ok(html.includes(`href="${artifactPath(runId, 'cpu')}"`));
  assert.match(html, /Download cpu recording · 2\.0 MiB/);
  assert.match(html, /Download report/);
  assert.match(html, /<button[^>]*>Run again<\/button>/);
  assert.doesNotMatch(html, /<button[^>]*disabled[^>]*>Run again/);
  assert.match(html, /cpu-monitor/);
  assert.doesNotMatch(html, /Reset required/);
  assert.doesNotMatch(html, /CPU total/);
});

test('an interrupted run admits what it does not know and offers no recording download', () => {
  // Arrange
  const detail = wire.HistoryDetail.create({
    entry: entry({ state: State.HISTORY_INTERRUPTED, finishedAtMs: null, errorSummary: 'Console stopped before this run reported a result.', artifacts: [{ id: 'cpu', sizeBytes: 1024, downloadable: false }] }),
    status: status({ phase: Phase.RUNNING, outcome: null, reportAvailable: false }),
  });
  // Act
  const html = renderToStaticMarkup(<HistoryRun detail={detail} runAgain={() => {}} runActive={false}/>);
  // Assert
  assert.match(html, /Run interrupted/);
  assert.doesNotMatch(html, /Run complete/);
  assert.match(html, /Interrupted run/);
  assert.match(html, /cpu recording unavailable · 1\.0 KiB left on host/);
  assert.doesNotMatch(html, /Download cpu recording/);
  assert.match(html, /Console stopped before/);
});

test('run again is disabled while another run is active', () => {
  // Arrange
  const detail = wire.HistoryDetail.create({ entry: entry(), status: status() });
  // Act
  const html = renderToStaticMarkup(<HistoryRun detail={detail} runAgain={() => {}} runActive/>);
  // Assert
  assert.match(html, /<button[^>]*disabled[^>]*>Run again/);
});

test('a run missing its file on disk stops offering the download', () => {
  // Arrange
  const detail = wire.HistoryDetail.create({ entry: entry({ artifacts: [{ id: 'cpu', downloadable: false }] }), status: status() });
  // Act
  const html = renderToStaticMarkup(<HistoryRun detail={detail} runAgain={() => {}} runActive={false}/>);
  // Assert
  assert.match(html, /cpu recording unavailable/);
  assert.doesNotMatch(html, /Download cpu recording/);
});

test('metrics can omit the CPU panels a stored run has no frames for', () => {
  // Arrange / Act
  const stored = renderToStaticMarkup(<RunMetrics historical recordingOnly/>);
  const current = renderToStaticMarkup(<RunMetrics historical/>);
  // Assert
  assert.doesNotMatch(stored, /CPU total/);
  assert.match(current, /CPU total/);
});

test('routes accept only history hashes with strict run ids', () => {
  // Arrange / Act / Assert
  assert.deepEqual(historyRoute('#/history'), { runId: '' });
  assert.deepEqual(historyRoute(historyHash(runId)), { runId });
  for (const other of ['', '#/', '#/history/', '#/history/../x', `#/history/${runId.toUpperCase()}`, `#/history/${runId}0`])
    assert.equal(historyRoute(other), null, other);
});

test('duration and total size handle running, interrupted and absent values', () => {
  // Arrange / Act / Assert
  assert.equal(durationLabel(entry({ state: State.HISTORY_RUNNING, finishedAtMs: null })), 'In progress');
  assert.equal(durationLabel(entry({ state: State.HISTORY_INTERRUPTED, finishedAtMs: null })), 'Unknown');
  assert.equal(durationLabel(entry({ finishedAtMs: 1_012_345 })), '12.3 s');
  assert.equal(durationLabel(entry({ finishedAtMs: 1_000_000 + 3_725_000 })), '1 h 2 min');
  assert.equal(entryRecordingBytes(entry()), 2 * 1024 * 1024);
  assert.equal(entryRecordingBytes(entry({ artifacts: [{ id: 'report', sizeBytes: 9 }] })), null);
});

test('the client sends the typed query and decodes the typed responses', async () => {
  // Arrange
  const calls: { path: string; method: string; body: Uint8Array }[] = [];
  const reply = (bytes: Uint8Array) => new Response(new Blob([new Uint8Array(bytes)]), { status: 200, headers: { 'content-type': MEDIA_TYPE } });
  const client = createClient((async (path: string, init: RequestInit) => {
    calls.push({ path, method: init.method!, body: init.body ? new Uint8Array(await (init.body as Blob).arrayBuffer()) : new Uint8Array() });
    return init.method === 'POST'
      ? reply(wire.HistoryPage.encode(wire.HistoryPage.create({ entries: [entry()], nextCursor: `1:${runId}` })).finish())
      : reply(wire.HistoryDetail.encode(wire.HistoryDetail.create({ entry: entry(), status: status() })).finish());
  }) as typeof fetch);
  // Act
  const page = await client.history({ state: State.HISTORY_FAILED, project: '/projects/robot', cursor: `1:${runId}` });
  const detail = await client.historyRun(runId);
  // Assert
  assert.equal(calls[0].path, routes.history.path);
  const sent = wire.HistoryQuery.decode(calls[0].body);
  assert.equal(sent.state, State.HISTORY_FAILED);
  assert.equal(sent.project, '/projects/robot');
  assert.equal(page.entries[0].runId, runId);
  assert.equal(calls[1].path, `/api/history/${runId}`);
  assert.equal(calls[1].method, 'GET');
  assert.equal(detail.status!.runId, runId);
});

test('a run still in progress links to the live view without stale activity', () => {
  // Arrange
  const status = wire.Status.fromObject({ phase: Phase.RUNNING, runId: 'a'.repeat(32), steps: [] });
  const running = wire.HistoryDetail.create({ entry: entry({ state: State.HISTORY_RUNNING }), status });
  const interrupted = wire.HistoryDetail.create({ entry: entry({ state: State.HISTORY_INTERRUPTED }), status });
  // Act
  const live = renderToStaticMarkup(<HistoryRun detail={running} runAgain={() => {}} runActive/>);
  const stopped = renderToStaticMarkup(<HistoryRun detail={interrupted} runAgain={() => {}} runActive={false}/>);
  // Assert
  assert.match(live, /View live run/);
  assert.doesNotMatch(live, /Interrupted run/);
  assert.match(stopped, /Interrupted run/);
});
