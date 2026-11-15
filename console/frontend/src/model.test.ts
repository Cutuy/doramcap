import { wire } from './api';
import { Phase } from './api';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { acceptsSnapshot, canStopRun, comparedFactValue, factLabel, factValue, latestRequest, outcomeLabels, pastRunLabel, requestKey, runAgainBlocked, toggleProfile } from './model';
import type { Snapshot } from './api';

test('comparison values retain small byte sizes and microsecond units', () => {
  // Arrange / Act / Assert
  assert.equal(factValue('write_policy', 'WRITE_POLICY_KEEP'), 'Keep with the run');
  assert.equal(factValue('write_policy', 'WRITE_POLICY_WRITE_BACK'), 'Write back');
  assert.equal(factValue('write_policy', 'WRITE_POLICY_DISCARD'), 'Discard');
  assert.equal(factValue('write_policy', 'WRITE_POLICY_FUTURE'), 'Unknown');
  assert.equal(factValue('recording_bytes', '32'), '32 B');
  assert.equal(factValue('recording_bytes', '32768'), '32.0 KiB');
  assert.equal(factValue('recording_bytes', '1048576'), '1.0 MiB');
  assert.equal(factLabel('rt_runtime_us'), 'RT runtime');
  assert.equal(factValue('rt_runtime_us', '950000'), '950000 µs');
});

test('rounded comparison collisions show exact native units without losing uint64 digits', () => {
  // Arrange
  const cases = [
    ['disk_free_bytes', ['22763326668', '22763322572'], ['22763326668 B', '22763322572 B']],
    ['uptime_seconds', ['1440', '1441'], ['1440 s', '1441 s']],
    ['duration_seconds', ['10.01', '10.02'], ['10.01 s', '10.02 s']],
    ['memory_total_bytes', ['18446744073709551614', '18446744073709551615'], ['18446744073709551614 B', '18446744073709551615 B']],
  ] as const;
  // Act / Assert
  for (const [name, values, expected] of cases) assert.deepEqual(values.map(value => comparedFactValue(name, value, [...values])), expected);
});

test('equal and visibly distinct comparison values keep their readable units', () => {
  // Arrange
  const cases = [
    ['disk_free_bytes', ['1048576', '1048576'], ['1.0 MiB', '1.0 MiB']],
    ['disk_free_bytes', ['1048576', '2097152'], ['1.0 MiB', '2.0 MiB']],
    ['uptime_seconds', ['1440', '1800'], ['0.4 hours', '0.5 hours']],
    ['uptime_seconds', ['', '1440'], ['', '0.4 hours']],
    ['disk_free_bytes', ['unavailable', '1048576'], ['unavailable', '1.0 MiB']],
  ] as const;
  // Act / Assert
  for (const [name, values, expected] of cases) assert.deepEqual(values.map(value => comparedFactValue(name, value, [...values])), expected);
});

test('profile changes and lost Start responses retain explicit intent', () => {
  // Arrange
  const firstKey = 'request-1';
  // Act / Assert
  assert.deepEqual(toggleProfile(['extra'], 'extra'), []);
  assert.deepEqual(toggleProfile([], 'extra'), ['extra']);
  assert.equal(requestKey(firstKey, () => 'request-2'), firstKey);
  assert.equal(requestKey('', () => firstKey), firstKey);
});

test('an older active run cannot replace the expected run', () => {
  // Arrange
  const old = wire.Status.fromObject({ phase: Phase.RUNNING, runId: 'old' });
  const current = wire.Status.fromObject({ phase: Phase.RUNNING, runId: 'new' });
  // Act / Assert
  assert.equal(acceptsSnapshot('new', old), false);
  assert.equal(acceptsSnapshot('new', { ...old, phase: Phase.FINISHED }), false);
  assert.equal(acceptsSnapshot('new', current), true);
});

