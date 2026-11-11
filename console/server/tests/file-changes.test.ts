import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, rm, symlink, writeFile, mkdir } from 'node:fs/promises';
import { closeSync, fstatSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileChangeFixture } from './file-change-fixture.js';
import { exportFileChanges, fileChangePreview, FILE_CHANGE_PREVIEW_BYTES, filesystem } from '../src/file-changes.js';
import { capabilities, wire } from '../src/native.js';

const jsonLines = async (root: string, reference: Parameters<typeof exportFileChanges>[1]) => {
  const rows = [];
  for await (const bytes of exportFileChanges(root, reference)) rows.push(JSON.parse(bytes.toString()));
  return rows;
};

test('large manifests retain every raw path, 64-bit value and note through independent JSON Lines readback', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'file-changes-'));
  try {
    const { reference } = await fileChangeFixture(root, 3000, ['coverage: ' + 'x'.repeat(16000), 'another source excluded']);
    // Act
    const preview = await fileChangePreview(root, reference);
    const rows = await jsonLines(root, reference);
    // Assert: display bounds do not limit capture or download.
    assert.ok(preview?.downloadable);
    assert.ok(preview.previewIncomplete && preview.notesIncomplete);
    assert.ok(preview.preview.length > 0 && preview.preview.length < 3000);
    assert.ok(wire.FileChanges.encode(preview).finish().length <= FILE_CHANGE_PREVIEW_BYTES);
    assert.equal(rows.filter(row => row.type === 'coverage').length, 2);
    const changes = rows.filter(row => row.type === 'change');
    assert.equal(changes.length, 3000);
    assert.equal(changes[2999].path, Buffer.concat([Buffer.from('file-2999-'), Buffer.from([0xff])]).toString('base64'));
    assert.equal(changes[0].pathDisplay, 'file-0-\\xff');
    assert.equal(changes[0].after.size, '9223372036854775807');
    assert.equal(changes[0].after.mtimeSeconds, '-9223372036854775808');
    assert.equal(changes[0].after.symlink, '/g==');
    assert.equal(changes[0].after.symlinkDisplay, '\\xfe');
    assert.deepEqual(rows.at(-1), { type: 'complete', changes: '3000', coverageComplete: false });
    assert.equal(preview.report?.categories?.[0].counts?.added?.toString(), '3000');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('absence, failed capture, complete zero and an oversized preview record remain distinct', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'file-changes-'));
  try {
    // Act / Assert
    assert.equal(await fileChangePreview(root), null);
    assert.match((await fileChangePreview(root, null, 'Writers are active'))!.error, /Writers/);
    const { reference } = await fileChangeFixture(root, 0, []);
    const zero = await fileChangePreview(root, reference);
    assert.ok(zero?.downloadable && !zero.previewIncomplete);
    assert.equal((await jsonLines(root, reference)).at(-1).changes, '0');
    // Act: no individual record is shortened to fit the display.
    const large = await fileChangeFixture(root, 1, []);
    large.records[0].source = 'x'.repeat(FILE_CHANGE_PREVIEW_BYTES * 2);
    large.records[0].after = null;
    await writeFile(join(root, 'filesystem/host.pb'), filesystem.FileChange.encodeDelimited(large.records[0]).finish());
    const view = await fileChangePreview(root, large.reference);
    const exported = await jsonLines(root, large.reference);
    // Assert
    assert.equal(view?.preview.length, 0);
    assert.ok(view?.downloadable && view.previewIncomplete);
    assert.equal(exported.find(row => row.type === 'change').source.length, FILE_CHANGE_PREVIEW_BYTES * 2);
    assert.equal(exported.find(row => row.type === 'change').after, null);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('missing, truncated, malformed and count-mismatched manifests never emit completion', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'file-changes-'));
  try {
    for (const failure of ['missing', 'truncated', 'malformed', 'whole-record', 'kind-count', 'checkpoint']) {
      const { reference, records } = await fileChangeFixture(root, 2);
      const manifest = join(root, 'filesystem/host.pb');
      // Act
      if (failure === 'missing') await rm(manifest);
      if (failure === 'truncated') { const bytes = await readFile(manifest); await writeFile(manifest, bytes.subarray(0, -1)); }
      if (failure === 'malformed') await writeFile(manifest, Buffer.from([0xff, 0xff, 0xff, 0xff, 0xff]));
      if (failure === 'whole-record') await writeFile(manifest, filesystem.FileChange.encodeDelimited(records[0]).finish());
      if (failure === 'kind-count') { records[0].kind = 2; await writeFile(manifest, Buffer.concat(records.map(row => filesystem.FileChange.encodeDelimited(row).finish()))); }
      if (failure === 'checkpoint') await rm(join(root, reference.checkpoint));
      const preview = await fileChangePreview(root, reference);
      const rows: unknown[] = [];
      await assert.rejects(async () => { for await (const row of exportFileChanges(root, reference)) rows.push(JSON.parse(row.toString())); });
      // Assert
      assert.ok(preview?.error && !preview.downloadable, failure);
      assert.ok(!rows.some((row: any) => row.type === 'complete'), failure);
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('run-relative references reject traversal, cross-run paths and symlink escapes', async () => {
  // Arrange
  const parent = await mkdtemp(join(tmpdir(), 'file-changes-'));
  const root = join(parent, 'a'), other = join(parent, 'b');
  await mkdir(root); await mkdir(other);
  try {
    const { reference } = await fileChangeFixture(root);
    await fileChangeFixture(other);
    await symlink(join(other, 'filesystem/host.pb'), join(root, 'escape.pb'));
    for (const path of ['../b/filesystem/host.pb', join(other, 'filesystem/host.pb'), 'escape.pb']) {
      // Act: even an internally consistent saved reference cannot escape the run.
      const summary = { ...reference.categories[0], manifest: path };
      await writeFile(join(root, reference.checkpoint), filesystem.FileChangeReport.encode({ state: filesystem.ReportState.REPORT_COMPLETE, categories: [{ summary, coverageNotes: ['source excluded: live shared memory'] }] }).finish());
      const view = await fileChangePreview(root, { ...reference, categories: [summary] });
      // Assert
      assert.ok(view?.error && !view.downloadable, path);
    }
    await assert.rejects(jsonLines(root, { ...reference, checkpoint: '../b/filesystem/report.pb' }));
  } finally { await rm(parent, { recursive: true, force: true }); }
});

test('aborting a slow export closes its active manifest descriptor', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'file-changes-'));
  const original = capabilities.openFile;
  const opened: number[] = [];
  try {
    const { reference } = await fileChangeFixture(root, 3000);
    capabilities.openFile = (root, path) => { const fd = original(root, path); opened.push(fd); return fd; };
    const controller = new AbortController();
    const iterator = exportFileChanges(root, reference, controller.signal);
    // Act: consume one change, then disconnect while the rest is unread.
    let row;
    do { row = await iterator.next(); } while (JSON.parse(row.value!.toString()).type !== 'change');
    controller.abort();
    await iterator.return(undefined);
    // Assert
    for (const fd of opened) assert.throws(() => fstatSync(fd));
  } finally { capabilities.openFile = original; await rm(root, { recursive: true, force: true }); }
});


test('a published file report survives a later native report failure', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'file-changes-'));
  try {
    await fileChangeFixture(root, 3, []);
    // Act
    const recovered = await fileChangePreview(root, null, 'Native report write failed');
    // Assert
    assert.ok(recovered?.downloadable);
    assert.equal(recovered.state, filesystem.ReportState.REPORT_COMPLETE);
    assert.equal(recovered.preview.length, 3);
    assert.equal(recovered.error, '');
  } finally { await rm(root, { recursive: true, force: true }); }
});
