import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, open, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { capabilities, execution, launch, systemd } from '../src/native.js';

test('native inspection carries host shm metadata separately from reviewed inputs', async (t) => {
  // Arrange: no unit is started; this exercises the actual addon inspection boundary.
  const root = await mkdtemp(join(tmpdir(), 'console-host-shm-'));
  const directory = await mkdtemp('/dev/shm/console-host-shm-');
  const path = join(directory, 'payload');
  const payload = await open(path, 'w', 0);
  const unit = join(root, 'sample.service');
  await writeFile(unit, '[Service]\nExecStart=/bin/true\n');
  const owner = await capabilities.open('', 1);
  try {
    const request = execution.RunRequest.create({
      run: launch.RunPlan.create({
        id: 'host-observation', repositoryRoot: root, artifacts: join(root, 'artifacts'),
        readinessSeconds: 1, stopSeconds: 1,
      }),
      systemd: systemd.SystemdPlan.create({ unitFiles: [unit] }),
    });
    let reviewed: typeof execution.Inspection.prototype.systemdSnapshot;
    for (const size of [4096, 2 ** 40]) {
      await payload.truncate(size);
      // Act
      const start = performance.now();
      const inspected = await owner.inspect(request);
      t.diagnostic(`Native inspection with ${size} byte shm object: ${(performance.now() - start).toFixed(1)} ms`);
      const capture = execution.RunCapture.decode(execution.RunCapture.encode({ before: inspected.machine }).finish());
      // Assert
      assert.equal(capture.before?.hostShmError, '');
      const object = capture.before?.hostShm?.posix?.find(object => object.path === path);
      assert.ok(object);
      assert.equal(String(object.sizeBytes), String(size));
      assert.equal(object.mode! & 0o777, 0);
      if (reviewed) assert.deepEqual(inspected.systemdSnapshot, reviewed);
      reviewed = inspected.systemdSnapshot;
    }
  } finally {
    await owner.close();
    await payload.close();
    await rm(directory, { recursive: true, force: true });
    await rm(root, { recursive: true, force: true });
  }
});
