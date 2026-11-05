import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createClient, routes, wire, workflow, Phase, StepState, type HistoryPage } from './api';
import { MEDIA_TYPE, artifactPath } from './generated/transport';
import { HistoryRun, HistoryTable } from './History';
import { ReviewPreview } from './Comparison';
import { ConsoleContext } from './context';
import { compareHash, durationLabel, entryRecordingBytes, factLabel, factValue, historyHash, historyRoute, runHash, toggleCompared } from './model';
import { Result } from './Result';
import { PageFrame } from './ui/PageFrame';
import { RunMetrics } from './RunMetrics';

const { HistoryState: State } = wire;
const runId = 'a'.repeat(32);
const entry = (fields: object = {}) => wire.HistoryEntry.fromObject({
  runId, profiles: ['gpu'], systemApps: { ids: ['cpu-monitor'] },
  startedAtMs: 1_000_000, finishedAtMs: 1_125_000, state: State.HISTORY_COMPLETED,
  artifacts: [{ id: 'cpu', sizeBytes: 2 * 1024 * 1024, downloadable: true }, { id: 'report', sizeBytes: 100, downloadable: true }],
  ...fields,
});
const table = (page: HistoryPage | null, fields: Partial<Parameters<typeof HistoryTable>[0]> = {}) => renderToStaticMarkup(
  <HistoryTable page={page} loading={false} error="" state={0} setState={() => {}} more={() => {}} {...fields}/>);
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

