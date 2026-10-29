import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tmpdir } from 'node:os';
import { Experiment, serverFor, splitFrames, validate } from '../viewer.ts';
import { doramcap } from '../frontend/generated/wire.js';
const pb: any = doramcap.cuda_viewer.v1;

test('rejects invalid controls', () => {
  // Arrange
  const invalid = [{ frames: 0 }, { consumers: 9 }, { bytes: -1 }, { slots: true }, { checksum: 'yes' }, { failure: 'shell' }, { frames: 4, slots: 4 }];
  // Act / Assert
  for (const config of invalid) assert.throws(() => validate(config as never));
});
test('retains partial batches and rejects invalid lengths', () => {
  // Arrange
  const framed = Buffer.from([3, 0, 0, 0, 97, 98, 99, 2, 0, 0, 0, 120, 121]);
  // Act / Assert
  for (let cut = 0; cut <= framed.length; cut++) {
    const [first, tail] = splitFrames(framed.subarray(0, cut)); const [second, remaining] = splitFrames(Buffer.concat([tail, framed.subarray(cut)]));
    assert.deepEqual(first.concat(second).map(x => x.toString()), ['abc', 'xy']); assert.equal(remaining.length, 0);
  }
  assert.throws(() => splitFrames(Buffer.from([0, 0, 80, 0])));
});
test('serves the binary protocol, incremental state, and private hosted snapshots', async () => {
  // Arrange
  const experiment = new Experiment('dora', tmpdir(), 'run.mjs', true);
  experiment.run = 'fixture'; experiment.serial = 1; experiment.rows = [{ id: 1, role: 'producer', payload: Buffer.from('opaque') }]; experiment.log = 'private';
  let received: unknown; experiment.start = async config => { received = config; };
  const server = serverFor(experiment, tmpdir(), () => false);
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  try {
    // Act
    const response = await fetch(`${base}/api/state`); const decoded = pb.Snapshot.decode(new Uint8Array(await response.arrayBuffer()));
    // Assert
    assert.equal(response.headers.get('content-type'), 'application/x-protobuf'); assert.equal(Buffer.from(decoded.batches[0].payload).toString(), 'opaque');
    assert.equal(Object.hasOwn(decoded, 'log'), false); assert.equal(decoded.artifacts.length, 0);
    // Act / Assert
    assert.equal((await fetch(`${base}/api/state?since=1&run=fixture&state=idle`)).status, 204);
    assert.equal((await fetch(`${base}/artifact/run.log`)).status, 404);
    const config = pb.RunConfig.encode({ frames: 24, slots: 2, consumers: 3, bytes: 4097, checksum: true }).finish();
    assert.equal((await fetch(`${base}/api/run`, { method: 'POST', headers: { 'Content-Type': 'application/x-protobuf', Origin: base }, body: Uint8Array.from(config).buffer })).status, 202);
    assert.equal((received as { frames: number }).frames, 24);
    assert.equal((await fetch(`${base}/api/run`, { method: 'POST', headers: { 'Content-Type': 'application/x-protobuf', Origin: 'http://elsewhere' }, body: Uint8Array.from(config).buffer })).status, 403);
  } finally { server.close(); }
});
