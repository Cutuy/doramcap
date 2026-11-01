import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { openSync, fstatSync } from 'node:fs';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { test } from 'node:test';
import { sessionCookie } from './session.js';
import { createConsoleServer } from '../src/http.js';
import type { ConsoleBackend } from '../src/native.js';

test('streams large evidence and closes descriptors on disconnect', async () => {
  // Arrange
  const dir = await mkdtemp(join(tmpdir(), 'console-stream-'));
  const path = join(dir, 'cpu.mcap');
  const contents = Buffer.alloc(8 * 1024 * 1024, 0x5a);
  await writeFile(path, contents);
  let lastFd = -1;
  let slow = false;
  let opened: (() => void) | undefined;
  const native = {
    openArtifact: async () => { if (slow) { opened?.(); await delay(100); } lastFd = openSync(path, 'r'); return lastFd; },
  } as unknown as ConsoleBackend;
  const server = createConsoleServer({ backend: native, secret: 'secret', origin: 'http://console.test', assets: dir });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const url = `http://127.0.0.1:${address.port}/api/artifacts/${'a'.repeat(32)}/cpu`;
  const cookie = await sessionCookie(new URL(url).origin);
  try {
    // Act
    const response = await fetch(url, { headers: { cookie } });
    const downloaded = Buffer.from(await response.arrayBuffer());
    // Assert
    assert.deepEqual(downloaded, contents);
    for (let i = 0; i < 100; i++) { try { fstatSync(lastFd); await delay(10); } catch { break; } }
    assert.throws(() => fstatSync(lastFd));
    // Act: disconnect while the next transfer is in flight.
    const controller = new AbortController();
    const partial = await fetch(url, { headers: { cookie }, signal: controller.signal });
    const reader = partial.body!.getReader();
    await reader.read();
    controller.abort();
    await reader.cancel().catch(() => {});
    // Assert
    for (let i = 0; i < 100; i++) { try { fstatSync(lastFd); await delay(10); } catch { break; } }
    assert.throws(() => fstatSync(lastFd));
    // Act: disconnect while the native opener is still pending.
    slow = true;
    const started = new Promise<void>(resolve => { opened = resolve; });
    const earlyAbort = new AbortController();
    const pending = fetch(url, { headers: { cookie }, signal: earlyAbort.signal }).catch(() => null);
    await started;
    earlyAbort.abort();
    await pending;
    await delay(200);
    // Assert: the later-returned descriptor was closed too.
    assert.throws(() => fstatSync(lastFd));
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await rm(dir, { recursive: true, force: true });
  }
});
