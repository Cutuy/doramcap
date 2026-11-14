import assert from 'node:assert/strict';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { Result } from './Result';
import { HistoryRun } from './History';

test('Result and History expose independent log states, sizes and failures', () => {
  // Arrange
  const status = wire.Status.fromObject({ phase: wire.Phase.FINISHED, runId: 'a'.repeat(32), outcome: { execution: 'Failed' }, logs: { programs: [
    { programId: 'camera', state: 2, sizeBytes: '2048', entryCount: '3' },
    { programId: 'project', state: 3, reason: 'Logging configured by project' },
    { programId: '<script>failure</script>', state: 4, sizeBytes: '7', reason: 'Journal unavailable' },
    { programId: 'interrupted', state: 1 },
  ] } });
  // Act
  const current = renderToStaticMarkup(<Result snapshot={status} newRun={() => {}}/>);
  const history = renderToStaticMarkup(<HistoryRun detail={wire.HistoryDetail.create({ status, entry: wire.HistoryEntry.create({ runId: status.runId, state: wire.HistoryState.HISTORY_FAILED }) })} runAgain={() => {}} runActive={false}/>);
  // Assert
  for (const html of [current, history]) {
    for (const text of ['Logs', 'camera', 'Exported', '2.0 KiB', 'Skipped', 'Failed', 'Pending', 'Journal unavailable', 'entries still available in the host journal']) assert.ok(html.includes(text), text);
    assert.match(html, /&lt;script&gt;failure&lt;\/script&gt;/);
    assert.doesNotMatch(html, /<script>|<details|Download.*log/);
  }
});

test('missing and unreadable indexes remain distinct', () => {
  // Arrange / Act
  const missing = renderToStaticMarkup(<Result snapshot={wire.Status.create()} newRun={() => {}}/>);
  const failed = renderToStaticMarkup(<Result snapshot={wire.Status.create({ logsError: 'Logs unavailable: invalid index' })} newRun={() => {}}/>);
  // Assert
  assert.match(missing, /Logs unavailable for this run/);
  assert.match(failed, /role="alert">Logs unavailable: invalid index/);
});
