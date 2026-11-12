import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/store.js';
import { validateConfig } from '../src/config.js';
import { readFile } from 'node:fs/promises';
import { compose, launch, live, wire, execution, workflow, type NativeExecution } from '../src/native.js';
import Long from 'long';
import { LIMITS, PREPARING_RUN } from '../src/generated/transport.js';

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
  started?: Parameters<NativeExecution['start']>[0];
  async composeProfiles() {
    return execution.ComposeProfiles.create({ names: this.profileNames });
  }
  async inspect(request: Parameters<NativeExecution['inspect']>[0]) {
    if (this.needsProfiles && !request.compose!.projects![0].profiles?.length) throw new Error('EmptySelection');
    this.inspected = request;
    if (request.systemd)
      return execution.Inspection.create({ systemdSnapshot: { sha256: 'approved-systemd' } });
    return execution.Inspection.create({
      plan: { services: { worker: { imageId: 'sha256:approved' } } },
      composeSnapshot: { manifestSha256: 'approved' },
    });
  }
  start(request: Parameters<NativeExecution['start']>[0]) {
    this.starts++;
    this.started = request;
    if (this.rejectAdmission) return Promise.reject(new Error(this.rejectAdmission));
    this.update = execution.ExecutionUpdate.create({ runId: request.run!.run!.id });
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
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
    projectRoot: project,
    dataRoot: join(root, 'data'),
    dockerSocket: '/unused.sock',
    accessSecretFile: join(root, 'secret'),
    listenAddress: '127.0.0.1:8080',
    publicOrigin: 'http://127.0.0.1:8080',
  });
  if (recording || liveOutput)
    config.apps = [
      ...(liveOutput
        ? [{
            id: 'live-bridge',
            label: 'Live telemetry',
            defaultSelected: true,
            input: { directory: project, files: ['compose.yaml'] },
            configMount: { service: 'live-bridge', target: '/config/bridge.pbtxt' },
            liveRelay: { port: 43190 },
          }]
        : []),
      {
        id: 'recorder',
        defaultSelected: true,
        input: { directory: project, files: ['compose.yaml'] },
        recordings: recording ? [{ id: 'messages', path: 'nested/messages.mcap' }] : [],
        liveOutputs: liveOutput ? [{ source: 'monitor/snapshots', kind: live.Kind.CPU_ACCOUNTING }] : [],
      },
    ];
  validateConfig(config);
  const engine = new Engine(),
    store = await Store.open(config.dataRoot),
    backend = new ConsoleService(config, engine, store);
  const review = wire.Review.decode(
    await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps: systemApps && { ids: systemApps } }).finish())),
  );
  return { root, config, store, engine, backend, review };
}
const waitFor = async (predicate: () => boolean) => {
  for (let i = 0; i < 200 && !predicate(); i++) await new Promise((resolve) => setTimeout(resolve, 5));
  assert.ok(predicate());
};

