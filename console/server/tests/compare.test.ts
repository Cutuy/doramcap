import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { compareReview, compareRuns, diffFacts, manifestFacts, readManifest } from '../src/compare.js';
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
const row = (comparison: ReturnType<typeof compareRuns>, name: string) => comparison.rows.find(item => item.name === name);

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
    side('a'.repeat(32), { before: state({ turboEnabled: false, swappiness: 0 }) }),
    side('b'.repeat(32), { before: state({ turboEnabled: true }) }),
  );
  // Assert
  assert.equal(row(comparison, 'turbo_enabled')?.result, Result.COMPARISON_CHANGED);
  assert.equal(cell(row(comparison, 'turbo_enabled'), 0), '');
  assert.equal(cell(row(comparison, 'turbo_enabled'), 1), 'true');
  assert.equal(row(comparison, 'swappiness'), undefined);
});

test('two captured runs compare by group, and a setting changed during a run differs in its group', () => {
  // Arrange
  const a = side('a'.repeat(32), { setup: { sourceCommit: 'abc', images: [{ service: 'worker', imageId: 'sha256:1' }] }, before: machine('powersave'), after: machine('performance') });
  const b = side('b'.repeat(32), { setup: { sourceCommit: 'abc', images: [{ service: 'worker', imageId: 'sha256:2' }] }, before: machine('powersave') });
  // Act
  const comparison = compareRuns(a, b);
  // Assert
  assert.equal(row(comparison, 'source_commit')?.result, Result.COMPARISON_SAME);
  assert.equal(row(comparison, 'image.worker')?.result, Result.COMPARISON_CHANGED);
  assert.equal(row(comparison, 'kernel_release')?.result, Result.COMPARISON_SAME);
  const governor = row(comparison, 'governor');
  assert.equal(governor?.group, Group.COMPARISON_CONFIGURATION);
  assert.equal(governor?.result, Result.COMPARISON_CHANGED);
  assert.match(cell(governor, 0) ?? '', /powersave.*performance/);
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

test('boot flags hide sensitive parameters and keep the rest', () => {
  // Arrange
  const state = (flags: string) => ({ configuration: { bootFlags: flags } });
  // Act
  const comparison = compareRuns(side('a'.repeat(32), { before: state('quiet isolcpus=2 password=one') }), side('b'.repeat(32), { before: state('quiet isolcpus=2 password=two') }));
  // Assert
  assert.equal(cell(row(comparison, 'boot_flags'), 0), 'quiet isolcpus=2 password=(hidden)');
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

test('a saved configuration opens without its run ID, so paths compare across runs', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-compare-')), id = 'a'.repeat(32);
  await mkdir(join(root, id));
  await writeFile(join(root, id, 'experiment.input.json'), JSON.stringify({ services: { worker: { environment: { OUTPUT: `/data/runs/${id}/out` } } } }));
  // Act
  const facts = manifestFacts(readManifest(root, id));
  // Assert
  assert.equal(facts.get('environment.worker.OUTPUT')?.value, '/data/runs/@run/out');
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
