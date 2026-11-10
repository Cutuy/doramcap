import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { downloadNodeArchive, nodeArchiveChecksum } from './install-node.mts';
import { artifactNames, hashFile, parseArtifactManifest, verifyArtifacts, writeArtifactManifest } from './worker-artifacts.mts';

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

test('artifact verification rejects incomplete, duplicate, extra and malformed inventories', () => {
  // Arrange
  const lines = artifactNames.map(name => `${digest(name)}  ${name}`), valid = lines.join('\n') + '\n';
  // Act / Assert
  assert.equal(parseArtifactManifest(valid).size, artifactNames.length);
  assert.equal(parseArtifactManifest([...lines].reverse().join('\n')).size, artifactNames.length);
  for (const manifest of ['', lines.slice(1).join('\n'), valid + lines[0], valid + digest('extra') + '  extra\n', 'malformed']) {
    assert.throws(() => parseArtifactManifest(manifest));
  }
});

test('artifact hashes cover every file and reject changed bytes and symlinks', async t => {
  // Arrange
  const root = mkdtempSync(join(tmpdir(), 'worker-artifacts-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const name of artifactNames) writeFileSync(join(root, name), name);
  // Act / Assert
  await writeArtifactManifest(root); await verifyArtifacts(root);
  writeFileSync(join(root, 'docker'), 'changed');
  await assert.rejects(verifyArtifacts(root), /checksum mismatch: docker/);
  rmSync(join(root, 'docker')); symlinkSync(join(root, 'cargo-nextest'), join(root, 'docker'));
  await assert.rejects(verifyArtifacts(root), /regular file/);
});

test('cancelled hashing stops before reading a file or replacing a manifest', async t => {
  // Arrange
  const root = mkdtempSync(join(tmpdir(), 'worker-artifact-cancel-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'artifacts')); const path = join(root, 'artifacts');
  writeFileSync(join(path, 'SHA256SUMS'), 'previous');
  const controller = new AbortController(); controller.abort(new Error('interrupted'));
  // Act / Assert
  await assert.rejects(hashFile(join(path, 'missing'), controller.signal), /interrupted/);
  await assert.rejects(writeArtifactManifest(path, controller.signal), /interrupted/);
  assert.equal(readFileSync(join(path, 'SHA256SUMS'), 'utf8'), 'previous');
});
