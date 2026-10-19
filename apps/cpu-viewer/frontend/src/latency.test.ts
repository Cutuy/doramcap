import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clockSample, browserDelay } from './latency';
test('server epoch maps to browser monotonic time independently of browser wall clock', () => {
  const sample = clockSample('1800000000000000000', 100, 120);
  assert.equal(sample.uncertaintyMs, 10);
  assert.equal(browserDelay('1800000000050000000', 180, sample), 20);
});
test('stale calibration and impossible future captures are unavailable', () => {
  const sample = clockSample('1800000000000000000', 100, 120);
  assert.equal(browserDelay('1800000001000000000', 180, sample), null);
  assert.equal(browserDelay('1800000000000000000', 100000, sample), null);
  assert.throws(() => clockSample('bad', 100, 120));
});
