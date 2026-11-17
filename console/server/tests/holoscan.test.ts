import { strict as assert } from 'node:assert';
import { mkdtemp, open, readFile, rm, writeFile, type FileHandle } from 'node:fs/promises';
import { once } from 'node:events';
import { createServer, type Socket } from 'node:net';
import { createInterface } from 'node:readline';
import { setTimeout as delay } from 'node:timers/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { validateConfig, loadConfig } from '../src/deployment/config.js';
import { releaseConfig } from '../src/deployment/release.js';
import { capabilities, holoscan, launch, wire } from '../src/native.js';
import { buildRunRequest } from '../src/run/run-request.js';
import { ConsoleService } from '../src/service.js';
import { Store } from '../src/records/store.js';

const configuration = (extra = '') => capabilities.parseConfig(
  `host_shm_access: HOST_SHM_ACCESS_PASSTHROUGH listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"/projects" data_root:"/data" docker_socket:"/missing/docker.sock" ${extra}`,
);

test('Holoscan configuration requires the approved absolute CLI and preserves existing form values', () => {
  // Arrange / Act
  const config = configuration('form: HOLOSCAN holoscan { cli:"/opt/holoscan" }');
  validateConfig(config);
  // Assert
  assert.equal(config.form, launch.Form.HOLOSCAN);
  assert.equal(config.holoscan!.cli, '/opt/holoscan');
  assert.deepEqual([launch.Form.COMPOSE, launch.Form.SYSTEMD, launch.Form.SUPERVISOR, launch.Form.HOLOSCAN], [1, 2, 3, 4]);
  for (const extra of ['form: HOLOSCAN', 'form: HOLOSCAN holoscan {}', 'form: HOLOSCAN holoscan { cli:"relative" }'])
    assert.throws(() => validateConfig(configuration(extra)), /holoscan.cli must be absolute/);
  for (const form of ['', 'form: COMPOSE', 'form: SYSTEMD', 'form: SUPERVISOR'])
    assert.throws(() => validateConfig(configuration(`${form} holoscan { cli:"/opt/holoscan" }`)), /holoscan applies only to HOLOSCAN/);
  assert.throws(() => validateConfig(wire.ConsoleConfig.create({ ...config, dockerSocket: '' })), /docker_socket must be absolute/);
  const release = releaseConfig(config, '/selected/project', '/release/apps', '/state');
  assert.equal(release.holoscan!.cli, '/opt/holoscan');
});

test('native catalog uses generated messages and never connects to the configured Docker socket', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-holoscan-'));
  const cli = fileURLToPath(new URL('../../../libs/workload-holoscan/tests/fixtures/cli.sh', import.meta.url));
  await writeFile(join(root, 'response.sh'), 'echo "metadata warning" >&2\necho \'{"schema_version":1,"projects":[]}\'\n');
  const owner = await capabilities.open(join(root, 'missing-docker.sock'), 1);
  try {
    // Act
    const result = await owner.catalog(holoscan.DiscoverRequest.create({ cli, projectRoot: root }));
    // Assert
    assert.ok(result instanceof holoscan.Catalog);
    assert.equal(result.outcome, holoscan.CatalogOutcome.CATALOG_OUTCOME_EMPTY);
    assert.equal(Buffer.from(result.diagnostics[0].stderr!).toString(), 'metadata warning\n');
    assert.equal(result.diagnostics[0].exitCode, 0);
  } finally { await owner.close(); await rm(root, { recursive: true, force: true }); }
});

test('Holoscan service loads and rejects unsupported lifecycle before Compose discovery or side effects', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-holoscan-service-'));
  const path = join(root, 'console.pbtxt');
  await writeFile(path, capabilities.formatConfig(wire.ConsoleConfig.create({
    ...configuration('form:HOLOSCAN holoscan {cli:"/missing/holoscan"}'), projectRoot: root, dataRoot: join(root, 'state'),
  })));
  const { config } = await loadConfig(path);
  const store = await Store.open(config.dataRoot);
  const owner = await capabilities.open(config.dockerSocket, 1);
  const service = new ConsoleService(config, owner, store);
  try {
    // Act / Assert: neither a Compose file nor a reachable daemon exists.
    await assert.rejects(service.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())), /Holoscan Prepare and Start are not available yet/);
    await assert.rejects(service.start(Buffer.from(wire.StartRequest.encode({}).finish())), /Holoscan Prepare and Start are not available yet/);
    await assert.rejects(buildRunRequest(config, {
      id:'r', project: { directory:root }, artifacts:join(root, 'run'), workspace:root, stage:'test', apps:[], graph:'', writePolicy:config.writePolicy,
    }), /Holoscan Prepare and Start are not available yet/);
    const status = wire.Status.decode(await service.status());
    assert.equal(status.phase, wire.Phase.IDLE);
    assert.equal(owner.poll().runId, '');
  } finally { await service.close(); await rm(root, { recursive: true, force: true }); }
});

