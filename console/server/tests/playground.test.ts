import assert from 'node:assert/strict';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { checkPlayground } from '../scripts/check-playground.js';
import { doramcap } from '../src/generated/console.js';

const wire = doramcap.console.v1;

for (const failure of ['start', 'live']) test(`playground check stops the run and awaits cleanup after ${failure} fails`, async () => {
  // Arrange: even a failed Start response can follow a successful admission.
  const directory = await mkdtemp(join(tmpdir(), 'console-playground-check-'));
  const runId = 'a'.repeat(32);
  let started = false, stopped = false, cleanupObserved = false;
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = Buffer.concat(chunks);
    switch (request.url) {
      case '/api/session':
        assert.equal(request.headers.authorization, 'Bearer private-test-token');
        response.writeHead(204, { 'set-cookie': 'session=test; HttpOnly' });
        response.end(); return;
      case '/api/prepare':
        response.end(wire.Review.encode({ runId, preparedId: runId }).finish()); return;
      case '/api/start':
        started = true;
        if (failure === 'start') {
          response.writeHead(500);
          response.end(wire.ConsoleError.encode({ code: 'Internal', message: 'start response failed' }).finish());
        } else response.end(wire.StartResponse.encode({ runId }).finish());
        return;
      case '/api/live':
        response.writeHead(503); response.end(); return;
      case '/api/stop':
        assert.equal(wire.StopRequest.decode(body).runId, runId);
        stopped = true;
        response.end(wire.StopResponse.encode({ accepted: true }).finish()); return;
      case '/api/current':
        cleanupObserved ||= stopped;
        response.end(wire.Status.encode({ phase: stopped ? wire.Phase.FINISHED : wire.Phase.RUNNING, runId }).finish()); return;
      default: response.writeHead(404); response.end();
    }
  });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const launchFile = join(directory, 'launch.txt');
  await writeFile(launchFile, `Open: http://127.0.0.1:${address.port}/#access=private-test-token\n`, { mode: 0o600 });
  try {
    // Act
    await assert.rejects(checkPlayground({ launchFile }), failure === 'start' ? /start response failed/ : /Live stream failed/);
    // Assert: cleanup finishes before the original verification failure escapes.
    assert.ok(started && stopped && cleanupObserved);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await rm(directory, { recursive: true, force: true });
  }
});
