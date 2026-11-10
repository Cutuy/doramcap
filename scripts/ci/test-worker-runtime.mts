import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { workerHost } from './worker-ssh.mts';
import { downloadNodeArchive, nodeArchiveChecksum } from './install-node.mts';

const digest = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex');

test('Node installer requires exactly one published checksum for the requested archive', () => {
  // Arrange
  const archive = 'node-v1.2.3-linux-arm64.tar.gz', valid = `${digest('runtime')}  ${archive}`;
  // Act / Assert
  assert.equal(nodeArchiveChecksum(valid + '\n', archive), digest('runtime'));
  for (const manifest of ['', 'bad hash  ' + archive, valid + '\n' + valid]) {
    assert.throws(() => nodeArchiveChecksum(manifest, archive), /Expected one Node checksum/);
  }
});

for (const corrupt of [false, true]) {
  test(`Node installer ${corrupt ? 'rejects changed' : 'verifies downloaded'} runtime bytes`, async t => {
    // Arrange
    const root = mkdtempSync(join(tmpdir(), 'worker-node-test-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const archive = 'node-v1.2.3-linux-arm64.tar.gz', payload = Buffer.from('runtime fixture');
    const server = createServer((request, response) => {
      if (request.url === '/v1.2.3/SHASUMS256.txt') response.end(`${digest(payload)}  ${archive}\n`);
      else if (request.url === `/v1.2.3/${archive}`) response.end(corrupt ? 'changed bytes' : payload);
      else { response.statusCode = 404; response.end(); }
    });
    server.listen(0, '127.0.0.1'); await once(server, 'listening');
    t.after(() => { server.closeAllConnections(); server.close(); });
    const baseURL = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    const output = join(root, 'node.tar.gz');
    // Act / Assert
    if (corrupt) await assert.rejects(downloadNodeArchive('1.2.3', output, baseURL), /checksum mismatch/);
    else {
      assert.equal(await downloadNodeArchive('1.2.3', output, baseURL), digest(payload));
      assert.deepEqual(readFileSync(output), payload);
    }
  });
}

test('SSH worker names reject command and option injection', () => {
  // Arrange / Act / Assert
  assert.equal(workerHost('mac-mini-ubuntu24@100.64.0.22'), 'mac-mini-ubuntu24@100.64.0.22');
  assert.equal(workerHost('arm-builder'), 'arm-builder');
  for (const host of ['-oProxyCommand=id', '-user@host', 'user@host;id', 'user@$(id)', 'host name', 'user@-host']) {
    assert.throws(() => workerHost(host), /Invalid SSH worker/);
  }
});
