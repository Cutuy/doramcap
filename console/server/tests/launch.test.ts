import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { test } from 'node:test';
import { launchInstructions, listenConsole, portNumber } from '../src/launch.js';

test('occupied ports retry with the actual browser origin and explicit ports fail', async () => {
  // Arrange
  const occupied = createServer();
  occupied.listen(0, '127.0.0.1');
  await once(occupied, 'listening');
  const address = occupied.address();
  assert.ok(address && typeof address !== 'string');
  const create = (origin: string) => createServer((_req, response) => response.end(origin));
  try {
    // Act
    const launched = await listenConsole(create, address.port);
    try {
      // Assert
      assert.notEqual(launched.port, address.port);
      assert.equal(await (await fetch(launched.origin)).text(), launched.origin);
      assert.equal((launched.server.address() as { address: string }).address, '127.0.0.1');
    } finally { launched.server.closeAllConnections(); launched.server.close(); }
    // Act / Assert
    await assert.rejects(listenConsole(create, address.port, undefined, false), { code: 'EADDRINUSE' });
    const forwarded = await listenConsole(create, address.port, 45231);
    try {
      assert.equal(forwarded.origin, 'http://127.0.0.1:45231');
      const instructions = launchInstructions(forwarded.origin, forwarded.port, 'token', 'alice@host');
      assert.ok(instructions.includes(`-L 127.0.0.1:45231:127.0.0.1:${forwarded.port}`));
      assert.ok(instructions.includes('Open: http://127.0.0.1:45231/#access=token'));
    } finally { forwarded.server.close(); }
  } finally { occupied.close(); }
});

test('launch options reject invalid ports and SSH destinations', () => {
  // Arrange / Act / Assert
  for (const value of ['0', '8080oops', '65536', '22']) assert.throws(() => portNumber(value));
  assert.equal(portNumber('43187'), 43187);
  for (const destination of ['-oProxyCommand=bad', 'host;command', 'user@host\ncommand'])
    assert.throws(() => launchInstructions('http://127.0.0.1:43187', 43187, 'secret', destination));
});
