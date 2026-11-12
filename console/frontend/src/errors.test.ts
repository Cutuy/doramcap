import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { ApiError } from './api';
import { errorMessage } from './errors';

test('each Review rejection selects Console wording and identifies its subject', () => {
  // Arrange
  const cases = [
    ['EmptySelection', '', 'Select a profile'],
    ['NoComposeFile', '', 'Add a Compose file'],
    ['UnresolvedEnvironment', '', 'Define the environment'],
    ['ImageNotPrepared', 'worker', 'local image for worker'],
    ['AnonymousVolume', 'worker', 'volume in worker'],
    ['ImageVolume', 'worker: /data', 'image volume worker: /data'],
    ['ZeroReplicas', 'worker', 'replica count for worker'],
    ['NameOccupied', 'taken', 'container name taken'],
    ['ReservedLabel', 'io.doramcap.run', 'label io.doramcap.run'],
    ['UnscopedResource', 'networks/shared', 'Declare networks/shared external'],
    ['ForeignResource', 'volume/other', 'resource volume/other belongs'],
    ['NoJournal', 'worker, recorder', 'unavailable for worker, recorder'],
  ];
  for (const [reason, subject, expected] of cases) {
    // Act
    const message = errorMessage(new ApiError('InvalidInput', 'raw backend text', reason, subject));
    // Assert
    assert.ok(message.includes(expected), `${reason}: ${message}`);
    assert.ok(!message.includes('raw backend text'));
  }
});

test('unknown reasons and ordinary failures retain the generic message and raw detail', () => {
  // Arrange / Act / Assert
  for (const reason of ['', 'FutureRejection', 'toString', '__proto__']) {
    assert.equal(errorMessage(new ApiError('InvalidInput', 'raw cause', reason, 'worker')),
      'The project could not be reviewed. Check its Compose files and the selected profiles.\nraw cause');
  }
  assert.equal(errorMessage(new ApiError('Busy', 'queue full', 'NoJournal', 'worker')),
    'The Console is busy. Try again.\nqueue full');
  assert.equal(errorMessage(new Error('offline')), 'Connection lost; run may still be active.');
});
