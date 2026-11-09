import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { compareReview, compareRuns, diffFacts, machineChanges, manifestFacts, readManifest } from '../src/compare.js';
import { execution, wire } from '../src/native.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const entry = (runId: string, profiles: string[] = []) => wire.HistoryEntry.fromObject({ runId, profiles, startedAtMs: 1000, finishedAtMs: 3000, state: wire.HistoryState.HISTORY_COMPLETED });
const side = (id: string, capture: object = {}, manifest: object | null = null, profiles: string[] = []) => ({
  id, manifest,
  detail: wire.HistoryDetail.create({ entry: entry(id, profiles), status: wire.Status.create({ outcome: { execution: 'Completed', boundaryClean: true } }) }),
  capture: execution.RunCapture.fromObject(capture),
});
const machine = (governor: string, load = 1) => ({ configuration: { kernelRelease: '7.0', governor: [{ value: governor, cpus: '0-3' }] }, condition: { load_1m: load } });
const cell = (item: { cells?: { values?: string[] | null }[] | null } | undefined, side: number) => item?.cells?.[side]?.values?.[0];
const row = (comparison: Pick<ReturnType<typeof compareRuns>, 'rows'>, name: string) => comparison.rows.find(item => item.name === name);

test('a fact set in one run is a change, a side with no facts is unavailable, and a fact neither run set has no row', () => {
  // Arrange
  const a = new Map([['same', { value: '1' }], ['changed', { value: '1' }], ['only_a', { value: '1' }]]);
  const b = new Map([['same', { value: '1' }], ['changed', { value: '2' }], ['only_b', { value: '2' }]]);
  // Act
  const rows = Object.fromEntries(diffFacts(Group.COMPARISON_SETUP, a, b).map(item => [item.name, item]));
  const missing = diffFacts(Group.COMPARISON_SETUP, new Map(), b);
  // Assert
  assert.deepEqual(Object.keys(rows).sort(), ['changed', 'only_a', 'only_b', 'same']);
  assert.equal(rows.same.result, Result.COMPARISON_SAME);
  assert.equal(rows.changed.result, Result.COMPARISON_CHANGED);
  assert.equal(rows.only_a.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(rows.only_a, 1), '');
  assert.equal(rows.only_b.result, Result.COMPARISON_CHANGED);
  assert.ok(missing.every(item => item.result === Result.COMPARISON_UNAVAILABLE));
});

test('zero and false machine values equal an unset one, and differ from a set one', () => {
  // Arrange
  const state = (fields: object) => ({ configuration: { kernelRelease: '7.0', ...fields } });
  // Act
  const comparison = compareRuns(
    side('a'.repeat(32), { before: state({ turboEnabled: false, swappiness: 0, memoryTotalBytes: 0 }) }),
    side('b'.repeat(32), { before: state({ turboEnabled: true }) }),
  );
  // Assert
  assert.equal(row(comparison, 'turbo_enabled')?.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(row(comparison, 'turbo_enabled'), 0), '');
  assert.equal(cell(row(comparison, 'turbo_enabled'), 1), 'true');
  assert.equal(row(comparison, 'swappiness'), undefined);
  assert.equal(row(comparison, 'memory_total_bytes'), undefined);
});

test('host shm metadata survives stored captures and compares without changing configuration facts', () => {
  // Arrange
  const capture = (bytes: string) => execution.RunCapture.fromObject({ before: {
    ...machine('performance'),
    hostShm: { posix: [{ path: '/dev/shm/camera', device: '8', inode: '9007199254740993', sizeBytes: bytes, mode: 33152 }], systemV: [{ id: '12', key: '-4', sizeBytes: '4096', mode: 384 }] },
  } });
  const a = execution.RunCapture.decode(execution.RunCapture.encode(capture('4096')).finish());
  const b = execution.RunCapture.decode(execution.RunCapture.encode(capture('8192')).finish());

  // Act
  const comparison = compareRuns(side('a'.repeat(32), a), side('b'.repeat(32), b));

  // Assert
  const posix = row(comparison, 'host_shm.posix./dev/shm/camera');
  assert.equal(posix?.group, Group.COMPARISON_CONDITION);
  assert.equal(posix?.result, Result.COMPARISON_CHANGED);
  assert.equal(JSON.parse(cell(posix, 0)!).inode, '9007199254740993');
  assert.equal(JSON.parse(cell(posix, 1)!).sizeBytes, '8192');
  assert.equal(row(comparison, 'host_shm.system_v.12')?.result, Result.COMPARISON_SAME);
  assert.ok(comparison.rows.filter(item => item.group === Group.COMPARISON_CONFIGURATION).every(item => item.result === Result.COMPARISON_SAME));
});

