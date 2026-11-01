import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { doramcap } from './generated/console';
import { Health, Kind, LiveModel } from './streams';

const telemetry = doramcap.telemetry.v1;
const fault = doramcap.fault.v1;
const stream = (id: string, kind: number) => doramcap.console.v1.LiveStream.create({ id, appId: id, kind });
const frame = (stream: string, sequence: number, payload: Uint8Array) => doramcap.console.live.v1.Frame.create({ stream, sequence, payload });
const cpuPacket = (sequence: number, runtimeNs: number, sessionId = 's') =>
  telemetry.Packet.encode(telemetry.Packet.fromObject({ snapshot: { version: 2, sessionId, snapshot: {
    sequence, captureUnixNs: sequence * 1e9, monotonicEndNs: sequence * 1e9,
    records: [0, 1].map(cpu => ({ cgroupId: 1, cpu, flags: 17, runtimeNs })),
  } } })).finish();
const epoch = new Uint8Array(16).fill(1), source = new Uint8Array(16).fill(2);
const state = (condition: number, health: number) => ({ key: { source, condition }, health, definition: { component: 'camera', code: `E${condition}`, severity: 2 } });
const publication = (value: object) => fault.AggregationPublication.encode(fault.AggregationPublication.fromObject({ version: 1, ...value })).finish();

test('CPU totals derive from consecutive root counters and skipped frames count as gaps', () => {
  // Arrange
  const model = new LiveModel();
  model.configure('run', [stream('cpu_0', Kind.CPU_ACCOUNTING)]);
  // Act
  model.receive(frame('cpu_0', 1, cpuPacket(1, 0)));
  model.receive(frame('cpu_0', 2, cpuPacket(2, 500_000_000)));
  model.receive(frame('cpu_0', 5, cpuPacket(3, 1_500_000_000)));
  const view = model.view();
  // Assert
  assert.deepEqual(view.cpu!.history.map(sample => sample.percent), [100, 200]);
  assert.equal(view.cpu!.cores, 2);
  assert.equal(view.gaps, 2);
});

test('a restarted CPU monitor starts a new baseline and keeps updating', () => {
  // Arrange
  const model = new LiveModel();
  model.configure('run', [stream('cpu_0', Kind.CPU_ACCOUNTING)]);
  model.receive(frame('cpu_0', 1, cpuPacket(10, 0)));
  model.receive(frame('cpu_0', 2, cpuPacket(11, 500_000_000)));
  // Act
  model.receive(frame('cpu_0', 3, cpuPacket(1, 0, 'restarted')));
  model.receive(frame('cpu_0', 4, cpuPacket(2, 1_000_000_000, 'restarted')));
  // Assert
  assert.deepEqual(model.view().cpu!.history.map(sample => sample.percent), [100, 200]);
  assert.equal(model.view().cpu!.sequence, '2');
});

test('frames for unknown streams, duplicates and malformed payloads are ignored; a new run resets state', () => {
  // Arrange
  const model = new LiveModel();
  model.configure('run', [stream('cpu_0', Kind.CPU_ACCOUNTING)]);
  model.receive(frame('cpu_0', 1, cpuPacket(1, 0)));
  // Act / Assert
  assert.equal(model.receive(frame('cpu_0', 1, cpuPacket(1, 0))), false);
  assert.equal(model.receive(frame('other_0', 1, cpuPacket(1, 0))), false);
  assert.equal(model.receive(frame('cpu_0', 2, Uint8Array.of(0xff))), false);
  model.configure('next', [stream('cpu_0', Kind.CPU_ACCOUNTING)]);
  assert.equal(model.view().cpu!.history.length, 0);
  assert.equal(model.view().faults, undefined);
});

test('fault snapshots replace state, contiguous updates amend it and revision gaps mark it stale', () => {
  // Arrange
  const model = new LiveModel();
  model.configure('run', [stream('watchdog_0', Kind.FAULT_PUBLICATION)]);
  assert.equal(model.view().faults!.stale, true);
  // Act
  model.receive(frame('watchdog_0', 1, publication({ snapshot: { epoch, revision: 4, sources: [{ source, connected: true, conditions: [state(1, Health.HEALTH_ACTIVE), state(2, Health.HEALTH_HEALTHY)] }] } })));
  const snapshot = model.view().faults!;
  model.receive(frame('watchdog_0', 2, publication({ updates: { epoch, updates: [{ epoch, revision: 5, kind: fault.UpdateKind.UPDATE_KIND_TRANSITION, key: { source, condition: 2 }, state: state(2, Health.HEALTH_ACTIVE) }] } })));
  const updated = model.view().faults!;
  model.receive(frame('watchdog_0', 3, publication({ updates: { epoch, updates: [{ epoch, revision: 7, kind: fault.UpdateKind.UPDATE_KIND_RETIRED, key: { source, condition: 1 } }] } })));
  const gapped = model.view().faults!;
  // Assert
  assert.deepEqual([snapshot.sources, snapshot.connected, snapshot.stale], [1, 1, false]);
  assert.deepEqual(snapshot.conditions.map(condition => condition.code), ['E1']);
  assert.deepEqual(updated.conditions.map(condition => condition.code), ['E1', 'E2']);
  assert.equal(updated.stale, false);
  assert.deepEqual(gapped.conditions.map(condition => condition.code), ['E2']);
  assert.equal(gapped.stale, true);
});
