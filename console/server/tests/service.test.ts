import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, mkdir, writeFile, rm, chmod } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/store.js';
import { validateConfig } from '../src/config.js';
import { readFile } from 'node:fs/promises';
import { live, wire, execution, workflow, type NativeExecution } from '../src/native.js';

class Engine implements NativeExecution {
  starts = 0;
  stops = 0;
  closed = false;
  update = execution.ExecutionUpdate.create();
  release?: () => void;
  rejectAdmission = '';
  profileNames: string[] = [];
  needsProfiles = false;
  inspected?: Parameters<NativeExecution['inspect']>[0];
  async profiles() {
    return execution.Profiles.create({ names: this.profileNames });
  }
  async inspect(request: Parameters<NativeExecution['inspect']>[0]) {
    if (this.needsProfiles && !request.projects[0].profiles?.length) throw new Error('EmptySelection');
    this.inspected = request;
    return execution.Inspection.create({
      plan: { services: { worker: { imageId: 'sha256:approved' } } },
      snapshot: { manifestSha256: 'approved' },
    });
  }
  start(run: Parameters<NativeExecution['start']>[0]) {
    this.starts++;
    if (this.rejectAdmission) return Promise.reject(new Error(this.rejectAdmission));
    this.update = execution.ExecutionUpdate.create({ runId: run.id });
    return new Promise<void>((resolve) => {
      this.release = resolve;
    });
  }
  async stop() {
    this.stops++;
    this.release?.();
  }
  poll() {
    const update = execution.ExecutionUpdate.create(this.update);
    this.update.events = [];
    return update;
  }
  async close() {
    this.closed = true;
    this.release?.();
  }
}
async function fixture(recording = false, liveOutput = false, systemApps?: string[]) {
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
  if (liveOutput) config.liveBridge = { image: 'doramcap/live-bridge:local', port: 43190 };
  if (recording || liveOutput)
    config.systemApps = [
      {
        id: 'recorder',
        defaultSelected: true,
        input: { directory: project, files: ['compose.yaml'] },
        recordings: recording ? [{ id: 'messages', path: 'nested/messages.mcap' }] : [],
        liveOutputs: liveOutput ? [{ source: 'monitor/snapshots', kind: live.Kind.TELEMETRY_PACKET }] : [],
      },
    ];
  validateConfig(config);
  const engine = new Engine(),
    store = await Store.open(config.dataRoot),
    backend = new ConsoleService(config, engine, store);
  const review = wire.Review.decode(
    await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ directory: project, systemApps: systemApps && { ids: systemApps } }).finish())),
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

for (const rejection of ['Busy', 'Closing'])
  test(`native ${rejection} leaves the Console able to review again`, async () => {
    // Arrange
    const { root, config, engine, backend, review } = await fixture();
    engine.rejectAdmission = rejection;
    try {
      // Act / Assert
      await assert.rejects(
        backend.start(
          Buffer.from(
            wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'busy' }).finish(),
          ),
        ),
        { message: rejection },
      );
      assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.IDLE);
      // Act / Assert: preparation also preserves owner availability failures.
      const profiles = engine.profiles;
      engine.profiles = async () => {
        throw new Error(rejection);
      };
      await assert.rejects(
        backend.prepare(
          Buffer.from(
            wire.PrepareRequest.encode({
              directory: join(config.projectRoot, 'project'),
            }).finish(),
          ),
        ),
        { message: rejection },
      );
      engine.profiles = profiles;
      const next = wire.Review.decode(
        await backend.prepare(
          Buffer.from(
            wire.PrepareRequest.encode({ directory: join(config.projectRoot, 'project') }).finish(),
          ),
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

test('preflight failures return a safe classification and retain private diagnostics', async () => {
  // Arrange
  const { root, config, engine, backend, review } = await fixture();
  const privateReason = 'repository preflight failed at /operator/private/location';
  engine.start = async (run) => {
    engine.update = execution.ExecutionUpdate.create({
      runId: run.id,
      result: { blocked: false, error: privateReason },
    });
    throw new Error(privateReason);
  };
  try {
    // Act / Assert
    await assert.rejects(
      backend.start(
        Buffer.from(
          wire.StartRequest.encode({
            preparedId: review.preparedId,
            requestKey: 'preflight-failure',
          }).finish(),
        ),
      ),
      { message: 'PreflightRejected' },
    );
    const status = wire.Status.decode(await backend.status());
    assert.equal(status.phase, wire.Phase.IDLE);
    assert.equal(status.outcome!.execution, 'Failed');
    const { readdir, readFile } = await import('node:fs/promises');
    const directory = join(config.dataRoot, 'diagnostics');
    const notes = await Promise.all(
      (await readdir(directory)).map((name) => readFile(join(directory, name), 'utf8')),
    );
    assert.ok(notes.some((note) => note.includes(privateReason)));
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('runs without live outputs report no live streams or metric samples', async () => {
  // Arrange
  const { root, engine, backend, review } = await fixture();
  try {
    // Act
    const start = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'no-live' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!();
    await start;
    engine.update.ready = true;
    const status = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(status.phase, wire.Phase.RUNNING);
    assert.equal(status.recordings.length, 0);
    assert.equal(status.liveStreams.length, 0);
    assert.equal(status.metrics.length, 0);
    assert.ok(!review.systemApps.some((app) => app.id === 'live-bridge'));
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('the selected live bridge subscribes to declared outputs through generated Dora and Compose inputs', async () => {
  // Arrange
  const { root, engine, backend, review } = await fixture(false, true);
  try {
    const run = engine.inspected!;
    const bridge = run.systemApps.find((project) => project.key === 'live-bridge')!;
    const compose = JSON.parse(await readFile(bridge.files![0], 'utf8'));
    const graph = JSON.parse(await readFile(run.dora!.graph!, 'utf8'));
    // Assert
    assert.deepEqual(review.liveStreams.map((stream) => [stream.id, stream.appId, stream.kind]), [['recorder_0', 'recorder', live.Kind.TELEMETRY_PACKET]]);
    const option = review.systemApps.find((app) => app.id === 'live-bridge')!;
    assert.deepEqual([option.label, option.selected, option.required], ['Live telemetry', true, false]);
    assert.equal(compose.services['live-bridge'].image, 'doramcap/live-bridge:local');
    assert.deepEqual(compose.services['live-bridge'].command, ['--port', '43190']);
    assert.equal(compose.services['live-bridge'].network_mode, 'host');
    const node = graph.nodes.find((node: { id: string }) => node.id === 'live-bridge');
    assert.equal(node.inputs.recorder_0.source ?? node.inputs.recorder_0, 'monitor/snapshots');
    // Act
    const start = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'live' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!();
    await start;
    const status = wire.Status.decode(await backend.status());
    // Assert
    assert.deepEqual(status.liveStreams, review.liveStreams);
    assert.deepEqual(status.systemApps!.ids, ['live-bridge', 'recorder']);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('deselecting the live bridge removes its service, graph node and streams', async () => {
  // Arrange / Act
  const { root, engine, backend, review } = await fixture(false, true, ['recorder']);
  try {
    // Assert
    assert.equal(review.liveStreams.length, 0);
    assert.equal(review.systemApps.find((app) => app.id === 'live-bridge')!.selected, false);
    assert.ok(!engine.inspected!.systemApps.some((project) => project.key === 'live-bridge'));
    assert.ok(!engine.inspected!.dora?.graph);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});
