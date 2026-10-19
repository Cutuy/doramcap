import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CpuStream, TopCpuStreams, MAX_GROUPS, KERNEL_SERIES, WindowBuffer, type Snapshot } from './stream';
function snapshot(sequence: number, runtime: bigint, overrides: Partial<Snapshot> = {}): Snapshot {
  return { sequence: String(sequence), session_id: 'a', source: 'synthetic', capture_unix_ns: '1792345678901234567', monotonic_end_ns: String(sequence * 100_000_000), dropped_batches: '0', capacity_overflow: '0', dictionary: [], records: [{ cgroup_id: '18446744073709551615', cpu: '0', flags: '1', runtime_ns: String(runtime) }], ...overrides };
}
const group = '18446744073709551615';
test('u64 delta stays precise, sequence gaps and counter resets are visible', () => {
  const stream = new CpuStream();
  const base = 18_000_000_000_000_000_000n;
  stream.ingest(snapshot(1, base), group, 1);
  stream.ingest(snapshot(3, base + 100_000_000n), group, 1.2);
  assert.equal(stream.buffer.rows()[1].values.get('0'), 50);
  assert.equal(stream.gaps, 1n);
  stream.ingest(snapshot(4, 0n), group, 1.3);
  assert.equal(stream.buffer.rows()[2].values.get('0'), null);
  stream.ingest(snapshot(1, 50n, { session_id: 'b' }), group, 1.4);
  assert.equal(stream.buffer.size, 1);
  assert.equal(stream.gaps, 0n);
});
test('prunes during silence and caps memory during a burst', () => {
  const buffer = new WindowBuffer();
  for (let i = 0; i < 10000; i++) buffer.push({ time: i / 1000, values: new Map() });
  assert.equal(buffer.size, 600);
  buffer.prune(41);
  assert.equal(buffer.size, 0);
});
test('unavailable counters do not become zero utilization; cgroup changes clear history', () => {
  const stream = new CpuStream();
  stream.ingest(snapshot(1, 0n), group, 1);
  const invalid = snapshot(2, 50_000_000n);
  invalid.records[0].flags = '65';
  stream.ingest(invalid, group, 1.1);
  assert.equal(stream.buffer.rows()[1].values.get('0'), null);
  stream.ingest(snapshot(3, 100_000_000n), 'other', 1.2);
  assert.equal(stream.buffer.size, 1);
  stream.prune(32);
  assert.equal(stream.buffer.size, 0);
});

test('ranks eight busiest groups independently on each CPU and preserves history', () => {
  const stream = new TopCpuStreams();
  const make = (seq: number) => snapshot(seq, 0n, { records: Array.from({length: 12}, (_, i) => [0, 1].map(cpu => ({
    cgroup_id: String(i), cpu: String(cpu), flags: '1', runtime_ns: String(seq * (cpu === 0 ? i : 11-i) * 1_000_000),
  }))).flat() });
  for (let i = 1; i <= 21; i++) stream.ingest(make(i), i / 10);
  assert.equal(stream.top.get('0')!.length, MAX_GROUPS);
  assert.equal(stream.top.get('0')![0], '11');
  assert.equal(stream.top.get('1')![0], '0');
  assert.ok(!stream.top.get('0')!.includes('0'));
  assert.equal(stream.buffers.get('0')!.size, 21);
  stream.refresh(8);
  assert.deepEqual(stream.top.get('0'), []);
  stream.refresh(33);
  assert.equal(stream.buffers.size, 0);
});
test('rankings exclude unavailable counters and reset with session changes', () => {
  const stream = new TopCpuStreams();
  stream.ingest(snapshot(1, 0n), 1);
  stream.ingest(snapshot(2, 10_000_000n), 2);
  assert.deepEqual(stream.top.get('0'), [group]);
  const bad = snapshot(3, 999_999_999n); bad.records[0].flags = '65';
  stream.ingest(bad, 8);
  assert.deepEqual(stream.top.get('0'), []);
  stream.ingest(snapshot(1, 0n, {session_id:'new'}), 9);
  assert.equal(stream.buffers.get('0')!.size, 1);
  assert.deepEqual(stream.top.get('0'), []);
});


test('virtual kernel counters stay separate from cgroup rankings', () => {
 const stream = new TopCpuStreams();
 const make = (seq: number, session_id = 'a'): Snapshot => snapshot(seq, BigInt(seq * 5_000_000), {
   session_id, task_collection_enabled: true, records: [
     ...snapshot(seq, BigInt(seq * 5_000_000)).records,
     { cgroup_id: KERNEL_SERIES, cpu: '0', flags: '129', runtime_ns: String(seq * 30_000_000) },
   ],
 });
 stream.ingest(make(1), 1);
 assert.equal(stream.buffers.get('0')!.rows()[0].values.get(KERNEL_SERIES), null);
 stream.ingest(make(2), 2);
 const point = stream.buffers.get('0')!.rows()[1];
 assert.equal(point.values.get(KERNEL_SERIES), 30);
 assert.equal(point.values.get(group), 5);
 assert.deepEqual(stream.top.get('0'), [group]);
 stream.ingest(make(1, 'b'), 3);
 assert.equal(stream.buffers.get('0')!.rows()[0].values.get(KERNEL_SERIES), null);
 stream.refresh(34);
 assert.equal(stream.buffers.size, 0);
});