test('systemd Start carries its Review snapshot and rediscovers current unit files', async () => {
  // Arrange
  const { root, config, engine, backend } = await fixture();
  config.form = launch.Form.SYSTEMD;
  config.binaries = '/opt/doramcap/bin';
  const first = join(config.projectRoot, 'worker.service');
  const added = join(config.projectRoot, 'added.service');
  await writeFile(first, '[Service]\nExecStart=/bin/true\n');
  try {
    const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())));
    assert.deepEqual(engine.inspected!.systemd!.unitFiles, [first]);

    // Act
    await writeFile(added, '[Service]\nExecStart=/bin/true\n');
    const starting = backend.start(Buffer.from(wire.StartRequest.encode({
      preparedId: review.preparedId, requestKey: 'systemd-inputs',
    }).finish()));
    await waitFor(() => engine.starts === 1);

    // Assert: fresh native inputs and the original backend snapshot reach admission together.
    assert.equal(engine.started!.systemdExpected!.sha256, 'approved-systemd');
    assert.equal(engine.started!.composeExpected, null);
    assert.deepEqual(engine.started!.run!.systemd!.unitFiles, [added, first]);
    engine.release!();
    await starting;
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

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
    assert.equal(engine.started!.composeExpected!.manifestSha256, 'approved');
    assert.equal(engine.started!.systemdExpected, null);
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
  const { root, store, engine, backend, review } = await fixture();
  store.db.exec('DROP TABLE marker');
  try {
    // Act / Assert
    await assert.rejects(
      backend.start(
        Buffer.from(
          wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'no-marker' }).finish(),
        ),
      ),
      /no such table: marker/,
    );
    assert.equal(engine.starts, 0);
    assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.IDLE);
  } finally {
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
    engine.update.teardownStarted = true;
    engine.update.stoppingServices = [compose.StoppingService.create({ service: 'perception', forceStopUnixMs: Long.fromNumber(Date.now() + 12_000, true) })];
    const stopping = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(stopping.stoppingServices[0].service, 'perception');
    const remaining = stopping.stoppingServices[0].forceStopInSeconds!;
    assert.ok(remaining > 0 && remaining <= 12);
    // Act
    engine.update.stoppingServices = [compose.StoppingService.create({ service: 'perception', forceStopUnixMs: Long.fromNumber(Date.now() - 1_000, true) })];
    const overdue = wire.Status.decode(await backend.status());
    // Assert
    assert.equal(overdue.stoppingServices[0].forceStopInSeconds, 0);
    // Act
    engine.update.result = execution.ExecutionResult.create({
      blocked: false,
      boundaryClean: true,
      writersStopped: true,
      recordingFinalized: true,
      serviceExits: [{ service: 'perception', exitCode: 137, killed: true }],
      eventStreamHealthy: true,
    });
    const finished = wire.Status.decode(await backend.status());
    const fd = await backend.openArtifact(review.runId, 'messages');
    const { readFileSync, closeSync } = await import('node:fs');
    // Assert
    assert.equal(finished.stoppingServices.length, 0);
    assert.equal(finished.outcome!.recordingFinalized, true);
    assert.equal(finished.outcome!.serviceExits![0].service, 'perception');
    assert.equal(finished.outcome!.serviceExits![0].exitCode, 137);
    assert.equal(finished.outcome!.serviceExits![0].killed, true);
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
      const profiles = engine.composeProfiles;
      engine.composeProfiles = async () => {
        throw new Error(rejection);
      };
      await assert.rejects(
        backend.prepare(
          Buffer.from(
            wire.PrepareRequest.encode({}).finish(),
          ),
        ),
        { message: rejection },
      );
      engine.composeProfiles = profiles;
      const next = wire.Review.decode(
        await backend.prepare(
          Buffer.from(
            wire.PrepareRequest.encode({}).finish(),
          ),
        ),
      );
      assert.ok(next.preparedId);
    } finally {
      await backend.close();
      await rm(root, { recursive: true, force: true });
    }
  });

