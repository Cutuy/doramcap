import { workflow } from './api';
import { StepState } from './api';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { elapsedDuration, summarySteps, evictionIndex, stepTimes, stepName } from './workflow';
import type { Step } from './api';

const step = (overrides: Record<string, unknown>): Step => (workflow.Event.fromObject({ session: 's', executor: 'local', invocation: '9007199254740993', step: 'compose.prepare', displayName: 'Prepare project', state: StepState.SUCCEEDED, elapsedNs: '9007199254740999', durationNs: '9007199254740993', ...overrides }));

test('summary groups top-level invocations by stable identity and executor', () => {
  // Arrange
  const first = step({});
  const latest = step({ invocation: '2', displayName: 'Updated label' });
  const child = step({ invocation: '3', parent: '2' });
  const remote = step({ invocation: '4', executor: 'remote' });
  // Act
  const summary = summarySteps([first, latest, child, remote]);
  // Assert
  assert.deepEqual(summary, [latest, remote]);
});

test('bounded history evicts completed rows when every step identity is unique', () => {
  // Arrange
  const rows = Array.from({length: 512}, (_, i) => step({ invocation: String(i), step: `step.${i}`, displayName: 'Step' }));
  const incoming = step({ step: 'new', displayName: 'New step' });
  // Act / Assert
  assert.equal(evictionIndex(rows, incoming), 0);
  assert.equal(evictionIndex(rows.map(row => workflow.Event.fromObject({...row, state: StepState.STARTED})), incoming), -1);
});

test('timestamps retain exact 64-bit subtraction and running elapsed time advances', () => {
  // Arrange
  const long = step({});
  const running = step({ elapsedNs: '2000000000', durationNs: '0', state: StepState.STARTED });
  // Act / Assert
  assert.equal(stepTimes(long).start, 6e-9);
  assert.equal(elapsedDuration(running, 5), 3);
  assert.equal(elapsedDuration(step({ elapsedNs: '5000000000', durationNs: '3000000000' }), 9), 3);
});

test('labels are independent of step identity', () => {
  // Arrange
  const custom = step({ step: 'custom.stage', displayName: 'Custom stage' });
  // Act / Assert
  assert.equal(stepName(custom), 'Custom stage');
  assert.equal(stepName(step({ step: 'compose::PrepareRun', displayName: null })), 'PrepareRun');
});
