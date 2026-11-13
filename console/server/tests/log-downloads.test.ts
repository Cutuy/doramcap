import assert from 'node:assert/strict';
import { test } from 'node:test';
import { once } from 'node:events';
import { closeSync, readFileSync } from 'node:fs';
import { mkdir, writeFile, symlink, link, unlink, rename } from 'node:fs/promises';
import { join } from 'node:path';
import { doramcap } from '../src/generated/console.js';
import { createConsoleServer } from '../src/http.js';
import { filesystem } from '../src/file-changes.js';
import { runFilePath } from '../src/generated/transport.js';
import { runFileFixture } from './run-file-fixture.js';
import { sessionCookie } from './session.js';

const { LogIndex } = doramcap.journal.v1;

test('saved program selectors download full and partial logs after restart through the shared route', async () => {
  // Arrange: the fixture reopened its database before these exports were published.
  const run = await runFileFixture();
  const directory = join(run.runRoot, 'logs');
  await mkdir(directory);
  const programs = [
    { programId: 'project/camera & one', state: 2, sizeBytes: 8 },
    { programId: 'partial', state: 4, sizeBytes: 7 },
    { programId: 'skipped', state: 3, sizeBytes: 7 },
    { programId: 'pending', state: 1, sizeBytes: 7 },
    { programId: 'empty', state: 2, sizeBytes: 0 },
    { programId: 'duplicate', state: 2, sizeBytes: 7 },
    { programId: 'duplicate', state: 2, sizeBytes: 7 },
    { programId: 'changed-size', state: 2, sizeBytes: 99 },
    { programId: 'hardlink', state: 2, sizeBytes: 7 },
    { programId: 'symlink', state: 2, sizeBytes: 7 },
    { programId: 'directory', state: 2, sizeBytes: 0 },
    { programId: '../../outside/secret', state: 2, sizeBytes: 7 },
    { programId: 'failed-empty', state: 4, sizeBytes: 0 },
  ];
  for (let position = 0; position < 8; position++) await writeFile(join(directory, `${position}.log`), position === 0 ? 'full log' : position === 4 ? '' : 'partial');
  await link(join(run.root, 'outside/secret'), join(directory, '8.log'));
  await symlink(join(run.root, 'outside/secret'), join(directory, '9.log'));
  await mkdir(join(directory, '10.log'));
  await writeFile(join(directory, '11.log'), 'indexed');
  await writeFile(join(directory, '12.log'), '');
  const index = LogIndex.encode({ programs }).finish();
  await writeFile(join(directory, 'index.pb'), index);
  const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin: 'http://console.test', assets: run.root });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  const url = (program: string, id = run.id) => base + runFilePath(id, { program });
  try {
    // Act / Assert
    assert.equal((await fetch(url('partial'))).status, 401);
    const headers = { cookie: await sessionCookie(base) };
    for (const [program, contents, filename] of [['project/camera & one', 'full log', '0.log'], ['partial', 'partial', '1.log'], ['empty', '', '4.log'], ['../../outside/secret', 'indexed', '11.log']]) {
      const response = await fetch(url(program), { headers });
      assert.equal(response.status, 200, program);
      assert.equal(response.headers.get('content-type'), 'application/octet-stream');
      assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
      assert.match(response.headers.get('content-disposition')!, new RegExp(`^attachment;.*filename\\*=UTF-8''${filename.replace('.', '\\.')}`));
      assert.equal(await response.text(), contents);
    }
    for (const program of ['missing', 'skipped', 'pending', 'duplicate', 'changed-size', 'hardlink', 'symlink', 'directory', '../outside/secret', 'failed-empty'])
      assert.equal((await fetch(url(program), { headers })).status, 404, program);
    assert.equal((await fetch(url('partial', run.other), { headers })).status, 404);
    assert.equal((await fetch(`${url('partial')}&change=0`, { headers })).status, 400);
    await assert.rejects(run.backend.openRunFile(run.id, { program: 'partial', changePosition: 0 }, new AbortController().signal), /Missing/);
    // Logs remain private when selected through a saved file-change position.
    run.rows[2] = filesystem.FileChange.create({ ...run.rows[2], source: run.runRoot, path: Buffer.from('logs/index.pb'), after: { mode: 0o100600, size: index.length } });
    await writeFile(join(run.runRoot, 'filesystem/host.pb'), Buffer.concat(run.rows.map(row => filesystem.FileChange.encodeDelimited(row).finish())));
    assert.equal((await fetch(base + runFilePath(run.id, { changePosition: 2 }), { headers })).status, 404);
    // Replacements cannot redirect an already opened response to another file.
    const opened = await run.backend.openRunFile(run.id, { program: 'partial' }, new AbortController().signal);
    await unlink(join(directory, '1.log')); await symlink(join(run.root, 'outside/secret'), join(directory, '1.log'));
    try { assert.equal(readFileSync(opened.fd).toString(), 'partial'); } finally { closeSync(opened.fd); }
    assert.equal((await fetch(url('partial'), { headers })).status, 404);
    await rename(directory, join(run.root, 'saved-logs'));
    await symlink(join(run.root, 'saved-logs'), directory);
    assert.equal((await fetch(url('project/camera & one'), { headers })).status, 404);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close'); await run.close();
  }
});