const CATALOG_CLOSE_DEADLINE_MS = 5_000;
const CHILD_REAP_OBSERVATION_MS = 10;

for (const retainedWriters of [false, true]) test(
  retainedWriters ? 'closing catalog discards retained pipe readers after the direct child has exited' : 'closing the native owner settles a pending catalog and reaps its direct CLI child',
  { timeout: CATALOG_CLOSE_DEADLINE_MS * 3 }, async () => {
    // Arrange: subscribe before the CLI starts, then synchronize on its own PID handshake.
    const root = await mkdtemp(join(tmpdir(), 'console-holoscan-close-'));
    const cli = fileURLToPath(new URL('./fixtures/holoscan-blocking-cli.mjs', import.meta.url));
    const owner = await capabilities.open(join(root, 'missing-docker.sock'), 1);
    const server = createServer();
    const signal = AbortSignal.timeout(CATALOG_CLOSE_DEADLINE_MS);
    const listening = once(server, 'listening', { signal });
    server.listen(join(root, 'catalog-close.sock'));
    let connection: Socket | undefined;
    let childPid: number | undefined;
    const pipes: FileHandle[] = [];
    try {
      await listening;
      const connected = once(server, 'connection', { signal });
      let settled = false;
      const pending = owner.catalog(holoscan.DiscoverRequest.create({ cli, projectRoot: root }))
        .then(() => null, error => error as Error).finally(() => { settled = true; });
      connection = (await connected)[0] as Socket;
      const lines = createInterface({ input: connection });
      childPid = Number((await once(lines, 'line', { signal }))[0]);
      assert.ok(Number.isSafeInteger(childPid) && childPid > 0);
      const terminated = once(connection, 'close');
      if (retainedWriters) {
        // Keep the same pipe write ends a descendant could inherit, then let the direct child exit.
        pipes.push(await open(`/proc/${childPid}/fd/1`, 'w'));
        pipes.push(await open(`/proc/${childPid}/fd/2`, 'w'));
        connection.write('exit\n');
        await terminated;
        assert.equal(settled, false);
      }
      // Act: a deadline also releases the retained test writers if close wrongly waits for EOF.
      const closeDeadline = AbortSignal.timeout(CATALOG_CLOSE_DEADLINE_MS);
      await Promise.race([
        owner.close(),
        once(closeDeadline, 'abort').then(() => { throw new Error('native catalog close exceeded its deadline'); }),
      ]);
      // Assert: cancellation rejects as Closing, terminates the process, and leaves no zombie.
      const rejection = await pending;
      assert.ok(rejection instanceof Error);
      assert.match(rejection.message, /Closing/);
      await terminated;
      const reapDeadline = Date.now() + CATALOG_CLOSE_DEADLINE_MS;
      let status = '';
      while (true) {
        try { status = await readFile(`/proc/${childPid}/status`, 'utf8'); }
        catch (error) { if ((error as NodeJS.ErrnoException).code === 'ENOENT') break; throw error; }
        assert.ok(Date.now() < reapDeadline, `CLI child ${childPid} remained after close: ${status.split('\n').find(line => line.startsWith('State:'))}`);
        await delay(CHILD_REAP_OBSERVATION_MS);
      }
      childPid = undefined;
    } finally {
      connection?.destroy();
      await Promise.all(pipes.map(pipe => pipe.close()));
      if (childPid !== undefined) {
        try { process.kill(childPid, 'SIGKILL'); }
        catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error; }
      }
      await owner.close();
      await new Promise<void>(resolve => server.close(() => resolve()));
      await rm(root, { recursive: true, force: true });
    }
  },
);
