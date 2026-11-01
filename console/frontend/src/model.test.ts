import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { acceptsSnapshot, canStopRun, outcomeLabels, requestKey, toggleProfile } from './model';
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
  const old = { phase: 'RUNNING', runId: 'old' } as Snapshot;
  const current = { phase: 'RUNNING', runId: 'new' } as Snapshot;
  // Act / Assert
  assert.equal(acceptsSnapshot('new', old), false);
  assert.equal(acceptsSnapshot('new', { ...old, phase: 'FINISHED' }), false);
  assert.equal(acceptsSnapshot('new', current), true);
});

test('execution and cleanup failures remain separate from saved recording', () => {
  // Arrange
  const outcome = { execution: 'Failed', recordingFinalized: true, boundaryClean: false };
  // Act
  const labels = outcomeLabels(outcome);
  // Assert
  assert.deepEqual(labels, { execution: 'Failed', recording: 'Saved', cleanup: 'Unverified' });
  assert.equal(outcomeLabels(null).cleanup, 'Unknown');
});

test('Stop is available only during preparation/execution and remains disabled after shutdown starts', () => {
  // Arrange / Act / Assert
  for (const phase of ['STARTING','RUNNING']) {
    assert.equal(canStopRun({phase,runId:'run'}),true);
    assert.equal(canStopRun({phase,runId:'run'},true),false);
  }
  for (const phase of ['IDLE','PREPARING','REVIEWED','STOPPING','FINISHED','BLOCKED']) assert.equal(canStopRun({phase,runId:'run'}),false);
  assert.equal(canStopRun(null),false);
});


test('pre-launch and empty-run frames cannot replace an in-flight Start', () => {
  // Arrange
  const expected = 'starting-run';
  // Act / Assert
  for (const phase of ['IDLE', 'PREPARING', 'REVIEWED']) {
    assert.equal(acceptsSnapshot(expected, {phase, runId: expected} as Snapshot), false);
    assert.equal(acceptsSnapshot(expected, {phase, runId: ''} as Snapshot), false);
  }
  for (const phase of ['STARTING', 'RUNNING', 'STOPPING', 'FINISHED', 'BLOCKED']) {
    assert.equal(acceptsSnapshot(expected, {phase, runId: expected} as Snapshot), true);
    assert.equal(acceptsSnapshot(expected, {phase, runId: ''} as Snapshot), false);
  }
  assert.equal(acceptsSnapshot('', {phase:'IDLE', runId:''} as Snapshot), true);
});
