import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { LiveRelay } from '../src/live.js';
import { live } from '../src/native.js';
import { LIMITS } from '../src/generated/transport.js';

const encode = (stream: string, sequence: number) =>
  Buffer.from(live.Frame.encode({ stream, sequence, payload: Buffer.from([sequence]) }).finish());

test('relay replays the latest frame per stream and forwards frames byte-for-byte', () => {
  // Arrange
  const relay = new LiveRelay('ws://127.0.0.1:1/stream');
  const first = encode('cpu_0', 1), second = encode('cpu_0', 2), fault = encode('watchdog_0', 1);
  relay.receive(first);
  relay.receive(second);
  relay.receive(fault);
  const seen: Buffer[] = [];
  // Act
  const unsubscribe = relay.subscribe((frame) => seen.push(frame));
  const live = encode('cpu_0', 3);
  relay.receive(live);
  unsubscribe();
  relay.receive(encode('cpu_0', 4));
  // Assert
  assert.deepEqual(seen, [second, fault, live]);
});

test('relay rejects malformed frames and forgets frames when the run closes', () => {
  // Arrange
  const relay = new LiveRelay('ws://127.0.0.1:1/stream');
  relay.receive(encode('cpu_0', 1));
  // Act / Assert
  assert.equal(relay.receive(Buffer.from([0xff, 0xff])), false);
  assert.equal(relay.receive(Buffer.from(live.Frame.encode({ sequence: 1 }).finish())), false);
  const largest = Buffer.alloc(LIMITS.bridgePayloadBytes);
  assert.equal(relay.receive(Buffer.from(live.Frame.encode({ stream: 'cpu_0', sequence: 2, payload: largest }).finish())), true);
  assert.equal(relay.receive(Buffer.from(live.Frame.encode({ stream: 'cpu_0', sequence: 3, payload: Buffer.alloc(LIMITS.bridgePayloadBytes + 1) }).finish())), false);
  relay.close();
  const seen: Buffer[] = [];
  relay.subscribe((frame) => seen.push(frame));
  assert.equal(seen.length, 0);
});
