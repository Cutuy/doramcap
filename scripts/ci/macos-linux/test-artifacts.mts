import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { successfulBuild } from './submit.mts';
import { hashFile } from '../hash-file.mts';
import { artifactNames, parseArtifactManifest, verifyArtifacts, writeArtifactManifest } from './artifacts.mts';
const digest = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex');

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

test('only a successful build of the exact queued task permits transfer', () => {
  // Arrange
  const status = JSON.stringify({ tasks: { 7: { status: { Done: { result: 'Success' } } },
    8: { status: { Done: { result: { Failed: 1 } } } }, 9: { status: 'Running' } } });
  // Act / Assert
  successfulBuild(status, '7');
  for (const id of ['8', '9', '10']) assert.throws(() => successfulBuild(status, id), /did not succeed/);
  assert.throws(() => successfulBuild(status, '7;id'), /Invalid Pueue task ID/);
});
