import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { doramcap } from '../src/generated/console.js';
import { readLogs } from '../src/logs.js';
import { History } from '../src/history.js';
import { Store } from '../src/store.js';
import { wire } from '../src/native.js';
import { LIMITS } from '../src/generated/transport.js';

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


test('large recovered indexes use existing History storage and response budgets', async () => {
  // Arrange: a real program list is larger than the former independent one-MiB cutoff.
  const root = await mkdtemp(join(tmpdir(), 'console-logs-budget-'));
  const id = 'b'.repeat(32), run = join(root, 'runs', id);
  const store = await Store.open(root), history = new History(store);
  const programs = Array.from({ length: 28000 }, (_, number) => ({
    programId: `camera-${number}`, identity: 'a'.repeat(64), state: 2, sizeBytes: 123,
  }));
  const bytes = doramcap.journal.v1.LogIndex.encode({ programs }).finish();
  assert.ok(bytes.length > 2 * 1024 * 1024);
  assert.ok(bytes.length < LIMITS.frameBytes);
  try {
    await mkdir(join(run, 'logs'), { recursive: true });
    await writeFile(join(run, 'logs/index.pb'), bytes);
    await writeFile(join(run, 'logs/0.log'), 'saved evidence\n');
    // Act: saving a derived log copy cannot overflow the existing History detail budget.
    history.record(wire.Status.create({ runId: id, phase: wire.Phase.FINISHED,
      outcome: wire.Outcome.create({ execution: 'Completed' }), logs: doramcap.journal.v1.LogIndex.create({ programs }) }), 1, 2);
    const row = store.db.prepare('SELECT detail FROM runs WHERE run_id = ?').get(id) as { detail: Uint8Array };
    // Assert
    assert.ok(row.detail.length <= 2 * 1024 * 1024);
    assert.equal(wire.Status.decode(row.detail).outcome!.execution, 'Completed');
    // Act: the authoritative list can still fit the larger History response budget.
    const detail = history.detail(id)!;
    // Assert
    assert.equal(detail.status!.logs!.programs!.length, programs.length);
    assert.equal(detail.status!.logsError, '');
    assert.ok(wire.HistoryDetail.encode(detail).finish().length <= LIMITS.frameBytes);
    // Act: other saved metadata consumes part of the response budget.
    history.record(wire.Status.create({ runId: id, phase: wire.Phase.FINISHED,
      profiles: Array.from({ length: 128 }, (_, number) => `profile-${number}-${'x'.repeat(4000)}`),
      outcome: wire.Outcome.create({ execution: 'Completed' }) }), 1, 2);
    const nearLimit = doramcap.journal.v1.LogIndex.encode({ programs: [{ programId: 'x'.repeat(LIMITS.frameBytes - 128), state: 2 }] }).finish();
    assert.ok(nearLimit.length < LIMITS.frameBytes);
    await writeFile(join(run, 'logs/index.pb'), nearLimit);
    const unavailable = history.detail(id)!;
    // Assert: reject the complete oversized response, preserving the other result data.
    assert.equal(unavailable.status!.logs, null);
    assert.match(unavailable.status!.logsError!, /message budget/);
    assert.equal(unavailable.status!.profiles!.length, 128);
    assert.equal(unavailable.status!.outcome!.execution, 'Completed');
    assert.ok(wire.HistoryDetail.encode(unavailable).finish().length <= LIMITS.frameBytes);
    assert.equal(await readFile(join(run, 'logs/0.log'), 'utf8'), 'saved evidence\n');
  } finally { store.close(); await rm(root, { recursive: true, force: true }); }
});

test('log reads honor the existing frame limit including protobuf envelope overhead', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-logs-frame-'));
  try {
    await mkdir(join(root, 'logs'));
    // Act: an index below the frame ceiling leaves no room for the enclosing Status.
    const bytes = doramcap.journal.v1.LogIndex.encode({ programs: [{ programId: 'x'.repeat(LIMITS.frameBytes - 10) }] }).finish();
    assert.ok(bytes.length <= LIMITS.frameBytes);
    await writeFile(join(root, 'logs/index.pb'), bytes);
    // Assert
    assert.equal(readLogs(root).logs, null);
    assert.match(readLogs(root).logsError, /message budget/);
    // Act: an on-disk index exceeds the existing transport input budget.
    await writeFile(join(root, 'logs/index.pb'), Buffer.alloc(LIMITS.frameBytes + 1));
    // Assert
    assert.equal(readLogs(root).logs, null);
    assert.match(readLogs(root).logsError, /message budget/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
