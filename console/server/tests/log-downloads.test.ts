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

test('saved log positions download distinct full and partial files with program names after restart', async () => {
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
    { programId: 'duplicate', identity: 'a'.repeat(64), state: 2, sizeBytes: 7 },
    { programId: 'duplicate', identity: 'b'.repeat(64), state: 4, sizeBytes: 7 },
    { programId: 'changed-size', state: 2, sizeBytes: 99 },
    { programId: 'hardlink', state: 2, sizeBytes: 7 },
    { programId: 'symlink', state: 2, sizeBytes: 7 },
    { programId: 'directory', state: 2, sizeBytes: 0 },
    { programId: '../../outside/secret', state: 2, sizeBytes: 7 },
    { programId: 'failed-empty', state: 4, sizeBytes: 0 },
    { programId: '', state: 2, sizeBytes: 7 },
    { programId: 'camera/"雪"\r\n', state: 2, sizeBytes: 7 },
  ];
  for (let position = 0; position < 8; position++) await writeFile(join(directory, `${position}.log`), position === 0 ? 'full log' : position === 4 ? '' : 'partial');
  await writeFile(join(directory, '5.log'), 'first!!');
  await writeFile(join(directory, '6.log'), 'second!');
  await link(join(run.root, 'outside/secret'), join(directory, '8.log'));
  await symlink(join(run.root, 'outside/secret'), join(directory, '9.log'));
  await mkdir(join(directory, '10.log'));
  await writeFile(join(directory, '11.log'), 'indexed');
  await writeFile(join(directory, '12.log'), '');
  await writeFile(join(directory, '13.log'), 'unnamed');
  await writeFile(join(directory, '14.log'), 'unicode');
  const index = LogIndex.encode({ programs }).finish();
  await writeFile(join(directory, 'index.pb'), index);
  const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin: 'http://console.test', assets: run.root });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`;
  const url = (logPosition: number | string, id = run.id) => base + runFilePath(id, doramcap.console.v1.RunFileRequest.fromObject({ logPosition }));
  try {
    // Act / Assert
    assert.equal((await fetch(url(1))).status, 401);
    const headers = { cookie: await sessionCookie(base) };
    for (const [position, contents, filename] of [
      [0, 'full log', 'project-camera & one.log'], [1, 'partial', 'partial.log'], [4, '', 'empty.log'],
      [5, 'first!!', 'duplicate.log'], [6, 'second!', 'duplicate.log'],
      [11, 'indexed', '..-..-outside-secret.log'], [13, 'unnamed', '13.log'], [14, 'unicode', 'camera--雪---.log'],
    ] as const) {
      const response = await fetch(url(position), { headers });
      assert.equal(response.status, 200, String(position));
      assert.equal(response.headers.get('content-type'), 'application/octet-stream');
      assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
      const attachment = response.headers.get('content-disposition')!;
      assert.ok(attachment.startsWith('attachment;'));
      assert.equal(decodeURIComponent(attachment.split("filename*=UTF-8''")[1]), filename);
      assert.equal(await response.text(), contents);
    }
    for (const position of [2, 3, 7, 8, 9, 10, 12, 15, '18446744073709551615'])
      assert.equal((await fetch(url(position), { headers })).status, 404, String(position));
    assert.equal((await fetch(url(1, run.other), { headers })).status, 404);
    for (const query of ['log=', 'log=-1', 'log=1.5', 'log=01', 'log=../outside/secret', 'log=18446744073709551616', 'log=0&log=1', 'log=1&change=0', 'program=partial'])
      assert.equal((await fetch(`${base}/api/artifacts/${run.id}/run-file.download?${query}`, { headers })).status, 400, query);
    assert.throws(() => runFilePath(run.id, { logPosition: 1, changePosition: 0 }), /InvalidInput/);
    await assert.rejects(run.backend.openRunFile(run.id, { logPosition: 1, changePosition: 0 }, new AbortController().signal), /Missing/);
    await assert.rejects(run.backend.openRunFile(run.id, { logPosition: -1 }, new AbortController().signal), /Missing/);
    // Logs remain private when selected through a saved file-change position.
    run.rows[2] = filesystem.FileChange.create({ ...run.rows[2], source: run.runRoot, path: Buffer.from('logs/index.pb'), after: { mode: 0o100600, size: index.length } });
    await writeFile(join(run.runRoot, 'filesystem/host.pb'), Buffer.concat(run.rows.map(row => filesystem.FileChange.encodeDelimited(row).finish())));
    assert.equal((await fetch(base + runFilePath(run.id, { changePosition: 2 }), { headers })).status, 404);
    // Replacements cannot redirect an already opened response to another file.
    const opened = await run.backend.openRunFile(run.id, { logPosition: 1 }, new AbortController().signal);
    await unlink(join(directory, '1.log')); await symlink(join(run.root, 'outside/secret'), join(directory, '1.log'));
    try { assert.equal(readFileSync(opened.fd).toString(), 'partial'); } finally { closeSync(opened.fd); }
    assert.equal((await fetch(url(1), { headers })).status, 404);
    await rename(directory, join(run.root, 'saved-logs'));
    await symlink(join(run.root, 'saved-logs'), directory);
    assert.equal((await fetch(url(0), { headers })).status, 404);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close'); await run.close();
  }
});