test('execution and cleanup failures remain separate from saved recording', () => {
  // Arrange
  const outcome = wire.Outcome.fromObject({ execution: 'Failed', recordingFinalized: true, boundaryClean: false });
  // Act
  const labels = outcomeLabels(outcome);
  // Assert
  assert.deepEqual(labels, { execution: 'Failed', recording: 'Saved', cleanup: 'Unverified' });
  assert.equal(outcomeLabels(null).cleanup, 'Unknown');
});

test('Stop is available only during preparation/execution and remains disabled after shutdown starts', () => {
  // Arrange / Act / Assert
  for (const phase of [Phase.STARTING,Phase.RUNNING]) {
    assert.equal(canStopRun({phase,runId:'run'}),true);
    assert.equal(canStopRun({phase,runId:'run'},true),false);
  }
  for (const phase of [Phase.IDLE,Phase.PREPARING,Phase.REVIEWED,Phase.STOPPING,Phase.FINISHED,Phase.BLOCKED]) assert.equal(canStopRun({phase,runId:'run'}),false);
  assert.equal(canStopRun(null),false);
});


test('pre-launch and empty-run frames cannot replace an in-flight Start', () => {
  // Arrange
  const expected = 'starting-run';
  // Act / Assert
  for (const phase of [Phase.IDLE, Phase.PREPARING, Phase.REVIEWED]) {
    assert.equal(acceptsSnapshot(expected, wire.Status.fromObject({phase, runId: expected})), false);
    assert.equal(acceptsSnapshot(expected, wire.Status.fromObject({phase, runId: ''})), false);
  }
  for (const phase of [Phase.STARTING, Phase.RUNNING, Phase.STOPPING, Phase.FINISHED, Phase.BLOCKED]) {
    assert.equal(acceptsSnapshot(expected, wire.Status.fromObject({phase, runId: expected})), true);
    assert.equal(acceptsSnapshot(expected, wire.Status.fromObject({phase, runId: ''})), false);
  }
  assert.equal(acceptsSnapshot('', wire.Status.fromObject({phase:Phase.IDLE, runId:''})), true);
});

test('only the latest history request stays current', () => {
  // Arrange
  const requests = latestRequest();
  // Act
  const first = requests.begin(), second = requests.begin();
  // Assert: a filter change or refresh supersedes an in-flight page.
  assert.equal(first(), false);
  assert.equal(second(), true);
  // Act
  requests.cancel();
  // Assert
  assert.equal(second(), false);
});

test('Run again waits for any prepare or active run', () => {
  // Arrange
  const snapshot = (phase: Snapshot['phase'], runId = 'run') => ({ phase, runId });
  // Act / Assert
  assert.equal(runAgainBlocked(null, false), false);
  assert.equal(runAgainBlocked(null, true), true);
  assert.equal(runAgainBlocked(snapshot(Phase.PREPARING, ''), false), true);
  for (const phase of [Phase.STARTING, Phase.RUNNING, Phase.STOPPING]) assert.equal(runAgainBlocked(snapshot(phase), false), true);
  for (const phase of [Phase.IDLE, Phase.REVIEWED, Phase.FINISHED, Phase.BLOCKED]) assert.equal(runAgainBlocked(snapshot(phase), false), false);
});

test('a past run is labelled by its result', () => {
  // Arrange
  const past = (phase: Snapshot['phase'], execution?: string) => wire.Status.create({ phase, ...(execution ? { outcome: wire.Outcome.create({ execution }) } : {}) });
  // Act / Assert
  assert.equal(pastRunLabel(past(Phase.FINISHED, 'Completed')), 'Completed run');
  assert.equal(pastRunLabel(past(Phase.FINISHED, 'Failed')), 'Failed run');
  assert.equal(pastRunLabel(past(Phase.FINISHED, 'Cancelled')), 'Cancelled run');
  assert.equal(pastRunLabel(past(Phase.BLOCKED, 'Completed')), 'Cleanup unverified');
  assert.equal(pastRunLabel(past(Phase.RUNNING)), 'Interrupted run');
  assert.equal(pastRunLabel(null), 'Interrupted run');
});
