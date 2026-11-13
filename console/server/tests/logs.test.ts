import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { doramcap } from '../src/generated/console.js';
import { readLogs } from '../src/logs.js';
import { History } from '../src/history.js';
import { Store } from '../src/store.js';
import { wire } from '../src/native.js';

test('History rereads recovered log evidence after the database and Console restart', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-logs-'));
  const id = 'a'.repeat(32), run = join(root, 'runs', id);
  let store = await Store.open(root);
  try {
    await mkdir(join(run, 'logs'), { recursive: true });
    new History(store).record(wire.Status.create({ runId: id, phase: wire.Phase.FINISHED, outcome: wire.Outcome.create({ execution: 'Completed' }) }), 1, 2);
    store.close();
    // Act: cleanup publishes the single index after the old History record was saved.
    await writeFile(join(run, 'logs/index.pb'), doramcap.journal.v1.LogIndex.encode({ programs: [{ programId: 'camera', state: 2, sizeBytes: 17 }] }).finish());
    store = await Store.open(root);
    const restored = new History(store).detail(id)!;
    // Assert
    assert.equal(restored.status!.logs!.programs![0]!.programId, 'camera');
    assert.equal(String(restored.status!.logs!.programs![0]!.sizeBytes), '17');
    assert.equal(restored.status!.outcome!.execution, 'Completed');
    // Act / Assert: corrupt and escaped indexes produce explicit errors.
    await writeFile(join(run, 'logs/index.pb'), Buffer.from([255]));
    assert.match(readLogs(run).logsError, /Logs unavailable/);
    await rm(join(run, 'logs/index.pb'));
    assert.equal(readLogs(run).logsError, '');
    await symlink('/etc/passwd', join(run, 'logs/index.pb'));
    assert.match(readLogs(run).logsError, /Logs unavailable/);
  } finally { store.close(); await rm(root, { recursive: true, force: true }); }
});
