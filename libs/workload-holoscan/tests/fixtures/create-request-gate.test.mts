import { strict as assert } from 'node:assert';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer, request } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { createRequestGate } from './create-request-gate.mjs';

const BODY = JSON.stringify({ Image: 'cached-fixture', Labels: { 'com.docker.compose.service': 'cpu-monitor' } });
async function fixture(t: TestContext, holdResponse = false) {
  const root = await mkdtemp(join(tmpdir(), 'compose-create-gate-'));
  let count = 0, accepted!: () => void;
  const reachedDaemon = new Promise<void>(resolve => accepted = resolve);
  const daemon = createServer(async (req, res) => {
    const chunks: Buffer[] = []; for await (const chunk of req) chunks.push(Buffer.from(chunk));
    assert.equal(Buffer.concat(chunks).toString(), BODY);
    count++; accepted(); if (!holdResponse) res.end('created');
  });
  const socket = join(root, 'daemon.sock');
  await new Promise<void>(resolve => daemon.listen(socket, resolve));
  const proxy = join(root, 'proxy.sock'), gate = await createRequestGate(proxy, socket);
  t.after(async () => {
    await gate.close(); daemon.closeAllConnections();
    await new Promise<void>(resolve => daemon.close(() => resolve()));
    await rm(root, { recursive: true, force: true });
  });
  const post = (signal?: AbortSignal) => new Promise<string>((resolve, reject) => {
    const req = request({ socketPath: proxy, path: '/v1.56/containers/create', method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(BODY) }, signal }, res => {
      let body = ''; res.setEncoding('utf8'); res.on('data', chunk => body += chunk); res.once('end', () => resolve(body)); res.once('error', reject);
    });
    req.once('error', reject); req.end(BODY);
  });
  return { gate, post, reachedDaemon, count: () => count };
}

test('live gated request is forwarded unchanged to a real Unix HTTP endpoint', { timeout: 2000 }, async t => {
  // Arrange / Act
  const f = await fixture(t), pending = f.post(); await f.gate.reached;
  // Assert
  assert.equal(f.count(), 0); f.gate.release(); assert.equal(await pending, 'created');
  assert.equal(f.count(), 1); assert.deepEqual(f.gate.observation(), { held: true, forwarded: true, cancelledBeforeForward: false });
});

test('client closure before forwarding is observed and the held request never reaches the daemon', { timeout: 2000 }, async t => {
  // Arrange
  const f = await fixture(t), client = new AbortController(), pending = assert.rejects(f.post(client.signal), /abort/i);
  await f.gate.reached; assert.equal(f.count(), 0);
  // Act / Assert
  client.abort(); await pending; await f.gate.clientClosed;
  f.gate.release(); await f.gate.close();
  f.gate.assertCancelled(); assert.equal(f.count(), 0);
});

test('closing a client after forwarding does not undo daemon acceptance', { timeout: 2000 }, async t => {
  // Arrange
  const f = await fixture(t, true), client = new AbortController(), pending = assert.rejects(f.post(client.signal), /abort/i);
  await f.gate.reached; f.gate.release(); await f.reachedDaemon;
  // Act / Assert: this is the in-flight daemon limitation, distinct from a held request.
  client.abort(); await pending; await f.gate.clientClosed;
  assert.equal(f.count(), 1); assert.deepEqual(f.gate.observation(), { held: true, forwarded: true, cancelledBeforeForward: false });
});
