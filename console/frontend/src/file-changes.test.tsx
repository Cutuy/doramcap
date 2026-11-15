import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { FileChanges } from './FileChanges';
import { Result } from './Result';
import { HistoryRun } from './History';
import { entryRecordingBytes } from './model';
import { doramcap } from './generated/console';

const snapshot = () => wire.Status.fromObject({
  phase: wire.Phase.BLOCKED, runId: 'a'.repeat(32), outcome: { execution: 'Failed', recordingFinalized: false },
  fileChanges: {
    report: { checkpoint: 'filesystem/report.pb', categories: [{ category: 1, counts: { added: '3000', modified: '2', deleted: '1' }, coverageComplete: false, coverageNoteCount: '4' }] },
    downloadable: true, previewIncomplete: true, notesIncomplete: true, coverageNotes: ['unreadable source: permission denied'],
    preview: [{ category: 1, kind: 1, source: '/source', path: new TextEncoder().encode('<script>evil()</script>'), consumers: ['project/writer:/out'], after: { size: '9223372036854775807', mode: 33152 } }],
  },
});

test('source association labels stay separate from run-file location and retain unknown legacy records', () => {
  // Arrange: all rows deliberately share a source; associations can differ for nested mounts.
  const { AppScope, SourceKind } = doramcap.filesystem.v1;
  const scopes = [AppScope.APP_SCOPE_USER, AppScope.APP_SCOPE_SYSTEM, AppScope.APP_SCOPE_USER_AND_SYSTEM, AppScope.APP_SCOPE_UNSPECIFIED];
  const status = snapshot();
  status.fileChanges!.preview = scopes.map((appScope, index) => doramcap.filesystem.v1.FileChange.create({
    category: 1, source: '/console/runs/customer-artifacts', appScope,
    sourceKind: index === 3 ? SourceKind.SOURCE_KIND_UNSPECIFIED : SourceKind.SOURCE_KIND_RUN_OUTPUT,
    kind: 1, path: new TextEncoder().encode(`retained-${index}`),
  }));
  status.fileChanges!.preview.push(doramcap.filesystem.v1.FileChange.create({
    ...status.fileChanges!.preview[0], path: new TextEncoder().encode('retained-4'), consumers: ['nested-reader:/in'],
  }));
  // Act
  const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
  // Assert: no record is hidden or attributed by its path.
  assert.match(html, /Run files · Used by User apps/);
  assert.match(html, /Run files · Used by System apps/);
  assert.match(html, /Run files · Used by User and System apps/);
  assert.match(html, /Host folder · App association unknown/);
  for (let index = 0; index < 5; index++) assert.ok(html.includes(`retained-${index}`));
  const groups = [...html.matchAll(/<tbody>(.*?)<\/tbody>/gs)].map(match => match[1]);
  assert.ok(groups.find(group => group.includes('retained-4'))!.includes('nested-reader:/in'));
  assert.ok(!groups.find(group => group.includes('retained-0'))!.includes('nested-reader:/in'));
  assert.match(html, /Showing 5 of 3003/);
});

test('interleaved associations retain saved row order and download positions', () => {
  // Arrange: a nested system alias changes the association between two user rows.
  const status = snapshot();
  status.fileChanges!.preview = [1, 3, 1].map((appScope, index) => doramcap.filesystem.v1.FileChange.create({
    category: 1, source: '/same', appScope, sourceKind: 2, kind: 1,
    path: new TextEncoder().encode(`ordered-${index}`), consumers: appScope === 3 ? ['user:/out', 'system:/in'] : ['user:/out'],
  }));
  status.fileChanges!.previewPositions = [0, 1, 2];
  status.fileChanges!.downloadablePositions = [0, 1, 2];
  // Act
  const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
  // Assert: headers can repeat; each saved row keeps its order and original download.
  assert.deepEqual([...html.matchAll(/>ordered-([0-9])/g)].map(match => Number(match[1])), [0, 1, 2]);
  assert.deepEqual([...html.matchAll(/run-file\.download\?change=([0-9]+)/g)].map(match => Number(match[1])), [0, 1, 2]);
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
    assert.match(html, /Showing 1 of 3003 host folder changes · Preview incomplete/);
    assert.match(html, /Incomplete observation/);
    assert.match(html, /More coverage notes/);
    assert.match(html, /Download file changes/);
    assert.ok(html.includes(`href="/api/artifacts/${status.runId}/file-changes.jsonl"`));
    assert.match(html, /download="file-changes.jsonl"/);
    assert.match(html, /0600 · 0:0/);
    assert.doesNotMatch(html, /<details|<summary/);
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
    wire.HistoryArtifact.create({ id: 'file-changes.jsonl', sizeBytes: 999, kind: wire.ArtifactKind.FILE_CHANGES }),
  ] });
  // Assert: absent kinds retain legacy recording behavior; unknown future values do not.
  const future = wire.HistoryArtifact.decode(wire.HistoryArtifact.encode(wire.HistoryArtifact.fromObject({ id: 'future', sizeBytes: 9999, kind: 99 })).finish());
  assert.equal(future.kind, 99);
  entry.artifacts.push(future);
  assert.equal(entryRecordingBytes(entry), 123);
});


