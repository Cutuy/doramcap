import { wire } from './api';
import { Phase } from './api';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { acceptsSnapshot, canStopRun, latestRequest, outcomeLabels, requestKey, runAgainBlocked, toggleProfile } from './model';
import type { Snapshot } from './api';

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
  const snapshot = (phase: Phase, runId = 'run') => ({ phase, runId });
  // Act / Assert
  assert.equal(runAgainBlocked(null, false), false);
  assert.equal(runAgainBlocked(null, true), true);
  assert.equal(runAgainBlocked(snapshot(Phase.PREPARING, ''), false), true);
  for (const phase of [Phase.STARTING, Phase.RUNNING, Phase.STOPPING]) assert.equal(runAgainBlocked(snapshot(phase), false), true);
  for (const phase of [Phase.IDLE, Phase.REVIEWED, Phase.FINISHED, Phase.BLOCKED]) assert.equal(runAgainBlocked(snapshot(phase), false), false);
});
