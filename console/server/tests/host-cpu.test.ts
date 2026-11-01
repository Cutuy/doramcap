import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { cpuUsage } from '../src/host-cpu.js';

const core = (user: number, idle: number) => ({
  model: 'test CPU', speed: 1000, times: { user, idle, nice: 0, sys: 0, irq: 0 },
});

test('host CPU utilization uses core percent and preserves idle zero', () => {
  // Arrange
  const before = [core(0, 0), core(0, 0)];
  // Act / Assert
  assert.deepEqual(cpuUsage(before, [core(100, 100), core(100, 100)]), { cpuPercent: 100, cpuCores: 2 });
  assert.equal(cpuUsage(before, [core(100, 0), core(100, 0)]).cpuPercent, 200);
  assert.equal(cpuUsage(before, [core(0, 100), core(0, 100)]).cpuPercent, 0);
});

test('warm-up, unchanged counters, resets and CPU changes remain unavailable', () => {
  // Arrange
  const before = [core(100, 100)];
  // Act / Assert
  for (const after of [before, [], [core(1, 1)], [core(200, 200), core(200, 200)]])
    assert.equal(cpuUsage(before, after).cpuPercent, null);
  assert.equal(cpuUsage([], before).cpuPercent, null);
});
