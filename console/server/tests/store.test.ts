import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { chmod, mkdtemp, rm, stat } from 'node:fs/promises';
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

test('a data root other users can access is refused before any file is created', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-private-'));
  try {
    await chmod(root, 0o770);
    // Act / Assert
    await assert.rejects(Store.open(root), /must be owned by this user and mode 0700/);
    await assert.rejects(stat(join(root, 'console.db')), { code: 'ENOENT' });
    // Act / Assert
    await chmod(root, 0o700);
    (await Store.open(root)).close();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('the marker survives reopening and an unreadable marker reads as none', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-marker-'));
  let store = await Store.open(root);
  const original = wire.Marker.create({ version: 1, runId: 'a'.repeat(32), artifactReference: 'a'.repeat(32), phase: wire.Phase.FINISHED });
  try {
    // Act
    store.write(original);
    store.write(wire.Marker.create({ ...original, runId: 'b'.repeat(32) }));
    store.close();
    store = await Store.open(root);
    // Assert
    assert.equal(store.marker()!.runId, 'b'.repeat(32));
    // Act
    store.db.prepare('UPDATE marker SET value = ?').run(Buffer.from([0xff, 0xff, 0xff]));
    // Assert
    assert.equal(store.marker(), null);
  } finally {
    store.close();
    await rm(root, { recursive: true, force: true });
  }
});