test('a failed Review keeps its cause beside the browser error code', async () => {
  // Arrange
  const { root, engine, backend } = await fixture();
  engine.composeProfiles = async () => {
    throw new Error('compose.yaml: services must be a mapping');
  };
  try {
    // Act / Assert
    await assert.rejects(backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())), {
      message: 'InvalidInput',
      detail: 'compose.yaml: services must be a mapping',
    });
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a long Review failure keeps the end of its cause', async () => {
  // Arrange
  const { root, engine, backend } = await fixture();
  engine.composeProfiles = async () => {
    throw new Error(`${'x'.repeat(3000)} the cause`);
  };
  try {
    // Act / Assert
    await assert.rejects(backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())), (error: Error & { detail: string }) => {
      assert.equal(error.detail.length, execution.Limit.MAX_ERROR_CHARS);
      assert.ok(error.detail.endsWith('the cause'));
      return true;
    });
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
  try {
    // Act
    const choices = wire.Review.decode(
      await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ profiles: [] }).finish())),
    );
    // Assert
    assert.equal(choices.preparedId, '');
    assert.deepEqual(choices.availableProfiles, ['optional']);
    // Act
    const ready = wire.Review.decode(
      await backend.prepare(
        Buffer.from(wire.PrepareRequest.encode({ profiles: ['optional'] }).finish()),
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
  engine.start = async (request) => {
    engine.update = execution.ExecutionUpdate.create({
      runId: request.run!.run!.id,
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

test('the selected live bridge subscribes to declared outputs through a generated graph node and config', async () => {
  // Arrange
  const { root, engine, backend, review } = await fixture(false, true);
  try {
    const request = engine.inspected!;
    const bridge = request.compose!.projects!.find((project) => project.key === 'live-bridge')!;
    const compose = JSON.parse(await readFile(bridge.files!.at(-1)!, 'utf8'));
    const graph = JSON.parse(await readFile(request.run!.dataflowFile!, 'utf8'));
    // Assert
    assert.deepEqual(review.liveStreams.map((stream) => [stream.id, stream.appId, stream.kind]), [['recorder_0', 'recorder', live.Kind.CPU_ACCOUNTING]]);
    const option = review.systemApps.find((app) => app.id === 'live-bridge')!;
    assert.deepEqual([option.label, option.selected, option.required], ['Live telemetry', true, false]);
    const mount = compose.services['live-bridge'].volumes.find((volume:any) => volume.target === '/config/bridge.pbtxt');
    assert.equal(mount.read_only, true);
    assert.equal(await readFile(mount.source, 'utf8'), `port: 43190\nmax_payload_bytes: ${LIMITS.bridgePayloadBytes}\n`);
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

test('the selected launch profile mounts its config read-only and declares only its outputs', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-profile-')),
    project = join(root, 'project'),
    profile = (id: string) => join(root, `${id}.pbtxt`);
  await mkdir(project);
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  const config = wire.ConsoleConfig.create({
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
    projectRoot: project,
    dataRoot: join(root, 'data'),
    dockerSocket: '/unused.sock',
    accessSecretFile: join(root, 'secret'),
    listenAddress: '127.0.0.1:8080',
    publicOrigin: 'http://127.0.0.1:8080',
    apps: [
      {
        id: 'monitor',
        defaultSelected: true,
        input: { directory: project, files: ['compose.yaml'] },
        configMount: { service: 'monitor', target: '/config/monitor.pbtxt' },
        launchProfiles: [
          { id: 'record', label: 'Record', config: profile('record'), recordings: [{ id: 'cpu', path: 'cpu.mcap' }] },
          { id: 'live', label: 'Live', config: profile('live') },
        ],
      },
    ],
  });
  validateConfig(config);
  const engine = new Engine(),
    store = await Store.open(config.dataRoot),
    backend = new ConsoleService(config, engine, store),
    prepare = async (systemApps?: { ids: string[]; profiles: Record<string, string> }) =>
      wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps }).finish())));
  try {
    // Act: review with the default profile.
    const first = await prepare();
    const mounted = async () => {
      const files = engine.inspected!.compose!.projects!.find((app) => app.key === 'monitor')!.files!;
      return JSON.parse(await readFile(files[files.length - 1], 'utf8')).services.monitor.volumes;
    };
    // Assert
    assert.deepEqual(first.recordings.map((output) => output.id), ['cpu']);
    const option = first.systemApps.find((app) => app.id === 'monitor')!;
    assert.deepEqual(option.profiles!.map((item) => [item.id, item.label]), [['record', 'Record'], ['live', 'Live']]);
    assert.equal(option.profile, 'record');
    assert.deepEqual(await mounted(), [
      { type: 'bind', source: profile('record'), target: '/config/monitor.pbtxt', read_only: true, bind: { create_host_path: false } },
    ]);

    // Act: choose another profile, then start it.
    const second = await prepare({ ids: ['monitor'], profiles: { monitor: 'live' } });
    const volumes = await mounted();
    const start = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: second.preparedId, requestKey: 'profile' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!();
    await start;
    const status = wire.Status.decode(await backend.status());
    // Assert
    assert.deepEqual(second.recordings, []);
    assert.equal(volumes[0].source, profile('live'));
    assert.deepEqual(status.systemApps!.profiles, { monitor: 'live' });
    assert.deepEqual(status.recordings, []);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});
test('an unknown launch profile rejects preparation', async () => {
  // Arrange
  const { root, backend, review } = await fixture(true);
  try {
    // Act / Assert
    assert.ok(review.preparedId);
    await assert.rejects(backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps: { ids: ['recorder'], profiles: { recorder: 'missing' } } }).finish())));
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
    assert.ok(!engine.inspected!.compose!.projects!.some((project) => project.key === 'live-bridge'));
    assert.ok(!engine.inspected!.run!.dataflowFile);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a hung image download can be cancelled, and failed downloads keep their own error', async () => {
  // Arrange
  const { root, config, backend } = await fixture(), previous = process.env.PATH;
  const commands = join(root, 'commands');
  await mkdir(commands);
  await writeFile(join(commands, 'docker'), `#!${process.execPath} --
const args = process.argv.slice(2), image = require('node:fs').readFileSync(${JSON.stringify(join(commands, 'image'))}, 'utf8');
if (args.includes('compose')) console.log(JSON.stringify({ services: { app: {image: image} } }));
else if (args.includes('inspect')) process.exit(1);
else if (args.at(-1).includes('broken')) process.exit(1);
else setTimeout(() => {}, 60000);
`, { mode: 0o755 });
  process.env.PATH = `${commands}:${previous}`;
  config.downloadMissingImages = true;
  const request = Buffer.from(wire.PrepareRequest.encode({}).finish());
  const stop = Buffer.from(wire.StopRequest.encode({ runId: PREPARING_RUN }).finish());
  try {
    // Act / Assert: cancellation returns the Console to idle.
    config.doraApps = ['communication'];
    config.apps = [{id:'communication',input:{directory:root,files:['hung']}}];
    await writeFile(join(commands, 'image'), 'registry/hung@sha256:hung');
    const hung = backend.prepare(request);
    await new Promise((resolve) => setTimeout(resolve, 200));
    await assert.rejects(backend.prepare(request), /Conflict/);
    await backend.stop(stop);
    await assert.rejects(hung, /^Error: Stopped$/);
    assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.IDLE);
    // Act / Assert: a registry failure is distinct from invalid project input.
    config.apps[0].input!.files = ['broken'];
    await writeFile(join(commands, 'image'), 'registry/broken@sha256:broken');
    await assert.rejects(backend.prepare(request), /^Error: ImageDownload$/);
    // Act / Assert: closing never waits for a hung download.
    config.doraApps = ['communication'];
    config.apps = [{id:'communication',input:{directory:root,files:['hung']}}];
    await writeFile(join(commands, 'image'), 'registry/hung@sha256:hung');
    const closing = backend.prepare(request);
    await new Promise((resolve) => setTimeout(resolve, 200));
    await Promise.all([backend.close(), assert.rejects(closing)]);
  } finally {
    process.env.PATH = previous;
    await rm(root, { recursive: true, force: true });
  }
});