test('host shm capture errors remain visible and never turn partial metadata into successful facts', () => {
  // Arrange
  const failed = side('a'.repeat(32), { before: { ...machine('performance'), hostShmError: '/dev/shm/private: permission denied', hostShm: { posix: [{ path: '/dev/shm/partial' }] } } });
  const empty = side('b'.repeat(32), { before: { ...machine('performance'), hostShm: {} } });

  // Act
  const comparison = compareRuns(failed, empty);
  const review = compareReview(null, { ...failed, profiles: [] });
  const changes = machineChanges(failed.capture.before, failed.capture.before);

  // Assert
  assert.equal(cell(row(comparison, 'host_shm'), 0), '');
  assert.equal(cell(row(comparison, 'host_shm'), 1), '0 POSIX entries, 0 System V segments');
  assert.equal(row(comparison, 'host_shm.posix./dev/shm/partial'), undefined);
  assert.match(cell(row(review, 'host_shm_error'), 1)!, /permission denied/);
  assert.equal(cell(changes.find(item => item.name === 'host_shm_error'), 1), '/dev/shm/private: permission denied');
});

test('two captured runs compare their Prepare captures by group', () => {
  // Arrange
  const a = side('a'.repeat(32), { setup: { sourceCommit: 'abc', images: [{ service: 'worker', imageId: 'sha256:1' }] }, before: machine('powersave') });
  const b = side('b'.repeat(32), { setup: { sourceCommit: 'abc', images: [{ service: 'worker', imageId: 'sha256:2' }] }, before: machine('performance') });
  // Act
  const comparison = compareRuns(a, b);
  // Assert
  assert.equal(row(comparison, 'source_commit')?.result, Result.COMPARISON_SAME);
  assert.equal(row(comparison, 'image.worker')?.result, Result.COMPARISON_CHANGED);
  assert.equal(row(comparison, 'kernel_release')?.result, Result.COMPARISON_SAME);
  assert.equal(row(comparison, 'governor')?.group, Group.COMPARISON_CONFIGURATION);
  assert.equal(row(comparison, 'governor')?.result, Result.COMPARISON_CHANGED);
});

test('capture errors stay visible when setup differences exhaust the comparison row budget', () => {
  // Arrange
  const before = { ...machine('performance'), hostShmError: '/dev/shm: permission denied' };
  const baseline = side('a'.repeat(32), { before });
  const current = side('b'.repeat(32), { before, setup: {
    images: Array.from({ length: 3000 }, (_, i) => ({ service: `service-${i}`, imageId: `image-${i}` })),
  } });
  // Act
  const comparisons = [compareRuns(baseline, current), compareReview(baseline, { ...current, profiles: [] })];
  // Assert
  for (const comparison of comparisons) {
    assert.equal(comparison.truncated, true);
    assert.equal(cell(row(comparison, 'host_shm_error'), 1), '/dev/shm: permission denied');
  }
});

test('post-run changes list only what differs, and nothing when a capture is missing', () => {
  // Arrange
  const state = (governor: string, load: number) => execution.RunCapture.fromObject({ before: machine(governor, load) }).before;
  // Act
  const changes = machineChanges(state('powersave', 1), state('performance', 2));
  // Assert
  assert.deepEqual(changes.map(item => [item.group, item.name]).sort(), [[Group.COMPARISON_CONFIGURATION, 'governor'], [Group.COMPARISON_CONDITION, 'load_1m']]);
  assert.deepEqual(machineChanges(null, state('performance', 2)), []);
});

