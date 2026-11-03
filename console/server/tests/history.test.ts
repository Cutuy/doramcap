import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { closeSync, readFileSync, statSync } from 'node:fs';
import { mkdir, mkdtemp, readdir, rm, symlink, utimes, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { test } from 'node:test';
import { validateConfig } from '../src/config.js';
import { History } from '../src/history.js';
import { createConsoleServer } from '../src/http.js';
import { execution, wire, type NativeExecution } from '../src/native.js';
import { ConsoleService } from '../src/service.js';
import { MIGRATIONS, Store, migrate } from '../src/store.js';
import { LIMITS, MEDIA_TYPE, historyRunPath, routes } from '../src/generated/transport.js';
import { sessionCookie } from './session.js';

const { HistoryState: State, Phase } = wire;
const id = (n: number) => n.toString(16).padStart(32, '0');
const temporary = () => mkdtemp(join(tmpdir(), 'console-history-'));

class Engine implements NativeExecution {
  update = execution.ExecutionUpdate.create();
  starts = 0;
  async profiles() {
    return execution.Profiles.create({ names: ['gpu'] });
  }
  async inspect() {
    return execution.Inspection.create({ plan: { services: { worker: { imageId: 'sha256:a' } } }, snapshot: { manifestSha256: 'a' } });
  }
  async start(run: Parameters<NativeExecution['start']>[0]) {
    this.starts++;
    this.update = execution.ExecutionUpdate.create({ runId: run.id });
  }
  async stop() {}
  poll() {
    const update = execution.ExecutionUpdate.create(this.update);
    this.update.events = [];
    return update;
  }
  async close() {}
}

/** A real service, store and database over a fake Compose engine. */
async function service(root: string) {
  const project = join(root, 'project');
  await mkdir(project, { recursive: true });
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  const config = wire.ConsoleConfig.create({
    projectRoot: project,
    dataRoot: join(root, 'data'),
    dockerSocket: '/unused.sock',
    accessSecretFile: join(root, 'secret'),
    listenAddress: '127.0.0.1:8080',
    publicOrigin: 'http://127.0.0.1:8080',
    systemApps: [{ id: 'recorder', defaultSelected: true, input: { directory: project, files: ['compose.yaml'] }, recordings: [{ id: 'messages', path: 'nested/messages.mcap' }] }],
  });
  validateConfig(config);
  const store = await Store.open(config.dataRoot),
    engine = new Engine(),
    backend = new ConsoleService(config, engine, store);
  new History(store).closeInterrupted();
  return { project, config, store, engine, backend };
}
type Service = Awaited<ReturnType<typeof service>>;
const list = async ({ backend }: Pick<Service, 'backend'>, query: object = {}) =>
  wire.HistoryPage.decode(await backend.history(Buffer.from(wire.HistoryQuery.encode(query).finish())));
const detail = async ({ backend }: Pick<Service, 'backend'>, runId: string) => wire.HistoryDetail.decode(await backend.historyRun(runId));
async function startRun({ backend, project }: Service, key = 'key') {
  const review = wire.Review.decode(await backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())));
  await backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: key }).finish()));
  return review.runId;
}
async function finishRun(run: Service, runId: string, result: Partial<typeof execution.ExecutionResult.prototype> = {}) {
  await mkdir(join(run.config.dataRoot, 'runs', runId, 'nested'), { recursive: true });
  await writeFile(join(run.config.dataRoot, 'runs', runId, 'nested/messages.mcap'), 'captured evidence');
  run.engine.update.result = execution.ExecutionResult.create({ blocked: false, boundaryClean: true, writersStopped: true, recordingFinalized: true, eventStreamHealthy: true, ...result });
  return wire.Status.decode(await run.backend.status());
}
/** Save one finished past run. */
async function checkpoint(store: Store, runId: string, fields: object = {}) {
  await mkdir(join(store.root, 'runs', runId, 'nested'), { recursive: true });
  new History(store).record(wire.Status.create({
    phase: Phase.FINISHED,
    runId,
    profiles: ['gpu'],
    systemApps: { ids: ['recorder'] },
    recordings: [{ id: 'messages', path: 'nested/messages.mcap' }],
    outcome: { execution: 'Completed', recordingFinalized: true, boundaryClean: true },
    ...fields,
  }), 1, 2);
}

