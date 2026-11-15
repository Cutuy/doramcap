import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { ApiError, createClient, wire } from './api';
import { MEDIA_TYPE, LIMITS, SYSTEM_APP_SELECTION_CHECK } from './generated/transport';

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
  for (const check of ['configuration.rs:1', SYSTEM_APP_SELECTION_CHECK, '']) {
    const bytes = wire.ConsoleError.encode(wire.ConsoleError.create({ code: 'InvalidInput', message: 'raw evidence', check })).finish();
    const client = createClient(async () => new Response(new Uint8Array(bytes), { status: 400, headers: { 'content-type': MEDIA_TYPE } }));
    // Act / Assert
    await assert.rejects(client.prepare([]), (error: ApiError) => {
      assert.equal(error.code, 'InvalidInput');
      assert.equal(error.detail, 'raw evidence');
      assert.equal(error.check, check);
      return true;
    });
  }
});

test('Review requests preserve project profiles, comparison baseline and selection presence', async () => {
  // Arrange
  const profiles = ['gpu', 'sensors'], baselineRunId = 'a'.repeat(32);
  for (const systemApps of [undefined, [], ['cpu-monitor']]) {
    const appProfiles: Record<string, string> = systemApps?.length ? { 'cpu-monitor': 'record' } : {};
    let received: typeof wire.PrepareRequest.prototype | undefined;
    const client = createClient(async (_input, init) => {
      assert.ok(init?.body instanceof Blob);
      received = wire.PrepareRequest.decode(new Uint8Array(await init.body.arrayBuffer()));
      return new Response(new Uint8Array(wire.Review.encode(wire.Review.create()).finish()), { headers: { 'content-type': MEDIA_TYPE } });
    });
    // Act
    await client.prepare(profiles, systemApps, appProfiles, baselineRunId);
    // Assert
    assert.deepEqual(received?.profiles, profiles);
    assert.equal(received?.baselineRunId, baselineRunId);
    assert.deepEqual(received?.systemApps?.ids, systemApps);
    if (systemApps) assert.deepEqual(received?.systemApps?.profiles, appProfiles);
    else assert.equal(received?.systemApps, null);
  }
});
