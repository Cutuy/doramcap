import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { Progress, MAX_PROGRESS_ROWS } from '../src/progress.js';
import { workflow } from '../src/native.js';
const { STARTED, SUCCEEDED } = workflow.Event.State;
const event = (id: number, sequence: number, state = STARTED, parent?: number) =>
  workflow.Event.create({
    version: 1,
    session: 'run',
    executor: 'local',
    invocation: id,
    sequence,
    step: 'compose.observe',
    state,
    parent,
  });

test('progress replaces invocations, redacts private data and exposes sequence loss', () => {
  // Arrange
  const progress = new Progress();
  // Act
  progress.apply({ ...event(1, 1), payloadJson: 'secret', error: '/private/host' });
  progress.apply(event(1, 3, SUCCEEDED));
  // Assert
  assert.equal(progress.rows.length, 1);
  assert.equal(progress.rows[0].payloadJson, null);
  assert.equal(progress.gaps, 1n);
  assert.equal(progress.incomplete, true);
  // Act
  progress.apply({ ...event(2, 4), error: '/private/host' });
  // Assert
  assert.equal(progress.rows[1].error, 'step failed');
});

test('active rows survive capacity pressure and completed children can be replaced', () => {
  // Arrange
  const progress = new Progress();
  for (let id = 1; id <= MAX_PROGRESS_ROWS; id++) progress.apply(event(id, id));
  // Act
  progress.apply(event(MAX_PROGRESS_ROWS + 1, MAX_PROGRESS_ROWS + 1));
  // Assert
  assert.equal(progress.rows.length, MAX_PROGRESS_ROWS);
  assert.equal(progress.overflow, 1);
  // Act
  progress.apply(event(1, MAX_PROGRESS_ROWS + 2, SUCCEEDED, 10));
  progress.apply(event(MAX_PROGRESS_ROWS + 2, MAX_PROGRESS_ROWS + 3));
  // Assert
  assert.equal(progress.rows.length, MAX_PROGRESS_ROWS);
  assert.equal(
    progress.rows.some((row) => Number(row.invocation) === 1),
    false,
  );
  assert.equal(progress.truncated, 1);
});

test('unknown event versions make history incomplete without creating display rows', () => {
  // Arrange
  const progress = new Progress();
  // Act
  progress.apply({ ...event(1, 1), version: 999 });
  // Assert
  assert.equal(progress.rows.length, 0);
  assert.equal(progress.incomplete, true);
});
