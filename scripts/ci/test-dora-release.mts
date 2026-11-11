import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { doraArchive, doraVersion, downloadDora } from './dora-release.mts';

function graph(version = '1.2.3') {
  return { workspace_members: ['client'], packages: [
    { id: 'client', name: 'dora-control', version: '0.1.0' },
    { id: 'wire', name: 'dora-message', version },
  ], resolve: { nodes: [{ id: 'client', deps: [{ pkg: 'wire' }] }, { id: 'wire', deps: [] }] } };
}
const assetURL = 'https://github.com/dora-rs/dora/releases/download/v1.2.3/dora-cli-x86_64-unknown-linux-gnu.tar.gz';
const sha256 = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');

test('resolved protocol upgrades follow the client edge and reject mixed wire versions', () => {
  // Arrange / Act / Assert
  assert.equal(doraVersion(graph()), '1.2.3');
  assert.equal(doraVersion(graph('1.9.4')), '1.9.4');
  const mixed = graph();
  mixed.packages.push({ id: 'other-wire', name: 'dora-message', version: '1.1.0' });
  mixed.resolve.nodes.push({ id: 'other-wire', deps: [] });
  assert.throws(() => doraVersion(mixed), /Mixed Dora protocol versions/);
  const missing = graph();
  missing.resolve.nodes[0].deps = [];
  assert.throws(() => doraVersion(missing), /resolved client dora-message/);
  assert.throws(() => doraVersion({ ...graph(), resolve: null }), /Resolved Cargo/);
  assert.throws(() => doraVersion(graph('1.2.3-beta')), /stable Dora protocol release/);
});

test('release selection requires a committed checksum for the exact version and architecture', () => {
  // Arrange
  const pins = { '1.2.3': { amd64: 'a'.repeat(64) }, '1.2.5': { amd64: 'bad' } };
  // Act / Assert
  assert.deepEqual(doraArchive('1.2.3', 'amd64', pins),
    { url: assetURL, sha256: 'a'.repeat(64), member: 'dora-cli-x86_64-unknown-linux-gnu/dora' });
  assert.throws(() => doraArchive('1.2.4', 'amd64', pins), /No reviewed checksum/);
  assert.throws(() => doraArchive('1.2.3', 'arm64', pins), /No reviewed checksum/);
  assert.throws(() => doraArchive('1.2.5', 'amd64', pins), /No reviewed checksum/);
  assert.throws(() => doraArchive('1.2.3', 'unknown', pins), /Unsupported/);
  assert.throws(() => doraArchive('latest', 'amd64', pins), /Invalid Dora release version/);
  assert.match(doraArchive('1.1.0', 'arm64').member, /aarch64/);
});

test('download verifies bytes and executable version before replacing an existing artifact', async t => {
  // Arrange: use a real archive and executable with an upgraded version.
  const root = mkdtempSync(join(tmpdir(), 'dora-release-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const folder = join(root, 'dora-cli-x86_64-unknown-linux-gnu');
  mkdirSync(folder);
  const output = join(root, 'installed-dora');
  const archive = join(root, 'archive.tar.gz');
  const pack = (version: string) => {
    writeFileSync(join(folder, 'dora'), `#!/bin/sh\nprintf '%s\\n' 'dora-cli ${version}'\n`);
    chmodSync(join(folder, 'dora'), 0o755);
    execFileSync('tar', ['-czf', archive, '-C', root, 'dora-cli-x86_64-unknown-linux-gnu']);
    return readFileSync(archive);
  };
  let bytes = pack('1.2.3');
  const pins = { '1.2.3': { amd64: sha256(bytes) } };
  t.mock.method(globalThis, 'fetch', async (url: string) => {
    assert.equal(url, assetURL);
    return new Response(bytes);
  });
  // Act / Assert: a successful upgrade installs the matching executable.
  await downloadDora('1.2.3', 'amd64', output, pins);
  assert.equal(execFileSync(output, ['--version'], { encoding: 'utf8' }).trim(), 'dora-cli 1.2.3');
  const previous = readFileSync(output);
  // Act / Assert: a replaced or corrupt archive preserves the previous binary and removes staging.
  bytes = Buffer.from('corrupt archive');
  await assert.rejects(downloadDora('1.2.3', 'amd64', output, pins), /checksum mismatch/);
  assert.deepEqual(readFileSync(output), previous);
  assert(!readdirSync(root).some(name => name.startsWith('.dora-')));
  // Act / Assert: a pinned archive holding the wrong executable also fails.
  bytes = pack('1.2.4');
  pins['1.2.3'].amd64 = sha256(bytes);
  await assert.rejects(downloadDora('1.2.3', 'amd64', output, pins), /binary version differs/);
  assert.deepEqual(readFileSync(output), previous);
  assert(!readdirSync(root).some(name => name.startsWith('.dora-')));
});

test('an unpinned or unavailable release fails without selecting a different version', async t => {
  // Arrange
  const root = mkdtempSync(join(tmpdir(), 'dora-release-missing-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const output = join(root, 'dora');
  writeFileSync(output, 'previous');
  const requests: string[] = [];
  t.mock.method(globalThis, 'fetch', async (url: string) => {
    requests.push(url);
    return new Response('missing', { status: 404 });
  });
  // Act / Assert: an unpinned version is refused before any download.
  await assert.rejects(downloadDora('1.9.4', 'amd64', output, {}), /No reviewed checksum/);
  assert.deepEqual(requests, []);
  // Act / Assert: a pinned but unpublished archive fails and keeps the previous output.
  await assert.rejects(downloadDora('1.2.3', 'amd64', output, { '1.2.3': { amd64: 'a'.repeat(64) } }), /HTTP 404/);
  assert.deepEqual(requests, [assetURL]);
  assert.equal(readFileSync(output, 'utf8'), 'previous');
});
