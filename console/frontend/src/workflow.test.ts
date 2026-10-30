import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { elapsedDuration, isLifecycleStep, stepTimes, stepName, isHealthCheck } from './workflow';
import type { Step } from './api';

const step = (overrides: Partial<Step>): Step => ({ session: 's', executor: 'local', invocation: '9007199254740993', step: 'renamed::PreparationImpl', descriptor: { id: 'compose.prepare', displayName: 'Prepare project', retention: 'KEEP', kind: 'OPERATION' }, state: 'SUCCEEDED', elapsedNs: '9007199254740999', durationNs: '9007199254740993', ...overrides });

test('classifies top-level lifecycle steps separately from periodic and nested events', () => {
  // Arrange / Act / Assert
  assert.equal(isLifecycleStep(step({})), true);
  assert.equal(isLifecycleStep(step({ parent: '1' })), false);
  assert.equal(isLifecycleStep(step({ descriptor: { id: 'compose.observe', displayName: 'Observe', retention: 'RECENT', kind: 'OBSERVATION' } })), false);
});

test('timestamps retain exact 64-bit subtraction and running elapsed time advances', () => {
  // Arrange
  const long = step({});
  const running = step({ elapsedNs: '2000000000', durationNs: '0', state: 'STARTED' });
  // Act / Assert
  assert.equal(stepTimes(long).start, 6e-9);
  assert.equal(elapsedDuration(running, 5), 3);
  assert.equal(elapsedDuration(step({ elapsedNs: '5000000000', durationNs: '3000000000' }), 9), 3);
});

test('labels and retention follow descriptors across Rust renames and new workflows', () => {
  // Arrange
  const custom = step({ step: 'other::RenamedType', descriptor: { id: 'custom.stage', displayName: 'Custom stage', retention: 'KEEP', kind: 'OBSERVATION' } });
  // Act / Assert
  assert.equal(stepName(custom), 'Custom stage');
  assert.equal(isLifecycleStep(custom), true);
  assert.equal(isHealthCheck(custom), true);
  assert.equal(isLifecycleStep(step({ step: 'compose::PrepareRun', descriptor: null })), false);
});
