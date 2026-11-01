import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, mkdir, writeFile, rm, chmod } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/store.js';
import { validateConfig } from '../src/config.js';
import { wire, execution, workflow, type NativeExecution } from '../src/native.js';

class Engine implements NativeExecution {
  starts = 0;
  stops = 0;
  closed = false;
  update = execution.ExecutionUpdate.create();
  release?: () => void;
  rejectAdmission = false;
  profileNames: string[] = [];
  needsProfiles = false;
  async profiles() {
    return Buffer.from(execution.Profiles.encode({ names: this.profileNames }).finish());
  }
  async inspect(input: Buffer) {
    if (this.needsProfiles && !execution.RunRequest.decode(input).projects[0].profiles?.length)
      throw new Error('EmptySelection');
    return Buffer.from(
      execution.Inspection.encode({
        plan: { services: { worker: { imageId: 'sha256:approved' } } },
        snapshot: { manifestSha256: 'approved' },
      }).finish(),
    );
  }
  start(input: Buffer) {
    const request = execution.StartRequest.decode(input);
    this.starts++;
    if (this.rejectAdmission) return Promise.reject(new Error('Busy'));
    this.update = execution.ExecutionUpdate.create({ runId: request.run!.id });
    return new Promise<Buffer>((resolve) => {
      this.release = () => resolve(Buffer.alloc(0));
    });
  }
  async stop() {
    this.stops++;
    this.release?.();
  }
  poll() {
    const bytes = Buffer.from(execution.ExecutionUpdate.encode(this.update).finish());
    this.update.events = [];
    return bytes;
  }
  async close() {
    this.closed = true;
    this.release?.();
  }
}
async function fixture(recording = false) {
  const root = await mkdtemp(join(tmpdir(), 'console-service-')),
    project = join(root, 'project');
  await mkdir(project);
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  const config = wire.ConsoleConfig.create({
    projectRoot: root,
    dataRoot: join(root, 'data'),
    dockerSocket: '/unused.sock',
    accessSecretFile: join(root, 'secret'),
    listenAddress: '127.0.0.1:8080',
    publicOrigin: 'http://127.0.0.1:8080',
  });
  if (recording)
    config.systemApps = [
      {
        id: 'recorder',
        defaultSelected: true,
        input: { directory: project, files: ['compose.yaml'] },
        recordings: [{ id: 'messages', path: 'nested/messages.mcap' }],
      },
    ];
  validateConfig(config);
  const engine = new Engine(),
    store = await Store.open(config.dataRoot),
    backend = new ConsoleService(config, engine, store);
  const review = wire.Review.decode(
    await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ directory: project }).finish())),
  );
  return { root, config, engine, backend, review };
}
const waitFor = async (predicate: () => boolean) => {
  for (let i = 0; i < 200 && !predicate(); i++) await new Promise((resolve) => setTimeout(resolve, 5));
  assert.ok(predicate());
};

