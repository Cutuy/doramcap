import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer, type Socket } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { capabilities, holoscan, launch } from '../src/native.js';

test('native Build retains failed setup, log references, Stop and close outcomes across reopen', { timeout: 15_000 }, async () => {
  // Arrange: real native owner/storage, with no container or privileged host effect.
  const root = await mkdtemp(join(tmpdir(), 'holoscan-build-native-'));
  await mkdir(join(root, 'state'), { mode: 0o700 });
  const store = holoscan.BuildStoreRequest.create({ root: join(root, 'state/builds'), key: { repositoryRoot: root, selection: { application: 'sample' } } });
  const request = (id: string) => holoscan.BuildRequest.create({ attempt: { store, id }, cli: '/missing/holoscan', readinessSeconds: 5, stopSeconds: 1, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH });
  let owner = await capabilities.open(join(root, 'missing.sock'), 1);
  const sockets = new Set<Socket>();
  const server = createServer(socket => { sockets.add(socket); socket.on('close', () => sockets.delete(socket)); });
  try {
    // Act / Assert: transport construction itself is an admitted failed latest attempt.
    const failed = await owner.build(request('failed'));
    assert.equal(failed.attempt!.id, 'failed');
    assert.match(failed.attempt!.result!.processError!, /connect|socket/i);
    assert.equal(failed.attempt!.result!.record, null);
    assert.equal(failed.publicationError, '');
    const current = await owner.buildLookup(store);
    assert.equal(current.latest!.id, 'failed');
    assert.equal(current.record, null);
    assert.deepEqual(current.logs, failed.logs);
    assert.equal(owner.poll().buildOutcome!.attempt!.id, 'failed');
    assert.equal(owner.poll().buildOutcome!.attempt!.id, 'failed');
    await assert.rejects(readFile(failed.logs!.stdout!), { code: 'ENOENT' });
    const unsupportedBudget = holoscan.BuildRequest.fromObject({ ...request('unsupported-clock'), readinessSeconds: '18446744073709551615' });
    const rejected = await owner.build(unsupportedBudget);
    assert.match(rejected.error!, /supported-clock/);
    assert.equal(rejected.attempt, null);
    assert.equal((await owner.buildLookup(store)).latest!.id, 'failed');
    await owner.close();

    // Arrange: negotiation blocks on observable accepted connection, before CLI/helper launch.
    const socket = join(root, 'docker.sock');
    server.listen(socket); await once(server, 'listening');
    owner = await capabilities.open(socket, 1);
    // Act / Assert: synchronous Build enqueue precedes an immediate Stop before any poll.
    const immediate = owner.build(request('immediate'));
    await owner.stop('immediate');
    assert.equal((await immediate).attempt!.result!.cancelled, true);
    assert.equal(owner.poll().buildOutcome!.attempt!.id, 'immediate');
    const accepted = () => once(server, 'connection');
    const arrived = accepted();
    const pending = owner.build(request('stopped'));
    await arrived;
    const progress = owner.poll().buildProgress!;
    assert.equal(progress.attemptId, 'stopped');
    assert.ok(progress.logs!.stdout!.endsWith('/stdout.log'));
    const other = await capabilities.open(socket, 1);
    try { await assert.rejects(other.buildLookup(store), /another run owns this repository/); }
    finally { await other.close(); }
    await assert.rejects(owner.build(request('conflict')), /Conflict/);
    // Act / Assert: Stop signals the active Build and leaves no older success.
    await owner.stop('stopped');
    const stopped = await pending;
    assert.equal(stopped.attempt!.result!.cancelled, true);
    assert.equal((await owner.buildLookup(store)).record, null);

    // Act / Assert: close waits for active Build cancellation/publication; reopen sees that attempt.
    const closingArrived = accepted();
    const closing = owner.build(request('closed'));
    await closingArrived;
    await owner.close();
    assert.equal((await closing).attempt!.result!.cancelled, true);
    await assert.rejects(owner.buildLookup(store), /Closing/);
    owner = await capabilities.open(socket, 1);
    const restored = await owner.buildLookup(store);
    assert.equal(restored.latest!.id, 'closed');
    assert.equal(restored.latest!.result!.cancelled, true);
    assert.equal(restored.record, null);
  } finally {
    await owner.close();
    for (const socket of sockets) socket.destroy();
    if (server.listening) await new Promise<void>(resolve => server.close(() => resolve()));
    await rm(root, { recursive: true, force: true });
  }
});