test('rows show state, duration, recording size, a detail link and paging', () => {
  // Arrange
  const page = wire.HistoryPage.create({ entries: [entry(), entry({ runId: 'b'.repeat(32), state: State.HISTORY_INTERRUPTED, finishedAtMs: null, errorSummary: 'Console stopped', artifacts: [] })], nextCursor: `1:${runId}` });
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
  assert.deepEqual(historyRoute('#/history'), { runId: '', compare: [] });
  assert.deepEqual(historyRoute(historyHash(runId)), { runId, compare: [] });
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
  const page = await client.history({ state: State.HISTORY_FAILED, cursor: `1:${runId}` });
  const detail = await client.historyRun(runId);
  // Assert
  assert.equal(calls[0].path, routes.history.path);
  const sent = wire.HistoryQuery.decode(calls[0].body);
  assert.equal(sent.state, State.HISTORY_FAILED);
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

const other = 'b'.repeat(32);
test('the comparison selection lives in the URL, holds two runs and survives a reload', () => {
  // Arrange / Act
  const first = toggleCompared([], runId), both = toggleCompared(first, other);
  // Assert
  assert.deepEqual(first, [runId]);
  assert.deepEqual(both, [runId, other]);
  assert.deepEqual(toggleCompared(both, 'c'.repeat(32)), both);
  assert.deepEqual(toggleCompared(both, runId), [other]);
  assert.deepEqual(historyRoute(compareHash(both)), { runId: '', compare: both });
  assert.deepEqual(historyRoute(compareHash(first)), { runId: '', compare: first });
  assert.deepEqual(historyRoute(`#/history/compare/${runId}/${runId}`), { runId: '', compare: [runId] });
  assert.equal(compareHash([]), historyHash());
  assert.equal(historyRoute('#/history/compare/short'), null);
  assert.deepEqual(historyRoute(historyHash()), { runId: '', compare: [] });
});

test('compared facts read by their unit, and exact names stay exact', () => {
  // Arrange / Act / Assert
  assert.equal(factLabel('memory_available_bytes'), 'Memory available');
  assert.equal(factLabel('cpu_model'), 'CPU model');
  assert.equal(factLabel('environment.worker.FIXED_DT'), 'environment.worker.FIXED_DT');
  assert.equal(factValue('disk_free_bytes', String(2 * 1024 ** 3)), '2.0 GiB');
  assert.equal(factValue('uptime_seconds', '172800'), '2.0 days');
  assert.equal(factValue('max_temperature_celsius', '49'), '49 °C');
  assert.equal(factValue('kernel_release', '7.0'), '7.0');
});

test('an embedded host keeps every route and link beneath its base path', () => {
  // Arrange
  const base = '/console', a = 'a'.repeat(32), b = 'b'.repeat(32);
  // Act
  const hashes = [historyHash('', base), historyHash(a, base), compareHash([a], base), compareHash([a, b], base)];
  // Assert
  assert.deepEqual(hashes.map(hash => historyRoute(hash, base)), [
    { runId: '', compare: [] }, { runId: a, compare: [] }, { runId: '', compare: [a] }, { runId: '', compare: [a, b] },
  ]);
  assert.equal(runHash(base), '#/console');
  assert.equal(historyRoute('#/console', base), null);
  assert.equal(historyRoute('#/history', base), null);
  assert.equal(historyRoute('#/consoles/history', base), null);
});

test('a client without a file for an artifact shows no download link', () => {
  // Arrange
  const client = { ...createClient(async () => new Response()), artifactUrl: (_run: string, id: string) => id === 'report' ? 'data:application/json,{}' : undefined };
  // Act
  const html = renderToStaticMarkup(<ConsoleContext.Provider value={{ client, basePath: '' }}>
    <Result snapshot={status()} newRun={() => {}}/>
  </ConsoleContext.Provider>);
  // Assert
  assert.match(html, /cpu recording unavailable/);
  assert.doesNotMatch(html, /Download cpu recording/);
  assert.match(html, /href="data:application\/json,\{\}" download="console-report.json"/);
});

test('embedded screens leave the page header and main landmark to the host', () => {
  // Arrange
  const client = createClient(async () => new Response());
  // Act
  const html = renderToStaticMarkup(<ConsoleContext.Provider value={{ client, basePath: '/console' }}>
    <PageFrame phase="Project">Body</PageFrame>
  </ConsoleContext.Provider>);
  // Assert
  assert.doesNotMatch(html, /<main|<header/);
  assert.match(html, /href="#\/console"[^>]*>Run/);
  assert.match(html, /href="#\/console\/history"[^>]*>History/);
});

test('matching comparison facts collapse behind a control that expands them', () => {
  // Arrange
  const row = (name: string, result: number) => wire.ComparisonRow.create({ group: wire.ComparisonGroup.COMPARISON_SETUP, name, result, cells: [wire.ComparisonCell.create({ values: ['x'] }), wire.ComparisonCell.create({ values: ['y'] })] });
  const comparison = wire.ReviewComparison.create({ baseline: entry(), rows: [row('profiles', wire.ComparisonResult.COMPARISON_CHANGED), row('source_state', wire.ComparisonResult.COMPARISON_SAME)] });
  // Act
  const html = renderToStaticMarkup(<ReviewPreview comparison={comparison}/>);
  // Assert
  assert.match(html, /<button[^>]*class="comparison-matching"[^>]*aria-expanded="false"/);
  assert.match(html, /1 matching/);
  assert.match(html, /Profiles/);
  assert.doesNotMatch(html, /Source state/);
});

test('the view tabs and run phases share one navigation row', () => {
  // Arrange
  const client = createClient(async () => new Response());
  // Act
  const embedded = renderToStaticMarkup(<ConsoleContext.Provider value={{ client, basePath: '/console' }}><PageFrame phase="Run">Body</PageFrame></ConsoleContext.Provider>);
  const standalone = renderToStaticMarkup(<PageFrame phase="Run">Body</PageFrame>);
  // Assert
  assert.match(embedded, /class="console-nav"><nav[^>]*Console views.*<nav[^>]*Run phases/);
  assert.match(standalone, /<header[^>]*>.*Console views.*Run phases.*<\/header><main/);
  assert.doesNotMatch(embedded + standalone, /<small>/);
});