test('cancelling a review without downloads discards its inspection', async () => {
  // Arrange
  const { root, engine, backend } = await fixture();
  const inspect = engine.inspect.bind(engine);
  let finish = () => {};
  engine.inspect = async (request) => {
    await new Promise<void>((resolve) => { finish = resolve; });
    return inspect(request);
  };
  try {
    // Act
    const review = backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish()));
    await new Promise((resolve) => setTimeout(resolve, 50));
    await backend.stop(Buffer.from(wire.StopRequest.encode({ runId: PREPARING_RUN }).finish()));
    finish();
    // Assert
    await assert.rejects(review, /^Error: Stopped$/);
    assert.equal(wire.Status.decode(await backend.status()).phase, wire.Phase.IDLE);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});


test('Compose always includes configured run shared memory independently of optional apps', async () => {
  // Arrange
  const { root, config, engine, backend } = await fixture();
  config.hostShmAccess = launch.HostShmAccess.HOST_SHM_ACCESS_BLOCKED;
  config.apps.push(wire.SystemAppDefinition.create({
    id: 'doramcap-shm',
    input: { directory: config.projectRoot, files: ['compose.yaml'] },
  }));
  try {
    // Act
    const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({ systemApps: { ids: [] } }).finish())));
    // Assert
    assert.deepEqual(engine.inspected!.run!.systemApps, ['doramcap-shm']);
    assert.equal(engine.inspected!.run!.hostShmAccess, launch.HostShmAccess.HOST_SHM_ACCESS_BLOCKED);
    assert.ok(engine.inspected!.compose!.projects!.some((project) => project.key === 'doramcap-shm'));
    assert.ok(!review.systemApps.some((app) => app.id === 'doramcap-shm'));

    // Arrange: switch the same configuration to native units.
    config.form = launch.Form.SYSTEMD;
    config.binaries = '/opt/doramcap/bin';
    await writeFile(join(config.projectRoot, 'worker.service'), '[Service]\nExecStart=/bin/true\n');
    // Act
    await backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish()));
    // Assert
    assert.deepEqual(engine.inspected!.run!.systemApps, []);
  } finally {
    await backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('preparation waits for the current file report and result transaction to finish saving', async () => {
  // Arrange
  const { fileChangeFixture } = await import('./file-change-fixture.js');
  const { capabilities } = await import('../src/native.js');
  const { root, config, engine, backend, review } = await fixture();
  const original = capabilities.openFile;
  try {
    const starting = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'report-save' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!(); await starting;
    const { reference } = await fileChangeFixture(join(config.dataRoot, 'runs', review.runId), 3000);
    let opened!: () => void;
    const observed = new Promise<void>(resolve => { opened = resolve; });
    capabilities.openFile = (runRoot, path) => {
      const fd = original(runRoot, path);
      if (path === reference.checkpoint) opened();
      return fd;
    };
    engine.update.result = execution.ExecutionResult.create({ fileChanges: reference, boundaryClean: true });
    // Act: attempt a new review while the previous result is reading its manifest.
    const finishing = backend.status();
    await observed;
    let reviewReturned = false;
    const preparing = backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())).finally(() => { reviewReturned = true; });
    await Promise.resolve();
    assert.equal(reviewReturned, false);
    const status = wire.Status.decode(await finishing);
    // Assert
    assert.equal(status.runId, review.runId);
    assert.equal(status.fileChanges?.previewIncomplete, true);
    assert.ok(status.fileChanges?.downloadable);
    await preparing;
  } finally { capabilities.openFile = original; await backend.close(); await rm(root, { recursive: true, force: true }); }
});

