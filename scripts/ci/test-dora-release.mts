import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { doraAsset, doraVersion, downloadDora } from './dora-release.mts';

function graph(version = '1.2.3') {
  return { workspace_members: ['client'], packages: [
    { id: 'client', name: 'dora-control', version: '0.1.0' },
    { id: 'wire', name: 'dora-message', version },
  ], resolve: { nodes: [{ id: 'client', deps: [{ pkg: 'wire' }] }, { id: 'wire', deps: [] }] } };
}
function release(version = '1.2.3', arch = 'x86_64', digest = 'a'.repeat(64)) {
  const name = `dora-cli-${arch}-unknown-linux-gnu.tar.gz`;
  return { tag_name: `v${version}`, draft: false, assets: [{ name, state: 'uploaded', digest: `sha256:${digest}`,
    browser_download_url: `https://github.com/dora-rs/dora/releases/download/v${version}/${name}` }] };
}

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

test('release selection requires the exact architecture, version and published digest', () => {
  // Arrange / Act / Assert
  assert.equal(doraAsset(release(), '1.2.3', 'amd64').sha256, 'a'.repeat(64));
  assert.match(doraAsset(release('1.2.3', 'aarch64'), '1.2.3', 'arm64').member, /aarch64/);
  assert.throws(() => doraAsset(release(), '1.2.4', 'amd64'), /published Dora/);
  assert.throws(() => doraAsset(release(), '1.2.3', 'arm64'), /Expected one Dora/);
  assert.throws(() => doraAsset(release(), '1.2.3', 'unknown'), /Unsupported/);
  const invalid = [
    { ...release(), draft: true },
    { ...release(), assets: [] },
    { ...release(), assets: [...release().assets, ...release().assets] },
    { ...release(), assets: [{ ...release().assets[0], digest: null }] },
    { ...release(), assets: [{ ...release().assets[0], digest: 'sha256:bad' }] },
    { ...release(), assets: [{ ...release().assets[0], state: 'new' }] },
    { ...release(), assets: [{ ...release().assets[0], browser_download_url: 'https://example.com/dora' }] },
  ];
  for (const metadata of invalid) assert.throws(() => doraAsset(metadata, '1.2.3', 'amd64'));
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
  const metadata = release('1.2.3', 'x86_64', createHash('sha256').update(bytes).digest('hex'));
  const assetURL = metadata.assets[0].browser_download_url;
  t.mock.method(globalThis, 'fetch', async (url: string) => {
    assert([assetURL, 'https://api.github.com/repos/dora-rs/dora/releases/tags/v1.2.3'].includes(url));
    return url === assetURL ? new Response(bytes) : Response.json(metadata);
  });
  // Act / Assert: a successful upgrade installs the matching executable.
  await downloadDora('1.2.3', 'amd64', output);
  assert.equal(execFileSync(output, ['--version'], { encoding: 'utf8' }).trim(), 'dora-cli 1.2.3');
  const previous = readFileSync(output);
  // Act / Assert: a checksum failure preserves the previous binary and removes staging.
  bytes = Buffer.from('corrupt archive');
  await assert.rejects(downloadDora('1.2.3', 'amd64', output), /checksum mismatch/);
  assert.deepEqual(readFileSync(output), previous);
  assert(!readdirSync(root).some(name => name.startsWith('.dora-')));
  // Act / Assert: an archive with a valid digest but wrong executable also fails.
  bytes = pack('1.2.4');
  metadata.assets[0].digest = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
  await assert.rejects(downloadDora('1.2.3', 'amd64', output), /binary version differs/);
  assert.deepEqual(readFileSync(output), previous);
  assert(!readdirSync(root).some(name => name.startsWith('.dora-')));
});

test('an unavailable release fails without selecting a different version', async t => {
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
  // Act / Assert
  await assert.rejects(downloadDora('1.9.4', 'amd64', output), /HTTP 404/);
  assert.deepEqual(requests, ['https://api.github.com/repos/dora-rs/dora/releases/tags/v1.9.4']);
  assert.equal(readFileSync(output, 'utf8'), 'previous');
});
