import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ConsoleService } from '../src/service.js';
import { wire } from '../src/native.js';

/** Opt-in Engine test: build unprivileged, elevate only this test executable. */
test('TypeScript Console runs and stops two real Compose experiments', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-typescript-compose-')),
    project = join(root, 'project');
  await mkdir(project);
  await writeFile(
    join(project, 'compose.yaml'),
    `services:\n  worker:\n    image: doramcap/mcap-recorder:local\n    entrypoint: ["/bin/sh", "-c"]\n    command: ["trap 'exit 0' TERM INT; while :; do /bin/sleep 1 & wait $$!; done"]\n`,
  );
  const config = join(root, 'console.pbtxt');
  await writeFile(join(root, 'secret'), 'test-secret', { mode: 0o600 });
  await writeFile(
    config,
    `listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"${root}" data_root:"${root}/data" docker_socket:"/var/run/docker.sock" access_secret_file:"${root}/secret" runtime { readiness_seconds:30 stop_seconds:3 }`,
  );
  const { backend } = await ConsoleService.open(config);
  try {
    for (let attempt = 0; attempt < 2; attempt++) {
      // Act
      const review = wire.Review.decode(
        await backend.prepare(
          Buffer.from(
            wire.PrepareRequest.encode({
              directory: project,
              profiles: [],
              systemApps: { ids: [] },
            }).finish(),
          ),
        ),
      );
      const request = Buffer.from(
        wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: `run-${attempt}` }).finish(),
      );
      const start = backend.start(request),
        retry = backend.start(request);
      assert.equal(start, retry);
      await start;
      let status = wire.Status.decode(await backend.status());
      const deadline = Date.now() + 45000;
      while (status.phase === wire.Phase.STARTING && Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, 100));
        status = wire.Status.decode(await backend.status());
      }
      // Assert
      assert.equal(
        status.phase,
        wire.Phase.RUNNING,
        JSON.stringify(wire.Status.toObject(status, { longs: String, enums: String })),
      );
      assert.ok(status.steps.length > 0);
      assert.equal(status.runId, review.runId);
      // Act
      await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
      while (
        [wire.Phase.STARTING, wire.Phase.RUNNING, wire.Phase.STOPPING].includes(status.phase) &&
        Date.now() < deadline
      ) {
        await new Promise((resolve) => setTimeout(resolve, 100));
        status = wire.Status.decode(await backend.status());
      }
      // Assert
      assert.equal(status.phase, wire.Phase.FINISHED);
      assert.equal(status.outcome!.boundaryClean, true);
      assert.equal(status.reportAvailable, true);
      assert.equal(status.progressIncomplete, false);
      assert.ok(status.steps.some((step) => step.step === 'compose.prepare'));
      const { closeSync } = await import('node:fs');
      closeSync(await backend.openArtifact(review.runId, 'report'));
    }
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});