test('result reconciliation retains downloads through save failure and the next Prepare', async () => {
  // Arrange
  const { fileChangeFixture } = await import('./file-change-fixture.js');
  const { root, config, store, engine, backend, review } = await fixture();
  try {
    const starting = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'history-failure' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!(); await starting;
    const { reference } = await fileChangeFixture(join(config.dataRoot, 'runs', review.runId), 3);
    store.db.exec("CREATE TRIGGER refuse_result BEFORE UPDATE ON runs WHEN NEW.state != 1 BEGIN SELECT RAISE(FAIL, 'result storage failed'); END");
    engine.update.result = execution.ExecutionResult.create({ fileChanges: reference, boundaryClean: true });
    // Act: storage fails through the normal error path; native outcome facts remain available.
    await assert.rejects(backend.status(), /result storage failed/);
    const status = wire.Status.decode(await backend.status());
    const rows = [];
    for await (const bytes of backend.fileChanges(review.runId, new AbortController().signal)) rows.push(JSON.parse(bytes.toString()));
    // Assert: a current run uses its own terminal facts, as existing artifact downloads do.
    assert.equal(status.phase, wire.Phase.FINISHED);
    assert.equal(status.outcome?.boundaryClean, true);
    assert.ok(status.fileChanges?.downloadable);
    assert.equal(rows.filter(row => row.type === 'change').length, 3);
    assert.equal(rows.at(-1).type, 'complete');
    const completedBy = Date.now();
    const prepare = Buffer.from(wire.PrepareRequest.encode({}).finish());
    // Act / Assert: failed reconciliation refuses Prepare without replacing the result.
    await assert.rejects(backend.prepare(prepare), /Internal/);
    const retained = wire.Status.decode(await backend.status());
    assert.equal(retained.runId, review.runId);
    assert.deepEqual(retained.fileChanges, status.fileChanges);
    // Act: storage recovers; the same reconciliation saves before Prepare proceeds.
    store.db.exec('DROP TRIGGER refuse_result');
    await backend.prepare(prepare);
    const saved = wire.HistoryDetail.decode(await backend.historyRun(review.runId));
    const downloaded = [];
    for await (const bytes of backend.fileChanges(review.runId, new AbortController().signal)) downloaded.push(JSON.parse(bytes.toString()));
    // Assert
    assert.equal(saved.entry!.state, wire.HistoryState.HISTORY_COMPLETED);
    assert.ok(Number(saved.entry!.finishedAtMs) <= completedBy);
    assert.deepEqual(saved.status!.fileChanges, status.fileChanges);
    assert.deepEqual(downloaded, rows);
    // Act / Assert: another Prepare leaves the reconciled result unchanged.
    await backend.prepare(prepare);
    assert.deepEqual(wire.HistoryDetail.decode(await backend.historyRun(review.runId)), saved);
  } finally { await backend.close(); await rm(root, { recursive: true, force: true }); }
});

