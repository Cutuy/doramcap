import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { ApiError, createClient, wire } from './api';
import { MEDIA_TYPE, LIMITS } from './generated/transport';

test('browser client rejects wrong media, status and malformed API responses', async () => {
  // Arrange
  const payload = wire.StartResponse.encode(wire.StartResponse.create({ runId: 'run' })).finish();
  const client = (status: number, type: string, bytes: Uint8Array) =>
    createClient(
      async () => new Response(new Uint8Array(bytes), { status, headers: { 'content-type': type } }),
    );
  // Act / Assert
  await assert.rejects(client(202, 'application/json', payload).start('run', 'key'), /response type/);
  await assert.rejects(client(200, MEDIA_TYPE, payload).start('run', 'key'), /response status/);
  await assert.rejects(client(202, MEDIA_TYPE, Uint8Array.of(0xff)).start('run', 'key'));
  await assert.rejects(client(202, MEDIA_TYPE, new Uint8Array(LIMITS.frameBytes + 1)).start('run', 'key'), /exceeds limit/);
});

test('the browser client preserves a Review rejection check', async () => {
  // Arrange
  const bytes = wire.ConsoleError.encode(wire.ConsoleError.create({ code: 'InvalidInput', message: 'raw evidence', check: 'configuration.rs:1' })).finish();
  const client = createClient(async () => new Response(new Uint8Array(bytes), { status: 400, headers: { 'content-type': MEDIA_TYPE } }));
  // Act / Assert
  await assert.rejects(client.prepare([]), (error: ApiError) => {
    assert.equal(error.code, 'InvalidInput');
    assert.equal(error.detail, 'raw evidence');
    assert.equal(error.check, 'configuration.rs:1');
    return true;
  });
});