test('sensitive values, Compose secrets and frozen files never reach a comparison, while changes still show', () => {
  // Arrange
  const digest = (name: string) => `/data/runs/@run/inputs/${name.repeat(32)}-644`;
  const manifest = (value: string, file: string) => ({
    services: { worker: { environment: { FIXED_DT: value, API_TOKEN: `token-${value}`, DB_URL: `postgres://robot:pw${value}@db/x`, CACHE_URL: `redis://:pw${value}@cache`, PLAIN: 'same' }, volumes: [{ type: 'bind', target: '/config.yaml', source: digest(file) }] } },
    configs: { settings: { file: digest(file) } },
    secrets: { api_key: { file: digest(file) } },
  });
  // Act
  const comparison = compareRuns(side('a'.repeat(32), {}, manifest('1', 'c')), side('b'.repeat(32), {}, manifest('2', 'd')));
  const wireText = Buffer.from(wire.HistoryComparison.encode(comparison).finish()).toString('latin1');
  // Assert
  assert.equal(cell(row(comparison, 'environment.worker.FIXED_DT'), 0), '1');
  assert.equal(cell(row(comparison, 'environment.worker.FIXED_DT'), 1), '2');
  for (const name of ['environment.worker.API_TOKEN', 'environment.worker.DB_URL', 'environment.worker.CACHE_URL', 'secret.api_key', 'config.settings', 'mount.worker./config.yaml']) {
    assert.equal(row(comparison, name)?.hidden, true, name);
    assert.equal(row(comparison, name)?.result, Result.COMPARISON_CHANGED, name);
    assert.equal(cell(row(comparison, name), 0), '', name);
  }
  assert.equal(row(comparison, 'environment.worker.PLAIN')?.result, Result.COMPARISON_SAME);
  for (const leak of ['token-1', 'pw1', 'cccccccc']) assert.ok(!wireText.includes(leak), leak);
});

test('boot flags compare original values between runs and at review while hiding sensitive parameters', () => {
  // Arrange
  const state = (password: string) => ({ configuration: { bootFlags: `quiet isolcpus=2 password=${password}` } });
  const baseline = side('a'.repeat(32), { before: state('first-secret') });
  const changed = side('b'.repeat(32), { before: state('second-secret') });
  // Act
  const comparisons = [
    compareRuns(baseline, changed),
    compareReview(baseline, { ...changed, profiles: [] }),
  ];
  const same = compareRuns(baseline, baseline);
  // Assert
  assert.equal(row(same, 'boot_flags')?.result, Result.COMPARISON_SAME);
  for (const comparison of comparisons) {
    const flags = row(comparison, 'boot_flags');
    assert.equal(flags?.result, Result.COMPARISON_CHANGED);
    for (const index of [0, 1]) assert.equal(cell(flags, index), 'quiet isolcpus=2 password=(hidden)');
    const encoded = Buffer.from(('sides' in comparison ? wire.HistoryComparison.encode(comparison) : wire.ReviewComparison.encode(comparison)).finish()).toString('latin1');
    for (const secret of ['first-secret', 'second-secret']) assert.ok(!encoded.includes(secret));
  }
});

test('a sensitive boot flag change during a run remains visible as a masked transition', () => {
  // Arrange
  const state = (password: string) => ({ configuration: { bootFlags: `quiet password=${password}` } });
  const before = execution.RunCapture.fromObject({ before: state('first-secret') }).before;
  const after = execution.RunCapture.fromObject({ before: state('second-secret') }).before;
  // Act
  const changes = machineChanges(before, after);
  // Assert
  const flags = changes.find(item => item.name === 'boot_flags');
  assert.equal(flags?.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(flags, 0), 'quiet password=(hidden)');
  assert.equal(cell(flags, 1), 'quiet password=(hidden)');
  const encoded = Buffer.from(wire.Status.encode({ machineChanges: changes }).finish()).toString('latin1');
  for (const secret of ['first-secret', 'second-secret']) assert.ok(!encoded.includes(secret));
});

test('a run in progress does not read as interrupted', () => {
  // Arrange
  const running = side('a'.repeat(32));
  running.detail.entry!.state = wire.HistoryState.HISTORY_RUNNING;
  running.detail.status!.outcome = null;
  // Act
  const comparison = compareRuns(running, side('b'.repeat(32)));
  // Assert
  assert.equal(cell(row(comparison, 'execution'), 0), 'In progress');
});

