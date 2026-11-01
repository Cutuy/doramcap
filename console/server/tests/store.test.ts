import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, rm, writeFile, symlink } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { Store } from '../src/store.js';
import { wire } from '../src/native.js';

test('the data-root lock excludes concurrent writers and releases on close', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-lock-'));
  const first = await Store.open(root);
  try {
    // Act / Assert
    await assert.rejects(Store.open(root), /already in use/);
    first.close();
    const second = await Store.open(root);
    second.close();
  } finally {
    first.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('an unfinished metadata write preserves the prior marker and rejects symlink reads', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-marker-')),
    store = await Store.open(root);
  const original = wire.Marker.create({
    version: 1,
    runId: 'a'.repeat(32),
    artifactReference: 'a'.repeat(32),
    phase: wire.Phase.FINISHED,
  });
  try {
    // Act
    await store.write(original);
    await writeFile(join(root, 'current.pb.unfinished.tmp'), 'partial');
    // Assert
    assert.equal((await store.marker())!.runId, original.runId);
    // Act
    await store.write(wire.Marker.create({ ...original, runId: 'b'.repeat(32) }));
    // Assert
    assert.equal((await store.marker())!.runId, 'b'.repeat(32));
    // Act
    await rm(join(root, 'current.pb'));
    await symlink('/etc/passwd', join(root, 'current.pb'));
    // Assert
    assert.equal(await store.marker(), null);
  } finally {
    store.close();
    await rm(root, { recursive: true, force: true });
  }
});