test('duplicate Start requests join one execution and preserve independent completion facts', async () => {
  // Arrange
  const { root, engine, backend, review } = await fixture();
  const request = Buffer.from(
    wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'same' }).finish(),
  );
  try {
    // Act
    const first = backend.start(request),
      retry = backend.start(request);
    await waitFor(() => engine.starts === 1);
    // Assert
    assert.equal(first, retry);
    assert.equal(engine.starts, 1);
    // Act
    engine.release!();
    await first;
    engine.update.ready = true;
    engine.update.events = [
      workflow.Event.fromObject({
        version: 1,
        session: 'run',
        executor: 'local',
        step: 'compose.prepare',
        state: 'SUCCEEDED',
        sequence: '9007199254740993',
        invocation: '9007199254740993',
        payloadJson: 'private',
      }),
    ];
    engine.update.droppedEvents = 4;
    const running = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(running.phase, wire.Phase.RUNNING);
    assert.equal(running.sequence.toString(), '9007199254740993');
    assert.equal(running.progressIncomplete, true);
    assert.equal(running.steps[0].payloadJson, null);
    // Act
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    engine.update.result = execution.ExecutionResult.create({
      blocked: false,
      boundaryClean: true,
      writersStopped: true,
      eventStreamHealthy: false,
    });
    const finished = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(finished.phase, wire.Phase.FINISHED);
    assert.equal(finished.outcome!.boundaryClean, true);
    assert.equal(finished.outcome!.eventStreamHealthy, false);
    assert.equal(engine.stops, 1);
    await assert.rejects(
      backend.start(
        Buffer.from(wire.StartRequest.encode({ preparedId: 'other', requestKey: 'same' }).finish()),
      ),
      /Conflict/,
    );
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('failed intent persistence prevents execution admission', async () => {
  // Arrange
  const { root, config, engine, backend, review } = await fixture();
  await chmod(config.dataRoot, 0o500);
  try {
    // Act / Assert
    await assert.rejects(
      backend.start(
        Buffer.from(
          wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'no-disk-access' }).finish(),
        ),
      ),
      /Internal/,
    );
    assert.equal(engine.starts, 0);
    assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.BLOCKED);
  } finally {
    await chmod(config.dataRoot, 0o700);
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('Stop latched before native admission cancels the pending Start', async () => {
  // Arrange
  const { root, engine, backend, review } = await fixture();
  const key = Buffer.from(
    wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'stop-early' }).finish(),
  );
  try {
    // Act
    const starting = backend.start(key);
    const rejected = assert.rejects(starting, /Stopped/);
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: review.runId }).finish()));
    await rejected;
    // Assert
    assert.equal(engine.starts, 0);
    const status = wire.Status.decode(await backend.status());
    assert.equal(status.phase, wire.Phase.FINISHED);
    assert.equal(status.outcome!.execution, 'Cancelled');
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('declared recording sizes and downloads are built in the TypeScript view', async () => {
  // Arrange
  const { root, config, engine, backend, review } = await fixture(true);
  try {
    // Act
    const starting = backend.start(
      Buffer.from(
        wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'recording' }).finish(),
      ),
    );
    await waitFor(() => engine.starts === 1);
    engine.release!();
    await starting;
    const directory = join(config.dataRoot, 'runs', review.runId, 'nested');
    await mkdir(directory, { recursive: true });
    await writeFile(join(directory, 'messages.mcap'), 'captured evidence');
    engine.update.ready = true;
    await new Promise((resolve) => setTimeout(resolve, config.runtime!.sampleIntervalMs! + 10));
    const running = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(running.recordings[0].id, 'messages');
    assert.equal(running.metrics.at(-1)!.recordingBytes!.toString(), '17');
    await assert.rejects(backend.openArtifact(review.runId, 'messages'), /Missing/);
    // Act
    engine.update.result = execution.ExecutionResult.create({
      blocked: false,
      boundaryClean: true,
      writersStopped: true,
      recordingFinalized: true,
      eventStreamHealthy: true,
    });
    const finished = wire.Status.decode(await backend.status());
    const fd = await backend.openArtifact(review.runId, 'messages');
    const { readFileSync, closeSync } = await import('node:fs');
    // Assert
    assert.equal(finished.outcome!.recordingFinalized, true);
    assert.equal(readFileSync(fd, 'utf8'), 'captured evidence');
    closeSync(fd);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('native admission failure leaves the Console able to review again', async () => {
  // Arrange
  const { root, config, engine, backend, review } = await fixture();
  engine.rejectAdmission = true;
  try {
    // Act / Assert
    await assert.rejects(
      backend.start(
        Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'busy' }).finish()),
      ),
      /Busy/,
    );
    assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.IDLE);
    const next = wire.Review.decode(
      await backend.prepare(
        Buffer.from(wire.PrepareRequest.encode({ directory: join(config.projectRoot, 'project') }).finish()),
      ),
    );
    assert.ok(next.preparedId);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a profiles-only project returns choices and can then be prepared explicitly', async () => {
  // Arrange
  const { root, config, engine, backend } = await fixture();
  engine.needsProfiles = true;
  engine.profileNames = ['optional'];
  const directory = join(config.projectRoot, 'project');
  try {
    // Act
    const choices = wire.Review.decode(
      await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ directory, profiles: [] }).finish())),
    );
    // Assert
    assert.equal(choices.preparedId, '');
    assert.deepEqual(choices.availableProfiles, ['optional']);
    // Act
    const ready = wire.Review.decode(
      await backend.prepare(
        Buffer.from(wire.PrepareRequest.encode({ directory, profiles: ['optional'] }).finish()),
      ),
    );
    // Assert
    assert.ok(ready.preparedId);
    assert.deepEqual(ready.selectedProfiles, ['optional']);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});