test('a preview exception fails its request without preventing a later Review', async () => {
  // Arrange
  const { fileChangeFixture } = await import('./file-change-fixture.js');
  const { capabilities } = await import('../src/native.js');
  const { root, config, engine, backend, review } = await fixture();
  const original = capabilities.openFile;
  try {
    const starting = backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'preview-fault' }).finish()));
    await waitFor(() => engine.starts === 1);
    engine.release!(); await starting;
    const { reference } = await fileChangeFixture(join(config.dataRoot, 'runs', review.runId), 3);
    capabilities.openFile = (runRoot, path) => {
      if (path === reference.checkpoint) throw null;
      return original(runRoot, path);
    };
    engine.update.result = execution.ExecutionResult.create({ fileChanges: reference, boundaryClean: true });
    // Act / Assert: an unexpected preview fault propagates; the native facts remain intact.
    await assert.rejects(backend.status());
    capabilities.openFile = original;
    const status = wire.Status.decode(await backend.status());
    assert.equal(status.phase, wire.Phase.FINISHED);
    assert.equal(status.outcome?.boundaryClean, true);
    assert.ok(status.fileChanges?.previewIncomplete);
    // Act: Review retries saving those facts instead of repeating finalization forever.
    await backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish()));
    const saved = wire.HistoryDetail.decode(await backend.historyRun(review.runId));
    const rows = [];
    for await (const bytes of backend.fileChanges(review.runId, new AbortController().signal)) rows.push(JSON.parse(bytes.toString()));
    // Assert
    assert.equal(saved.entry!.state, wire.HistoryState.HISTORY_COMPLETED);
    assert.equal(rows.filter(row => row.type === 'change').length, 3);
    assert.equal(rows.at(-1).type, 'complete');
  } finally { capabilities.openFile = original; await backend.close(); await rm(root, { recursive: true, force: true }); }
});
