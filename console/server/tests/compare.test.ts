import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { compareReview, compareRuns, diffFacts, machineChanges, setupFacts } from '../src/records/compare.js';
import { compose, execution, wire } from '../src/native.js';
import { readSetupRecord } from '../src/records/setup.js';
import type { doramcap } from '../src/generated/console.js';
import { filesystem } from '../src/records/file-changes.js';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const entry = (runId: string, profiles: string[] = []) => wire.HistoryEntry.fromObject({ runId, profiles, startedAtMs: 1000, finishedAtMs: 3000, state: wire.HistoryState.HISTORY_COMPLETED });
const side = (id: string, capture: object = {}, setup: typeof compose.SetupRecord.prototype | null = null, profiles: string[] = []) => ({
  id, setup,
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
    hostShm: [{ path: '/dev/shm/camera', modifiedUnixNs: '1700000000123456789', changedUnixNs: '1700000000987654321', sizeBytes: bytes, mode: 33152 }],
  } });
  const a = execution.RunCapture.decode(execution.RunCapture.encode(capture('4096')).finish());
  const b = execution.RunCapture.decode(execution.RunCapture.encode(capture('8192')).finish());

  // Act
  const comparison = compareRuns(side('a'.repeat(32), a), side('b'.repeat(32), b));

  // Assert
  const file = row(comparison, 'host_shm./dev/shm/camera');
  assert.equal(file?.group, Group.COMPARISON_CONDITION);
  assert.equal(file?.result, Result.COMPARISON_CHANGED);
  assert.equal(JSON.parse(cell(file, 0)!).modifiedUnixNs, '1700000000123456789');
  assert.equal(JSON.parse(cell(file, 1)!).changedUnixNs, '1700000000987654321');
  assert.equal(JSON.parse(cell(file, 1)!).sizeBytes, '8192');
  assert.ok(comparison.rows.filter(item => item.group === Group.COMPARISON_CONFIGURATION).every(item => item.result === Result.COMPARISON_SAME));
});