test('a BOM-only filename is displayed as path bytes rather than the source root', () => {
  // Arrange
  const status = snapshot();
  // Act / Assert
  for (const [name, shown] of [['name', 'name'], ['\ufeffname', '\\xef\\xbb\\xbfname'], ['\ufeff', '\\xef\\xbb\\xbf']]) {
    status.fileChanges!.preview[0].path = new TextEncoder().encode(name);
    const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
    assert.ok(html.includes(`>${shown}</td>`));
    assert.doesNotMatch(html, /\(source root\)/);
  }
});

test('modified rows flag changed identity fields and deleted or metadata-free rows keep their remaining facts', () => {
  // Arrange
  const status = snapshot();
  const file = (mode: number, extra = {}) => ({ mode, size: '112', uid: 1000, gid: 1000, mtimeSeconds: '10', ...extra });
  const path = (name: string) => new TextEncoder().encode(name);
  status.fileChanges!.preview = wire.FileChanges.fromObject({ preview: [
    { category: 1, kind: 2, source: '/source', path: path('a'), before: file(0o100644), after: file(0o100600, { size: '140', mtimeSeconds: '31' }) },
    { category: 1, kind: 3, source: '/source', path: path('b'), before: file(0o100644) },
    { category: 1, kind: 2, source: '/source', path: path('engine-only') },
  ] }).preview;
  // Act
  const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
  // Assert
  assert.match(html, /class="file-change-fact changed">mode 0644 → 0600</);
  assert.match(html, /112 B → 140 B/);
  assert.match(html, /mtime \+21 s/);
  assert.match(html, /was 0644 · 1000:1000/);
  assert.match(html, /metadata unavailable/);
  assert.match(html, /aria-pressed="true"[^>]*>All 3</);
  assert.match(html, /Modified 2/);
});

test('category tiles select one category, opening on the first with changes', () => {
  // Arrange
  const status = wire.Status.fromObject({ runId: 'a'.repeat(32), fileChanges: { downloadable: true,
    report: { categories: [
      { category: 1, counts: {}, coverageComplete: true },
      { category: 2, counts: { added: '5', modified: '35' }, coverageComplete: true },
      { category: 3, counts: {}, coverageComplete: true },
    ] },
    preview: [
      { category: 2, kind: 1, source: 'layer-a', path: new TextEncoder().encode('/tmp/new') },
      { category: 2, kind: 2, source: 'layer-a', path: new TextEncoder().encode('/usr') },
      { category: 1, kind: 1, source: '/host', path: new TextEncoder().encode('stray-host-row') },
    ] } });
  // Act
  const html = renderToStaticMarkup(<FileChanges snapshot={status}/>);
  // Assert
  assert.equal(html.match(/<button type="button" class="file-change-tile[^"]*" aria-pressed="(true|false)"/g)?.length, 3);
  assert.match(html, /aria-pressed="true"><span class="file-change-category">Container layers</);
  assert.match(html, /aria-label="Container layers change preview"/);
  assert.match(html, />\/tmp\/new</);
  assert.doesNotMatch(html, /stray-host-row/);
  assert.match(html, /All 2</);
  assert.match(html, /Showing 2 of 40 container layer changes · Preview incomplete/);
});
