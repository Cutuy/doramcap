import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { closeSync, readFileSync } from 'node:fs';
import { readFile, writeFile, unlink, symlink } from 'node:fs/promises';
import { join } from 'node:path';
import { test } from 'node:test';
import { createConsoleServer } from '../src/http.js';
import { wire } from '../src/native.js';
import { runFilePath } from '../src/generated/transport.js';
import { sessionCookie } from './session.js';
import { runFileFixture } from './run-file-fixture.js';
import { filesystem } from '../src/file-changes.js';

test('saved positions download kept raw names and outputs after restart; private and escaping entries are refused', async () => {
  // Arrange
  const run = await runFileFixture();
  const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin: 'http://console.test', assets: run.root });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  const url = (position: number) => base + runFilePath(run.id, { changePosition: position });
  try {
    // Act / Assert
    assert.equal((await fetch(url(0))).status, 401);
    const cookie = await sessionCookie(base), headers = { cookie };
    for (const [position, contents] of [[0, run.html], [1, Buffer.from('raw bytes')], [2, Buffer.from('output')], [run.rows.length - 1, Buffer.from('tail')]] as const) {
      const response = await fetch(url(position), { headers });
      assert.equal(response.status, 200);
      assert.equal(response.headers.get('content-type'), 'application/octet-stream');
      assert.match(response.headers.get('content-disposition')!, /^attachment;.*filename\*=UTF-8''/);
      assert.deepEqual(Buffer.from(await response.arrayBuffer()), contents);
    }
    const raw = await fetch(url(1), { headers });
    assert.match(raw.headers.get('content-disposition')!, /file-%5Cxff/); await raw.body?.cancel();
    for (const position of [3, 4, 5, 6, 7, 8, 9, 10, 9999]) assert.equal((await fetch(url(position), { headers })).status, 404, String(position));
    assert.equal((await fetch(base + runFilePath(run.other, { changePosition: 0 }), { headers })).status, 404);
    for (const query of ['', '?change=-1', '?change=../etc', '?change=0&program=app', '?change=0&change=1', '?path=etc', '?change=18446744073709551616'])
      assert.equal((await fetch(`${base}/api/artifacts/${run.id}/run-file${query}`, { headers })).status, 400, query);
    assert.equal((await fetch(base + runFilePath(run.id, { program: 'project/writer' }), { headers })).status, 404);
    const detail = wire.HistoryDetail.decode(await run.backend.historyRun(run.id));
    assert.equal(detail.status?.writePolicy, 2);
    assert.equal(detail.status?.fileChanges?.keptDirectory, join(run.runRoot, 'kept'));
    assert.ok(detail.status?.fileChanges?.previewIncomplete);
    assert.ok(!detail.status?.fileChanges?.previewPositions?.some(value => String(value) === String(run.rows.length - 1)));
    // Act / Assert: output files remain downloadable under every policy.
    const checkpoint = join(run.runRoot, 'filesystem/report.pb');
    const saved = filesystem.FileChangeReport.decode(await readFile(checkpoint));
    for (const policy of [filesystem.WritePolicy.WRITE_POLICY_WRITE_BACK, filesystem.WritePolicy.WRITE_POLICY_DISCARD]) {
      saved.writePolicy = policy; await writeFile(checkpoint, filesystem.FileChangeReport.encode(saved).finish());
      assert.equal((await fetch(url(2), { headers })).status, 200);
      assert.equal((await fetch(url(0), { headers })).status, 404);
    }
    saved.writePolicy = filesystem.WritePolicy.WRITE_POLICY_KEEP;
    await writeFile(checkpoint, filesystem.FileChangeReport.encode(saved).finish());
    // Act: a saved file disappears or becomes a symlink after the preview was saved.
    await unlink(join(run.runRoot, 'kept/0/page.html'));
    await symlink(join(run.root, 'outside/secret'), join(run.runRoot, 'kept/0/page.html'));
    assert.equal((await fetch(url(0), { headers })).status, 404);
    // Act: an incomplete manifest cannot authorize even an earlier complete record.
    const manifest = join(run.runRoot, 'filesystem/host.pb');
    const bytes = await readFile(manifest); await writeFile(manifest, bytes.subarray(0, bytes.length - 1));
    assert.equal((await fetch(url(2), { headers })).status, 404);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close'); await run.close();
  }
});

test('an authorized descriptor remains bound to the opened raw-name file across replacement', async () => {
  // Arrange
  const run = await runFileFixture();
  try {
    // Act
    const opened = await run.backend.openRunFile(run.id, { changePosition: 1 }, new AbortController().signal);
    const path = Buffer.concat([Buffer.from(`${run.runRoot}/kept/0/`), run.rawName]);
    await unlink(path); await symlink(join(run.root, 'outside/secret'), path);
    // Assert
    try { assert.equal(readFileSync(opened.fd).toString(), 'raw bytes'); } finally { closeSync(opened.fd); }
    await assert.rejects(run.backend.openRunFile(run.id, { changePosition: 1 }, new AbortController().signal), /Missing/);
  } finally { await run.close(); }
});
