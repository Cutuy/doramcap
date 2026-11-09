import assert from 'node:assert/strict';
import { test } from 'node:test';
import { successfulBuild, workerHost } from './submit-run-only.mts';

test('SSH workers accept account/host names and reject command and option injection', () => {
  // Arrange / Act / Assert
  assert.equal(workerHost('mac-mini-ubuntu24@100.64.0.22'), 'mac-mini-ubuntu24@100.64.0.22');
  assert.equal(workerHost('arm-builder'), 'arm-builder');
  for (const host of ['-oProxyCommand=id', '-user@host', 'user@host;id', 'user@$(id)', 'host name', 'user@-host']) {
    assert.throws(() => workerHost(host), /Invalid SSH worker/);
  }
});
test('only a successful build of the exact queued task permits transfer', () => {
  // Arrange
  const status = JSON.stringify({ tasks: { 7: { status: { Done: { result: 'Success' } } },
    8: { status: { Done: { result: { Failed: 1 } } } }, 9: { status: 'Running' } } });
  // Act / Assert
  successfulBuild(status, '7');
  for (const id of ['8', '9', '10']) assert.throws(() => successfulBuild(status, id), /did not succeed/);
  assert.throws(() => successfulBuild(status, '7;id'), /Invalid Pueue task ID/);
});