test('a saved configuration opens without its run ID or per-run ports, so runs compare cleanly', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-compare-')), id = 'a'.repeat(32);
  await mkdir(join(root, id));
  await writeFile(join(root, id, 'experiment.input.json'), JSON.stringify({ services: { worker: { environment: { OUTPUT: `/data/runs/${id}/out`, DORA_DAEMON_LOCAL_LISTEN_PORT: '1000' } } } }));
  // Act
  const facts = manifestFacts(readManifest(root, id));
  // Assert
  assert.equal(facts.get('environment.worker.OUTPUT')?.value, '/data/runs/@run/out');
  assert.equal(facts.has('environment.worker.DORA_DAEMON_LOCAL_LISTEN_PORT'), false);
});

test('a review compares setup and configuration with its baseline, and shows condition as current values', () => {
  // Arrange
  const baseline = side('a'.repeat(32), { setup: { sourceCommit: 'old' }, before: machine('performance', 1) }, null, ['gpu']);
  const current = { capture: execution.RunCapture.fromObject({ setup: { sourceCommit: 'new' }, before: machine('powersave', 3) }), manifest: null, profiles: ['gpu'], apps: null };
  // Act
  const comparison = compareReview(baseline, current);
  const rowNamed = (name: string) => comparison.rows.find(item => item.name === name);
  // Assert
  assert.equal(comparison.baseline?.runId, 'a'.repeat(32));
  assert.equal(rowNamed('source_commit')?.result, Result.COMPARISON_CHANGED);
  assert.equal(rowNamed('profiles')?.result, Result.COMPARISON_SAME);
  assert.equal(cell(rowNamed('governor'), 0), 'performance: 0-3');
  assert.equal(cell(rowNamed('governor'), 1), 'powersave: 0-3');
  assert.equal(cell(rowNamed('load_1m'), 0), '');
  assert.equal(cell(rowNamed('load_1m'), 1), '3');
});

test('a first review shows the current machine with no baseline', () => {
  // Arrange
  const current = { capture: execution.RunCapture.fromObject({ setup: { sourceCommit: 'new' }, before: machine('powersave') }), manifest: null, profiles: [], apps: null };
  // Act
  const comparison = compareReview(null, current);
  // Assert
  assert.equal(comparison.baseline, null);
  assert.ok(comparison.rows.every(row => row.group !== Group.COMPARISON_SETUP));
  assert.equal(comparison.rows.find(row => row.name === 'governor')?.cells?.[1]?.values?.[0], 'powersave: 0-3');
});

test('CPU zero remains a string fact for online, isolated and nohz CPUs', () => {
  // Arrange
  const zero = { before: { configuration: { kernelRelease: '7.0', onlineCpus: '0', isolatedCpus: '0', nohzFullCpus: '0' } } };
  const empty = { before: { configuration: { kernelRelease: '7.0' } } };
  // Act
  const comparison = compareRuns(side('a'.repeat(32), zero), side('b'.repeat(32), empty));
  // Assert
  for (const name of ['online_cpus', 'isolated_cpus', 'nohz_full_cpus']) {
    assert.equal(cell(row(comparison, name), 0), '0');
    assert.equal(row(comparison, name)?.result, Result.COMPARISON_CHANGED);
  }
});

test('unknown source state compares distinctly from verified clean and dirty states', () => {
  // Arrange
  const { SourceState } = execution;
  const captured = (sourceState: number) => side('a'.repeat(32), { setup: { sourceCommit: 'abc', sourceState } });
  const unknown = captured(SourceState.SOURCE_STATE_UNKNOWN);
  // Act
  const clean = compareRuns(unknown, captured(SourceState.SOURCE_STATE_CLEAN));
  const dirty = compareRuns(unknown, captured(SourceState.SOURCE_STATE_DIRTY));
  // Assert
  assert.equal(cell(row(clean, 'source_state'), 0), 'Unknown');
  assert.equal(cell(row(clean, 'source_state'), 1), 'Clean');
  assert.equal(cell(row(dirty, 'source_state'), 1), 'Dirty');
  assert.equal(row(clean, 'source_state')?.result, Result.COMPARISON_CHANGED);
  assert.equal(row(dirty, 'source_state')?.result, Result.COMPARISON_CHANGED);
});