test('each migration step raises user_version and a failed step leaves the last complete version', async () => {
  // Arrange
  const db = new DatabaseSync(':memory:');
  const version = () => (db.prepare('PRAGMA user_version').get() as { user_version: number }).user_version;
  // Act
  migrate(db);
  // Assert
  assert.equal(version(), MIGRATIONS.length);
  assert.deepEqual((db.prepare("SELECT name FROM pragma_table_info('runs')").all() as { name: string }[]).map(row => row.name),
    ['run_id', 'state', 'started_at_ms', 'entry', 'detail']);
  // Act
  assert.throws(() => migrate(db, [...MIGRATIONS, 'CREATE TABLE half (x); SELECT missing FROM nowhere']));
  // Assert
  assert.equal(version(), MIGRATIONS.length);
  assert.equal(db.prepare("SELECT name FROM sqlite_master WHERE name = 'half'").get(), undefined);
  // Act / Assert: a schema from a newer Console stays readable.
  db.exec(`PRAGMA user_version = ${MIGRATIONS.length + 1}`);
  migrate(db);
  assert.deepEqual(db.prepare('SELECT run_id FROM runs').all(), []);
  db.close();
});

test('a database from before per-project Consoles keeps its runs and accepts new ones', async () => {
  // Arrange
  const root = await temporary();
  const old = new DatabaseSync(join(root, 'console.db'));
  migrate(old, MIGRATIONS.slice(0, 1));
  old
    .prepare('INSERT INTO runs (run_id, project_directory, state, started_at_ms, entry, detail) VALUES (?, ?, ?, ?, ?, ?)')
    .run(id(1), '/old', State.HISTORY_COMPLETED, 1, new Uint8Array(), new Uint8Array());
  old.close();
  const store = await Store.open(root);
  try {
    // Act
    new History(store).record(wire.Status.create({ runId: id(2), phase: Phase.FINISHED }), 2, 3);
    // Assert
    const rows = store.db.prepare('SELECT run_id FROM runs ORDER BY run_id').all() as { run_id: string }[];
    assert.deepEqual(rows.map((row) => row.run_id), [id(1), id(2)]);
  } finally {
    store.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('the database is private, uses WAL and refuses a symlink', async () => {
  // Arrange
  const root = await temporary(), store = await Store.open(root);
  try {
    // Act
    new History(store).record(wire.Status.create({ phase: Phase.STARTING, runId: id(1) }), 1);
    // Assert
    for (const name of (await readdir(root)).filter(name => name.startsWith('console.db')))
      assert.equal(statSync(join(root, name)).mode & 0o777, 0o600, name);
    assert.deepEqual({ ...store.db.prepare('PRAGMA journal_mode').get() }, { journal_mode: 'wal' });
    assert.deepEqual({ ...store.db.prepare('PRAGMA synchronous').get() }, { synchronous: 2 });
    store.close();
    // Arrange
    await rm(join(root, 'console.db'));
    await writeFile(join(root, 'elsewhere'), 'keep');
    await symlink(join(root, 'elsewhere'), join(root, 'console.db'));
    // Act / Assert
    await assert.rejects(Store.open(root));
    assert.equal(readFileSync(join(root, 'elsewhere'), 'utf8'), 'keep');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Console refuses a run it cannot record', async () => {
  // Arrange
  const root = await temporary(), run = await service(root);
  const review = wire.Review.decode(await run.backend.prepare(Buffer.from(wire.PrepareRequest.encode({}).finish())));
  run.store.db.exec('DROP TABLE runs');
  try {
    // Act / Assert
    await assert.rejects(run.backend.start(Buffer.from(wire.StartRequest.encode({ preparedId: review.preparedId, requestKey: 'key' }).finish())));
    assert.equal(run.engine.starts, 0);
  } finally {
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a finished run survives a restart with its selection, times, error and artifact sizes', async () => {
  // Arrange
  const root = await temporary(), first = await service(root);
  const before = Date.now();
  // Act
  const runId = await startRun(first);
  // Assert: the run is indexed as soon as it starts.
  assert.equal((await list(first)).entries[0].state, State.HISTORY_RUNNING);
  // Act
  await finishRun(first, runId, { error: 'worker exited with status 3', boundaryClean: true });
  await first.backend.close();
  const second = await service(root);
  try {
    const page = await list(second), found = await detail(second, runId), entry = page.entries[0];
    // Assert
    assert.equal(page.entries.length, 1);
    assert.equal(entry.runId, runId);
    assert.equal(entry.state, State.HISTORY_FAILED);
    assert.equal(entry.errorSummary, 'worker exited with status 3');
    assert.equal(found.status!.outcome!.error, 'worker exited with status 3');
    assert.deepEqual(entry.systemApps!.ids, ['recorder']);
    assert.ok(Number(entry.startedAtMs) >= before);
    assert.ok(Number(entry.finishedAtMs) >= Number(entry.startedAtMs));
    const recording = entry.artifacts!.find(artifact => artifact.id === 'messages')!;
    assert.equal(Number(recording.sizeBytes), 'captured evidence'.length);
    assert.equal(recording.downloadable, true);
    assert.equal(found.status!.runId, runId);
    assert.equal(found.status!.phase, Phase.FINISHED);
    // Act
    const fd = await second.backend.openArtifact(runId, 'messages');
    // Assert: the past recording still downloads after restart.
    assert.equal(readFileSync(fd, 'utf8'), 'captured evidence');
    closeSync(fd);
  } finally {
    await second.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a long run error keeps its end in the history summary', async () => {
  // Arrange
  const root = await temporary(), store = await Store.open(root);
  const history = new History(store);
  const status = wire.Status.create({ phase: Phase.FINISHED, runId: id(1), outcome: { execution: 'Failed' } });
  try {
    // Act
    history.record(status, 5000, 6000, `${'x'.repeat(3000)} the cause`);
    const entry = history.list(wire.HistoryQuery.create()).entries[0];
    // Assert
    assert.equal(entry.errorSummary!.length, execution.Limit.MAX_ERROR_CHARS);
    assert.ok(entry.errorSummary!.endsWith('the cause'));
  } finally {
    store.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a long run error reaches the browser capped and stays whole in its report', async () => {
  // Arrange
  const root = await temporary(), run = await service(root);
  const error = `start of the error ${'x'.repeat(3000)} the cause`;
  const runId = await startRun(run);
  try {
    // Act
    await finishRun(run, runId, { error, boundaryClean: true });
    const found = await detail(run, runId);
    const report = JSON.parse(readFileSync(join(run.config.dataRoot, 'runs', runId, 'console-report.json'), 'utf8'));
    // Assert
    assert.equal(found.status!.outcome!.error!.length, execution.Limit.MAX_ERROR_CHARS);
    assert.ok(found.status!.outcome!.error!.endsWith('the cause'));
    assert.equal(report.error, error);
  } finally {
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a run open at a crash reopens as interrupted with no finish time or download', async () => {
  // Arrange: a crash leaves the row open and a partial recording behind.
  const root = await temporary(), store = await Store.open(root);
  const history = new History(store);
  await mkdir(join(root, 'runs', id(1)), { recursive: true });
  await writeFile(join(root, 'runs', id(1), 'cpu.mcap'), 'partial');
  history.record(wire.Status.create({ phase: Phase.RUNNING, runId: id(1), recordings: [{ id: 'cpu', path: 'cpu.mcap' }] }), 5000);
  try {
    // Act
    history.closeInterrupted();
    const entry = history.list(wire.HistoryQuery.create()).entries[0];
    // Assert
    assert.equal(entry.state, State.HISTORY_INTERRUPTED);
    assert.equal(Number(entry.startedAtMs), 5000);
    assert.equal(entry.finishedAtMs, null);
    assert.match(entry.errorSummary!, /stopped before/);
    assert.equal(Number(entry.artifacts![0].sizeBytes), 'partial'.length);
    assert.equal(entry.artifacts![0].downloadable, false);
  } finally {
    store.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a result that cannot reach History leaves the marker unfinished, so restart closes the run as interrupted', async () => {
  // Arrange
  const root = await temporary(), run = await service(root);
  const runId = await startRun(run);
  run.store.db.exec("CREATE TRIGGER refuse BEFORE UPDATE ON runs BEGIN SELECT RAISE(ABORT, 'disk full'); END");
  try {
    // Act
    await finishRun(run, runId);
    // Assert
    assert.ok(![Phase.FINISHED, Phase.BLOCKED].includes(run.store.marker()!.phase!));
    // Act
    run.store.db.exec('DROP TRIGGER refuse');
    new History(run.store).closeInterrupted();
    // Assert
    assert.equal((await list(run)).entries[0].state, State.HISTORY_INTERRUPTED);
  } finally {
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a report that cannot be written blocks the marker and History row alike', async () => {
  // Arrange
  const root = await temporary(), run = await service(root);
  const runId = await startRun(run);
  await mkdir(join(run.config.dataRoot, 'runs', runId), { recursive: true });
  await writeFile(join(run.config.dataRoot, 'runs', runId, 'console-report.json'), 'already here');
  try {
    // Act
    await finishRun(run, runId);
    // Assert
    assert.equal(run.store.marker()!.phase, Phase.BLOCKED);
    assert.equal((await list(run)).entries[0].state, State.HISTORY_BLOCKED);
  } finally {
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('a damaged database stops Console from opening and is left in place', async () => {
  // Arrange
  const root = await temporary(), path = join(root, 'console.db');
  await writeFile(path, 'this is not a database, and it must stay here');
  try {
    // Act / Assert
    await assert.rejects(Store.open(root));
    assert.match(readFileSync(path, 'utf8'), /must stay here/);
    // Act / Assert: the lock is released, so a repaired directory opens.
    await rm(path);
    (await Store.open(root)).close();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('the HTTP history API authorizes, bounds, paginates and filters', async () => {
  // Arrange
  const root = await temporary(), run = await service(root);
  const history = new History(run.store);
  const done = (runId: string, execution: string) => wire.Status.create({ phase: Phase.FINISHED, runId, outcome: { execution } });
  // Two runs share one start time so the cursor must break the tie by id.
  history.record(done(id(1), 'Completed'), 1000, 2000);
  history.record(done(id(2), 'Failed'), 2000, 3000);
  history.record(done(id(3), 'Completed'), 2000, 3000);
  history.record(done(id(4), 'Cancelled'), 4000, 5000);
  history.record(done(id(5), 'Completed'), 5000, 6000);
  const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin: 'http://console.test', assets: root });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`, cookie = await sessionCookie(base);
  const post = (query: object, headers: Record<string, string> = { cookie, origin: 'http://console.test' }) =>
    fetch(base + routes.history.path, { method: 'POST', headers: { 'content-type': MEDIA_TYPE, ...headers }, body: new Blob([new Uint8Array(wire.HistoryQuery.encode(query).finish())]) });
  const page = async (query: object) => wire.HistoryPage.decode(new Uint8Array(await (await post(query)).arrayBuffer()));
  const ids = (value: { entries: { runId?: string | null }[] }) => value.entries.map(entry => entry.runId);
  try {
    // Act / Assert: authorization and origin.
    assert.equal((await post({}, {})).status, 401);
    assert.equal((await post({}, { cookie, origin: 'http://evil.test' })).status, 403);
    assert.equal((await fetch(base + historyRunPath(id(1)))).status, 401);
    assert.equal((await fetch(base + historyRunPath(id(1)), { headers: { cookie, origin: 'http://evil.test' } })).status, 403);
    // Act / Assert: input bounds.
    assert.equal((await post({ limit: LIMITS.historyPageMax + 1 })).status, 400);
    assert.equal((await post({ limit: LIMITS.historyPageMax })).status, 200);
    assert.equal((await post({ cursor: `1:${id(1)}' OR 1=1` })).status, 400);
    assert.equal((await post({ cursor: 'garbage' })).status, 400);
    assert.equal((await post({ state: 99 })).status, 400);
    // Act / Assert: ids are strict.
    for (const bad of ['A'.repeat(32), id(1) + '0', '..%2f..%2fetc', id(1).slice(1)])
      assert.equal((await fetch(`${base}/api/history/${bad}`, { headers: { cookie } })).status, 404);
    assert.equal((await fetch(base + historyRunPath(id(99)), { headers: { cookie } })).status, 404);
    // Act: walk every page.
    const walked: string[] = []; let cursor = '', pages = 0;
    do {
      const next = await page({ limit: 2, cursor });
      assert.ok(next.entries.length <= 2);
      walked.push(...ids(next) as string[]); cursor = next.nextCursor!; pages++;
    } while (cursor);
    // Assert: newest first, ties stable, nothing repeated or lost.
    assert.deepEqual(walked, [id(5), id(4), id(3), id(2), id(1)]);
    assert.equal(pages, 3);
    // Act / Assert: the status filter.
    assert.deepEqual(ids(await page({ state: State.HISTORY_FAILED })), [id(2)]);
    // Act
    const response = await fetch(base + historyRunPath(id(2)), { headers: { cookie } });
    // Assert
    assert.equal(response.headers.get('content-type'), MEDIA_TYPE);
    const found = wire.HistoryDetail.decode(new Uint8Array(await response.arrayBuffer()));
    assert.equal(found.entry!.state, State.HISTORY_FAILED);
    assert.equal(found.status!.runId, id(2));
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});

test('past artifacts download only through safe paths and only when finalized', async () => {
  // Arrange
  const root = await temporary(), run = await service(root), runs = join(root, 'data', 'runs');
  await writeFile(join(root, 'secret.txt'), 'outside');
  const safe = id(1), traversal = id(2), linked = id(3), unfinished = id(4), linkedRun = id(5);
  await checkpoint(run.store, safe);
  await writeFile(join(runs, safe, 'nested/messages.mcap'), 'captured');
  await checkpoint(run.store, traversal, { recordings: [{ id: 'messages', path: '../../../secret.txt' }] });
  await checkpoint(run.store, linked);
  await symlink(join(root, 'secret.txt'), join(runs, linked, 'nested/messages.mcap'));
  await checkpoint(run.store, unfinished, { outcome: { execution: 'Completed', recordingFinalized: false } });
  await writeFile(join(runs, unfinished, 'nested/messages.mcap'), 'partial');
  await symlink(join(runs, safe), join(runs, linkedRun));
  const server = createConsoleServer({ backend: run.backend, secret: 'secret', origin: 'http://console.test', assets: root });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const address = server.address(); assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}`, cookie = await sessionCookie(base);
  const get = (path: string, headers: Record<string, string> = { cookie }) => fetch(base + path, { headers });
  try {
    // Act
    const download = await get(`/api/artifacts/${safe}/messages`);
    // Assert
    assert.equal(download.status, 200);
    assert.equal(await download.text(), 'captured');
    assert.equal((await get(`/api/artifacts/${safe}/messages`, {})).status, 401);
    // Act / Assert: traversal, symlinks, unfinalized recordings and undeclared kinds are refused.
    for (const path of [
      `/api/artifacts/${traversal}/messages`,
      `/api/artifacts/${linked}/messages`,
      `/api/artifacts/${unfinished}/messages`,
      `/api/artifacts/${linkedRun}/messages`,
      `/api/artifacts/${safe}/activity`,
      `/api/artifacts/${id(9)}/messages`,
      `/api/artifacts/..%2f..%2f/messages`,
      `/api/artifacts/${safe}/..%2fnested%2fmessages.mcap`,
    ]) assert.equal((await get(path)).status, 404, path);
  } finally {
    server.closeAllConnections(); server.close(); await once(server, 'close');
    await run.backend.close();
    await rm(root, { recursive: true, force: true });
  }
});