test('host shm capture errors remain visible and never turn partial metadata into successful facts', () => {
  // Arrange
  const failed = side('a'.repeat(32), { beforeError: '/dev/shm/private: permission denied' });
  const empty = side('b'.repeat(32), { before: { ...machine('performance'), hostShm: [] } });

  // Act
  const comparison = compareRuns(failed, empty);
  const review = compareReview(null, { ...failed, profiles: [] });
  const changes = machineChanges(failed.capture.before, failed.capture.before, failed.capture.beforeError, failed.capture.beforeError);

  // Assert
  assert.equal(cell(row(comparison, 'host_shm'), 0), '');
  assert.equal(cell(row(comparison, 'host_shm'), 1), '0 entries');
  assert.equal(row(comparison, 'host_shm./dev/shm/partial'), undefined);
  assert.match(cell(row(review, 'machine_capture_error'), 1)!, /permission denied/);
  assert.equal(cell(changes.find(item => item.name === 'machine_capture_error'), 1), '/dev/shm/private: permission denied');
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
  const beforeError = '/dev/shm: permission denied';
  const baseline = side('a'.repeat(32), { beforeError });
  const current = side('b'.repeat(32), { beforeError, setup: {
    images: Array.from({ length: 3000 }, (_, i) => ({ service: `service-${i}`, imageId: `image-${i}` })),
  } });
  // Act
  const comparisons = [compareRuns(baseline, current), compareReview(baseline, { ...current, profiles: [] })];
  // Assert
  for (const comparison of comparisons) {
    assert.equal(comparison.truncated, true);
    assert.equal(cell(row(comparison, 'machine_capture_error'), 1), '/dev/shm: permission denied');
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

test('live machine changes refuse an oversized comparison with a visible error', () => {
  // Arrange
  const before = { ...machine('performance'), hostShm: [] };
  const after = { ...machine('performance'), hostShm: Array.from({ length: 3000 }, (_, i) => ({ path: `/dev/shm/object-${i}` })) };
  // Act
  const changes = machineChanges(before, after);
  // Assert
  assert.equal(changes.length, 1);
  assert.equal(changes[0].name, 'machine_capture_error');
  assert.match(cell(changes[0], 1)!, /too large/);
});

test('sensitive values, Compose secrets and frozen files never reach a comparison, while changes still show', () => {
  // Arrange
  const digest = (name: string) => `/data/runs/@run/inputs/${name.repeat(32)}-644`;
  const manifest = (value: string, file: string) => compose.SetupRecord.fromObject({
    services: { worker: { environment: { FIXED_DT: value, API_TOKEN: `token-${value}`, DB_URL: `postgres://robot:pw${value}@db/x`, CACHE_URL: `redis://:pw${value}@cache`, PLAIN: 'same' }, binds: [{ target: '/config.yaml', source: digest(file) }] } },
    configs: { settings: digest(file) },
    secrets: { api_key: digest(file) },
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
  const facts = setupFacts(readSetupRecord(root, id));
  // Assert
  assert.equal(facts.get('environment.worker.OUTPUT')?.value, '/data/runs/@run/out');
  assert.equal(facts.has('environment.worker.DORA_DAEMON_LOCAL_LISTEN_PORT'), false);
});

test('a review compares setup and configuration with its baseline, and shows condition as current values', () => {
  // Arrange
  const baseline = side('a'.repeat(32), { setup: { sourceCommit: 'old' }, before: machine('performance', 1) }, null, ['gpu']);
  const current = { capture: execution.RunCapture.fromObject({ setup: { sourceCommit: 'new' }, before: machine('powersave', 3) }), setup: null, profiles: ['gpu'], apps: null };
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
  const current = { capture: execution.RunCapture.fromObject({ setup: { sourceCommit: 'new' }, before: machine('powersave') }), setup: null, profiles: [], apps: null };
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

test('Compare and Review show the chosen write policy, with historical absence meaning write back', () => {
  // Arrange
  const historical = side('a'.repeat(32));
  const kept = side('b'.repeat(32));
  kept.detail.status!.writePolicy = filesystem.WritePolicy.WRITE_POLICY_KEEP;
  // Act
  const comparisons = [compareRuns(historical, kept), compareReview(historical, { ...kept, profiles: [], writePolicy: filesystem.WritePolicy.WRITE_POLICY_KEEP })];
  // Assert
  for (const comparison of comparisons) {
    const policy = row(comparison, 'write_policy');
    assert.equal(policy?.group, Group.COMPARISON_SETUP);
    assert.equal(policy?.result, Result.COMPARISON_CHANGED);
    assert.equal(cell(policy, 0), 'WRITE_POLICY_WRITE_BACK');
    assert.equal(cell(policy, 1), 'WRITE_POLICY_KEEP');
  }
  assert.equal(row(compareRuns(historical, historical), 'write_policy')?.result, Result.COMPARISON_SAME);
});


const id = 'a'.repeat(32);
const value = (input: string | string[] | null | undefined): doramcap.compose.v1.IInvocationValue | null => input == null ? null
  : typeof input === 'string' ? { text: input } : { arguments: { values: input } };
const invocation = (command?: string | string[] | null, entrypoint?: string | string[] | null, stopSignal?: string | null, stopGracePeriod?: string | null) => ({
  command: value(command), entrypoint: value(entrypoint), stopSignal: stopSignal ?? '', stopGracePeriod: stopGracePeriod ?? '',
});
const record = (command?: string | string[] | null, entrypoint?: string | string[] | null, stopSignal?: string | null, stopGracePeriod?: string | null) => compose.SetupRecord.fromObject({
  version: 3, runId: id, complete: true, services: { worker: {
    environment: { RATE: '1' }, captured: invocation(command, entrypoint, stopSignal, stopGracePeriod), normalized: invocation(command, entrypoint, stopSignal, stopGracePeriod),
  } },
});
const save = (root: string, setup: doramcap.compose.v1.ISetupRecord, directory = id) =>
  writeFile(join(root, directory, 'experiment.setup.pb'), compose.SetupRecord.encode(setup).finish());

test('History and Review compare typed service invocations with exact boundaries', () => {
  // Arrange
  const a = side(id, {}, record(['capture', 'a b'], ['/entrypoint'], 'SIGINT'));
  const b = side('b'.repeat(32), {}, record(['capture', 'a', 'b'], [], 'SIGTERM'));
  // Act
  const comparisons = [compareRuns(a, b), compareReview(a, { ...b, profiles: [] })];
  // Assert
  for (const comparison of comparisons) {
    for (const field of ['command', 'entrypoint', 'stop_signal']) assert.equal(row(comparison, `${field}.worker`)?.result, Result.COMPARISON_CHANGED);
    assert.equal(cell(row(comparison, 'command.worker'), 0), '["capture","a b"]');
    assert.equal(cell(row(comparison, 'command.worker'), 1), '["capture","a","b"]');
    assert.equal(cell(row(comparison, 'entrypoint.worker'), 1), '[]');
    assert.equal(cell(row(comparison, 'stop_signal.worker'), 0), '"SIGINT"');
  }
});

test('Compose inheritance and empty overrides survive protobuf without redundant null states', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-presence-'));
  await mkdir(join(root, id));
  const expected = ['', '', '[]', '[]'];
  // Act / Assert
  for (const [index, input] of [undefined, null, '', []].entries()) {
    await save(root, record(input));
    const saved = side(id, {}, readSetupRecord(root, id));
    const comparison = compareRuns(side(id, {}, record()), saved);
    assert.equal(cell(row(comparison, 'command.worker'), 1), expected[index]);
    assert.equal(row(comparison, 'command.worker')?.result, index >= 2 ? Result.COMPARISON_CHANGED : Result.COMPARISON_SAME);
    assert.equal(row(compareRuns(saved, saved), 'command.worker')?.result, Result.COMPARISON_SAME);
  }
  assert.equal(row(compareRuns(side(id, {}, record('')), side(id, {}, record([]))), 'command.worker')?.result, Result.COMPARISON_SAME);
});

test('typed invocations mask named credentials and URLs before either API exposes them', () => {
  // Arrange
  const secret = 'NEVER_EXPOSE_';
  const commands = (suffix: string) => [
    ['worker', '--api-key', secret + suffix], ['worker', '--key=' + secret + suffix], ['worker', '--password', secret + suffix],
    ['sh', '-c', 'exec worker --access_key=' + secret + suffix],
    ['sh', '-c', 'worker --key\t' + secret + suffix], ['sh', '-c', 'worker --key\n' + secret + suffix],
    ['worker', 'postgres://robot:' + secret + suffix + '@db/x'],
  ];
  // Act / Assert
  for (let i = 0; i < commands('a').length; i++) {
    const a = side(id, {}, record(commands('a')[i], commands('a')[i]));
    const b = side('b'.repeat(32), {}, record(commands('b')[i], commands('b')[i]));
    for (const comparison of [compareRuns(a, b), compareReview(a, { ...b, profiles: [] })]) {
      for (const field of ['command', 'entrypoint']) {
        assert.equal(row(comparison, `${field}.worker`)?.result, Result.COMPARISON_CHANGED);
        assert.equal(row(comparison, `${field}.worker`)?.hidden, true);
        assert.equal(cell(row(comparison, `${field}.worker`), 0), '');
        assert.equal(cell(row(comparison, `${field}.worker`), 1), '');
      }
      const bytes = 'sides' in comparison ? wire.HistoryComparison.encode(comparison).finish() : wire.ReviewComparison.encode(comparison).finish();
      assert.ok(!Buffer.from(bytes).toString('latin1').includes(secret));
    }
  }
});

test('long commands compare before display truncation', () => {
  // Arrange
  const a = side(id, {}, record(['worker', 'x'.repeat(1000) + 'first']));
  const b = side('b'.repeat(32), {}, record(['worker', 'x'.repeat(1000) + 'second']));
  // Act
  const command = row(compareRuns(a, b), 'command.worker');
  // Assert
  assert.equal(command?.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(command, 0), cell(command, 1));
  assert.equal(cell(command, 0)?.length, 513);
});

test('one protobuf loads from both run and Review paths without reading Compose JSON', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-setup-'));
  await mkdir(join(root, id, 'resolved'), { recursive: true });
  const setup = record(['worker', id], ['/bin/' + id], 'SIGINT');
  setup.services.worker.environment = { OUTPUT: `/data/runs/${id}/output` };
  setup.services.worker.binds = [{ source: '/input', target: '/config' }];
  setup.configs.settings = `/inputs/${'c'.repeat(32)}-444`;
  setup.secrets.credential = `/inputs/${'d'.repeat(32)}-400`;
  setup.services.empty = compose.SetupService.fromObject({ captured: invocation(''), normalized: invocation('') });
  setup.services.default = compose.SetupService.fromObject({ captured: invocation(), normalized: invocation() });
  for (const directory of [id, `${id}/resolved`]) await save(root, setup, directory);
  // Act / Assert
  for (const directory of [id, `${id}/resolved`]) {
    const saved = readSetupRecord(root, id, directory)!;
    assert.equal(setupFacts(saved).get('environment.worker.OUTPUT')?.value, '/data/runs/@run/output');
    const comparison = compareRuns(side(id, {}, saved), side(id, {}, saved));
    assert.equal(cell(row(comparison, 'command.worker'), 0), JSON.stringify(['worker', id]));
    assert.equal(cell(row(comparison, 'entrypoint.worker'), 0), JSON.stringify(['/bin/' + id]));
    assert.equal(cell(row(comparison, 'stop_signal.worker'), 0), '"SIGINT"');
    assert.equal(row(comparison, 'command.worker')?.result, Result.COMPARISON_SAME);
    assert.equal(saved.configs.settings, setup.configs.settings);
    assert.equal(saved.secrets.credential, setup.secrets.credential);
    assert.equal(row(comparison, 'secret.credential')?.hidden, true);
    assert.deepEqual(saved.services.worker.binds?.map(bind => [bind.source, bind.target]), [['/input', '/config']]);
    assert.equal(cell(row(comparison, 'command.empty'), 0), '[]');
    assert.equal(cell(row(comparison, 'command.default'), 0), '');
  }
});

test('missing, invalid, incomplete, oversized and symlinked protobuf falls back to legacy captured settings', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-setup-invalid-'));
  await mkdir(join(root, id));
  const worker = { command: ['worker', id], entrypoint: ['/worker'], stop_signal: 'SIGINT', environment: { RATE: '1' } };
  await writeFile(join(root, id, 'experiment.input.json'), JSON.stringify({ services: { worker } }));
  const good = record(['worker']);
  const invalid = [Buffer.from([255]), Buffer.alloc(0), compose.SetupRecord.encode(good).finish().slice(0, -2),
    compose.SetupRecord.encode({ ...good, version: 4 }).finish(), compose.SetupRecord.encode({ ...good, runId: 'b'.repeat(32) }).finish(),
    compose.SetupRecord.encode({ ...good, services: {} }).finish(),
    compose.SetupRecord.encode({ ...good, services: { worker: { captured: invocation() } } }).finish(),
    compose.SetupRecord.encode({ ...good, services: { worker: { normalized: invocation() } } }).finish(),
    Buffer.alloc(4 * 1024 * 1024 + 1)];
  // Act / Assert
  for (const bytes of [null, ...invalid]) {
    if (bytes) await writeFile(join(root, id, 'experiment.setup.pb'), bytes);
    const saved = readSetupRecord(root, id)!;
    const a = side(id, {}, saved), b = side('b'.repeat(32), {}, record(['worker']));
    for (const comparison of [compareRuns(a, b), compareReview(a, { ...b, profiles: [] })]) {
      assert.equal(row(comparison, 'command.worker')?.result, Result.COMPARISON_UNAVAILABLE);
      assert.equal(cell(row(comparison, 'command.worker'), 0), JSON.stringify(worker.command));
      assert.equal(row(comparison, 'environment.worker.RATE')?.result, Result.COMPARISON_SAME);
      assert.equal(row(compareRuns(a, a), 'execution')?.result, Result.COMPARISON_SAME);
    }
  }
  await rm(join(root, id, 'experiment.setup.pb'));
  const other = 'b'.repeat(32);
  await mkdir(join(root, other));
  await save(root, good, other);
  await symlink(join(root, other, 'experiment.setup.pb'), join(root, id, 'experiment.setup.pb'));
  assert.equal(readSetupRecord(root, id)?.services.worker.normalized, null);
  await rm(join(root, id, 'experiment.input.json'));
  assert.equal(readSetupRecord(root, id), null);
});

test('the read budget accepts 4 MiB including unknown protobuf fields and rejects one byte more', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-setup-budget-'));
  await mkdir(join(root, id));
  const encoded = compose.SetupRecord.encode(record(['worker'])).finish();
  const limit = 4 * 1024 * 1024;
  // Unknown field 100 is a length-delimited value; its size uses four varint bytes.
  const payloadSize = limit - encoded.length - 6;
  const length = Buffer.from([payloadSize & 127 | 128, payloadSize >>> 7 & 127 | 128, payloadSize >>> 14 & 127 | 128, payloadSize >>> 21]);
  const bytes = Buffer.concat([encoded, Buffer.from([0xa2, 0x06]), length, Buffer.alloc(payloadSize)]);
  // Act / Assert
  assert.equal(bytes.length, limit);
  await writeFile(join(root, id, 'experiment.setup.pb'), bytes);
  assert.ok(readSetupRecord(root, id)?.services.worker.normalized);
  await writeFile(join(root, id, 'experiment.setup.pb'), Buffer.concat([bytes, Buffer.from([0])]));
  assert.equal(readSetupRecord(root, id), null);
});

test('older snapshot versions fall back to captured JSON instead of being read as the current version', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-setup-v1-'));
  await mkdir(join(root, id));
  await save(root, { ...record(['worker']), version: 1 });
  await writeFile(join(root, id, 'experiment.input.json'), JSON.stringify({ services: { worker: { command: ['worker'], environment: { RATE: '1' } } } }));
  // Act
  const saved = readSetupRecord(root, id)!;
  const comparison = compareRuns(side(id, {}, saved), side(id, {}, saved));
  // Assert
  assert.equal(row(comparison, 'command.worker')?.result, Result.COMPARISON_UNAVAILABLE);
  assert.equal(cell(row(comparison, 'command.worker'), 0), '["worker"]');
  assert.equal(row(comparison, 'environment.worker.RATE')?.result, Result.COMPARISON_SAME);
});


test('History and Review compare shutdown grace periods including the run default', () => {
  // Arrange
  const one = side(id, {}, record(['worker'], null, 'SIGINT', '1s'));
  const five = side('b'.repeat(32), {}, record(['worker'], null, 'SIGINT', '5s'));
  const inherited = side('c'.repeat(32), {}, record(['worker'], null, 'SIGINT'));
  // Act / Assert
  for (const comparison of [compareRuns(one, five), compareReview(one, { ...five, profiles: [] })]) {
    const grace = row(comparison, 'stop_grace_period.worker');
    assert.equal(grace?.result, Result.COMPARISON_CHANGED);
    assert.equal(cell(grace, 0), '"1s"');
    assert.equal(cell(grace, 1), '"5s"');
    assert.equal(row(comparison, 'command.worker')?.result, Result.COMPARISON_SAME);
  }
  assert.equal(row(compareRuns(one, inherited), 'stop_grace_period.worker')?.result, Result.COMPARISON_CHANGED);
  assert.equal(row(compareRuns(inherited, inherited), 'stop_grace_period.worker')?.result, Result.COMPARISON_SAME);
});

test('version 2 History keeps command comparisons and leaves shutdown grace unavailable', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-setup-v2-'));
  await mkdir(join(root, id));
  await save(root, { ...record(['worker']), version: 2 });
  try {
    // Act
    const historical = side(id, {}, readSetupRecord(root, id));
    const current = side('b'.repeat(32), {}, record(['worker'], null, null, '5s'));
    // Assert
    for (const comparison of [compareRuns(historical, current), compareReview(historical, { ...current, profiles: [] }), compareRuns(historical, historical)]) {
      assert.equal(row(comparison, 'stop_grace_period.worker')?.result, Result.COMPARISON_UNAVAILABLE);
      assert.equal(row(comparison, 'command.worker')?.result, Result.COMPARISON_SAME);
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('per-service exit changes preserve replicas, zero exits and missing evidence independently', () => {
  // Arrange
  const a = side('a'.repeat(32)), b = side('b'.repeat(32));
  a.detail.status!.outcome!.serviceExits = [
    { service: 'worker', exitCode: 137, killed: true }, { service: 'worker', exitCode: 0 },
    { service: 'other', exitCode: 137, oomKilled: true, killed: true },
  ];
  b.detail.status!.outcome!.serviceExits = [{ service: 'worker', exitCode: 0 }];
  // Act
  const compared = compareRuns(a, b);
  // Assert
  assert.equal(cell(row(compared, 'service.worker.exit'), 0), 'Exit 0, Exit 137; killed');
  assert.equal(cell(row(compared, 'service.worker.exit'), 1), 'Exit 0');
  assert.equal(row(compared, 'service.worker.exit')?.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(row(compared, 'service.other.exit'), 0), 'Exit 137; OOM killed');
  assert.equal(cell(row(compared, 'service.other.exit'), 1), 'Unavailable');
  assert.equal(row(compared, 'service.other.exit')?.result, Result.COMPARISON_UNAVAILABLE);
});
