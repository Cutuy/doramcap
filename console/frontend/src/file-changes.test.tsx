import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { FileChanges } from './FileChanges';
import { Result } from './Result';
import { HistoryRun } from './History';
import { entryRecordingBytes } from './model';

const snapshot = () => wire.Status.fromObject({
  phase: wire.Phase.BLOCKED, runId: 'a'.repeat(32), outcome: { execution: 'Failed', recordingFinalized: false },
  fileChanges: {
    report: { checkpoint: 'filesystem/report.pb', categories: [{ category: 1, counts: { added: '3000', modified: '2', deleted: '1' }, coverageComplete: false, coverageNoteCount: '4' }] },
    downloadable: true, previewIncomplete: true, notesIncomplete: true, coverageNotes: ['unreadable source: permission denied'],
    preview: [{ category: 1, kind: 1, source: '/source', path: new TextEncoder().encode('<script>evil()</script>'), consumers: ['project/writer:/out'], after: { size: '9223372036854775807', mode: 33152 } }],
  },
});

test('current and past failed runs share counts, coverage and a file download independent of recording validity', () => {
  // Arrange
  const status = snapshot();
  // Act
  const current = renderToStaticMarkup(<Result snapshot={status} newRun={() => {}}/>);
  const history = renderToStaticMarkup(<HistoryRun detail={wire.HistoryDetail.create({ status, entry: wire.HistoryEntry.create({ runId: status.runId, state: wire.HistoryState.HISTORY_FAILED }) })} runAgain={() => {}} runActive={false}/>);
  // Assert
  for (const html of [current, history]) {
    assert.match(html, /3000 added · 2 modified · 1 deleted/);
    assert.match(html, /Preview: 1 of 3003 collected changes · Preview incomplete/);
    assert.match(html, /Incomplete observation/);
    assert.match(html, /More coverage notes/);
    assert.match(html, /Download file changes/);
    assert.match(html, /file-changes.jsonl/);
    assert.match(html, /metadata unavailable/);
    assert.match(html, /&lt;script&gt;evil\(\)&lt;\/script&gt;/);
    assert.doesNotMatch(html, /<script>|href="\/source|href="&lt;script/);
  }
});

test('old runs, failed reports and complete zero changes render distinctly', () => {
  // Arrange
  const absent = wire.Status.create();
  const failed = wire.Status.create({ fileChanges: wire.FileChanges.create({ error: 'File changes were not collected' }) });
  const zero = wire.Status.fromObject({ fileChanges: { report: { categories: [{ category: 1, counts: {} }] }, downloadable: true } });
  // Act / Assert
  assert.match(renderToStaticMarkup(<FileChanges snapshot={absent}/>), /unavailable for this run/);
  assert.match(renderToStaticMarkup(<FileChanges snapshot={failed}/>), /role="alert"/);
  assert.match(renderToStaticMarkup(<FileChanges snapshot={zero}/>), /No changes observed/);
  assert.doesNotMatch(renderToStaticMarkup(<FileChanges snapshot={absent}/>), /No changes observed/);
});

test('artifact kinds exclude file reports from recording byte totals and preserve old entries', () => {
  // Arrange / Act
  const entry = wire.HistoryEntry.create({ artifacts: [
    wire.HistoryArtifact.create({ id: 'cpu', sizeBytes: 123 }),
    wire.HistoryArtifact.create({ id: 'report', sizeBytes: 222 }),
    wire.HistoryArtifact.create({ id: 'file-changes', sizeBytes: 999, kind: wire.ArtifactKind.FILE_CHANGES }),
  ] });
  // Assert
  assert.equal(entryRecordingBytes(entry), 123);
});


test('a BOM-only filename is displayed as path bytes rather than the source root', () => {
  // Arrange
  const status = snapshot();
  // Act / Assert
  for (const [name, shown] of [['name', 'name'], ['\ufeffname', '\\xef\\xbb\\xbfname'], ['\ufeff', '\\xef\\xbb\\xbf']]) {
    status.fileChanges!.preview[0].path = new TextEncoder().encode(name);
    const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
    assert.ok(html.includes(`>${shown}</p>`));
    assert.doesNotMatch(html, /\(source root\)/);
  }
});
