import { strict as assert } from 'node:assert';
import { fstatSync } from 'node:fs';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { CpuReducer as CpuComparison, analyzeCpu as analyzeRun } from '../src/analysis/modules/cpu/analyzer.js';
import { compareCpu } from '../src/analysis/modules/cpu/compare.js';
import { encodeAnalysis, decodeAnalysis } from '../src/analysis/modules/cpu/protocol.js';
import { compareRuns, type Side } from '../src/records/compare.js';
import { analysis, capabilities, execution, launch, wire } from '../src/native.js';
import { cpuPacket, cpuRecording, dictionary, snapshot } from './cpu-fixture.js';

const analyzeCpu = (root: string, side: Side, signal?: AbortSignal) => analyzeRun(analysis.AnalysisRun.create({
  directory: join(root, side.id), detail: side.detail, capture: side.capture,
  artifacts: [{ name: 'configuration', path: 'experiment.input.json' }, { name: 'placement', path: 'system-app-cgroup.pb' }],
}), signal);

const run = 'a'.repeat(32), second = 1_000_000_000n;
const parent = 'doramcap-compose-0123456789ab-fedcba987654-user-worker.slice';
const path = '/doramcap.slice/doramcap-compose.slice/doramcap-compose-0123456789ab.slice/' +
  'doramcap-compose-0123456789ab-fedcba987654.slice/doramcap-compose-0123456789ab-fedcba987654-user.slice/' + parent;
const side = (): Side => ({
  id: run, manifest: null, capture: execution.RunCapture.create(),
  detail: wire.HistoryDetail.fromObject({ entry: { runId: run, startedAtMs: 1, finishedAtMs: 2 },
    status: { recordings: [{ id: 'cpu', path: 'cpu.mcap', cpu: true }], outcome: {
      execution: 'Completed', recordingFinalized: true, boundaryClean: true,
      serviceExits: [{ service: 'worker', exitCode: 137, killed: true }],
    } } }),
});
type Facts = Map<string, { value: string; unavailable?: boolean }>;
const facts = (analysis: ReturnType<CpuComparison['result']>): Facts => new Map(compareCpu(analysis, analysis).map(row => [row.name, { value: /cpu_(coverage|capture)$/.test(row.name) ? row.cells[0].displayValues![0] ?? row.cells[0].values![0] : row.cells[0].values![0], unavailable: row.result === wire.ComparisonResult.COMPARISON_UNAVAILABLE }]));
const cpuComparison = async (root: string, side: Side, signal?: AbortSignal) => facts(await analyzeCpu(root, side, signal));
const value = (facts: Facts, field: string) => facts.get('service.worker.' + field)?.value;
async function fixture(packets: Uint8Array[]) {
  const root = await mkdtemp(join(tmpdir(), 'compare-cpu-'));
  await mkdir(join(root, run));
  await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: { worker: { cgroup_parent: parent } } }));
  await writeFile(join(root, run, 'cpu.mcap'), await cpuRecording(packets));
  return root;
}

test('index-free recorded parent counters measure service CPU once across cores and all descendants', async () => {
  // Arrange: two seconds, with 3 CPU-seconds; child and host totals must not be added.
  const root = await fixture([dictionary(path), snapshot(1, second, [10n, 20n]),
    snapshot(2, 3n * second, [second + 10n, 2n * second + 20n])]);
  try {
    // Act
    const facts = await cpuComparison(root, side());
    // Assert
    assert.equal(value(facts, 'cpu_seconds'), '3.000000000');
    assert.equal(value(facts, 'cpu_mean_percent'), '150');
    assert.equal(value(facts, 'cpu_observed_seconds'), '2.000000000');
    assert.match(value(facts, 'cpu_coverage')!, /^Measured: 2.000000000 of 2.000000000/);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('a new session never inherits cgroup names and counter resets start a new baseline', () => {
  // Arrange
  const reduction = new CpuComparison([{ service: 'worker', path }]);
  reduction.receive(dictionary(path));
  reduction.receive(snapshot(1, second, [100n]));
  reduction.receive(snapshot(2, 2n * second, [200n]));
  // Act: reuse the numeric ID for a different path in a new monitor session.
  reduction.receive(dictionary('/other', 'new'));
  reduction.receive(snapshot(1, second, [1_000n], { sessionId: 'new' }));
  reduction.receive(snapshot(2, 2n * second, [2_000n], { sessionId: 'new' }));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), '0.000000100');
  assert.match(value(facts(reduction.result()), 'cpu_coverage')!, /Partial.*monitor restarted/);
  // Act: restore a recorded name; then a counter reset cannot become negative or huge CPU.
  reduction.receive(dictionary(path, 'new', 2));
  reduction.receive(snapshot(3, 3n * second, [3_000n], { sessionId: 'new', dictionaryRevision: 2 }));
  reduction.receive(snapshot(4, 4n * second, [1n], { sessionId: 'new', dictionaryRevision: 2 }));
  reduction.receive(snapshot(5, 5n * second, [6n], { sessionId: 'new', dictionaryRevision: 2 }));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), '0.000000105');
  assert.equal(value(facts(reduction.result()), 'cpu_observed_seconds'), '2.000000000');
  assert.match(value(facts(reduction.result()), 'cpu_coverage')!, /counter reset/);
});

test('measured zero stays zero; unavailable dictionaries and overflow never establish idle CPU', () => {
  // Arrange
  const reduction = new CpuComparison([{ service: 'worker', path }]);
  reduction.receive(dictionary(path));
  reduction.receive(snapshot(1, second, [0n]));
  // Act
  reduction.receive(snapshot(2, 2n * second, [0n]));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), '0.000000000');
  assert.equal(value(facts(reduction.result()), 'cpu_mean_percent'), '0');
  // Act: the requested dictionary revision has not been recorded.
  reduction.receive(snapshot(3, 3n * second, [second], { dictionaryRevision: 2 }));
  reduction.receive(snapshot(4, 4n * second, [2n * second], { dictionaryRevision: 2 }));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), '0.000000000');
  assert.match(value(facts(reduction.result()), 'cpu_coverage')!, /Partial/);
  const missing = new CpuComparison([{ service: 'worker', path }]);
  missing.receive(dictionary(path));
  for (const sequence of [1, 2]) missing.receive(snapshot(sequence, BigInt(sequence) * second, [0n], {
    snapshot: { schemaVersion: 1, sequence, monotonicEndNs: String(BigInt(sequence) * second), capacityOverflow: 1, records: [] },
  }));
  assert.equal(value(facts(missing.result()), 'cpu_seconds'), 'Unavailable');
  assert.equal(facts(missing.result()).get('service.worker.cpu_coverage')?.unavailable, true);
});

test('changed identities and duplicate per-CPU rows cannot double count a service', () => {
  // Arrange
  const reduction = new CpuComparison([{ service: 'worker', path }]);
  reduction.receive(dictionary(path));
  reduction.receive(snapshot(1, second, [10n]));
  // Act
  reduction.receive(cpuPacket({ snapshot: { version: 2, sessionId: 'session', dictionaryRevision: 1,
    snapshot: { schemaVersion: 1, sequence: 2, monotonicEndNs: String(2n * second), records: [
      { cgroupId: '9007199254740993', parentId: 1, cpu: 0, flags: 5, runtimeNs: 20 },
      { cgroupId: '9007199254740993', parentId: 1, cpu: 0, flags: 5, runtimeNs: 20 },
    ] } } }));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), 'Unavailable');
  assert.equal(facts(reduction.result()).get('service.worker.cpu_coverage')?.unavailable, true);
});

test('missing, custom, unfinalized and malformed sources preserve exits and explicit unavailable cells', async () => {
  // Arrange
  const root = await fixture([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
  try {
    const a = side(), b = side();
    const aCpu = await analyzeCpu(root, a);
    await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: { worker: { cgroup_parent: '/shared' } } }));
    const bCpu = await analyzeCpu(root, b);
    // Act
    const comparison = { rows: compareCpu(aCpu, bCpu) };
    // Assert: other saved outcome facts do not turn missing CPU into Changed / Not set.
    const cpu = comparison.rows.find(row => row.name === 'service.worker.cpu_seconds')!;
    assert.equal(cpu.result, wire.ComparisonResult.COMPARISON_UNAVAILABLE);
    assert.equal(cpu.cells![1].values![0], 'Unavailable');
    assert.equal(compareRuns(a, b).rows.find(row => row.name === 'service.worker.exit')?.cells?.[0].values?.[0], 'Exit 137; killed');
    // Act / Assert
    b.detail.status!.outcome!.recordingFinalized = false;
    assert.match(value(await cpuComparison(root, b), 'cpu_coverage')!, /not finalized/);
    b.detail.status!.outcome!.recordingFinalized = true;
    await writeFile(join(root, run, 'cpu.mcap'), Buffer.from('broken'));
    assert.match((await cpuComparison(root, b)).get('cpu_capture')!.value, /could not be read/);
    await rm(join(root, run, 'cpu.mcap'));
    await symlink('/etc/passwd', join(root, run, 'cpu.mcap'));
    assert.ok((await cpuComparison(root, b)).get('cpu_capture')!.unavailable);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('cancellation rejects the scan and closes the safely opened recording descriptor', async () => {
  // Arrange
  const root = await fixture([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
  const controller = new AbortController(), original = capabilities.openFile;
  let descriptor = -1, recordingOpens = 0;
  capabilities.openFile = (base, relative) => {
    const fd = original(base, relative);
    if (relative === 'cpu.mcap' && ++recordingOpens === 2) { descriptor = fd; queueMicrotask(() => controller.abort()); }
    return fd;
  };
  try {
    // Act / Assert
    await assert.rejects(cpuComparison(root, side(), controller.signal), { name: 'AbortError' });
    assert.ok(descriptor >= 0);
    assert.throws(() => fstatSync(descriptor), { code: 'EBADF' });
  } finally { capabilities.openFile = original; await rm(root, { recursive: true, force: true }); }
});

test('all numbered rotation segments contribute and a missing segment is unavailable', async () => {
  // Arrange
  const root = await fixture([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
  try {
    await writeFile(join(root, run, 'cpu.000001.mcap'), await cpuRecording([
      dictionary(path), snapshot(3, 3n * second, [2n * second]), snapshot(4, 4n * second, [3n * second]),
    ]));
    // Act / Assert
    const measured = await cpuComparison(root, side());
    assert.equal(value(measured, 'cpu_seconds'), '3.000000000');
    assert.equal(value(measured, 'cpu_observed_seconds'), '3.000000000');
    await rm(join(root, run, 'cpu.000001.mcap'));
    await writeFile(join(root, run, 'cpu.000002.mcap'), await cpuRecording([]));
    assert.match((await cpuComparison(root, side())).get('cpu_capture')!.value, /missing rotation segments/);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('cgroupfs and escaped systemd service names use original generated placement', async () => {
  // Arrange
  const name = 'camera-driver', cgroupfs = '/doramcap-compose-0123456789ab/fedcba987654/user/' + name;
  const escaped = parent.replace('worker', 'camera\\x2ddriver');
  const escapedPath = path.replace(parent, escaped);
  for (const [placement, actual] of [[cgroupfs, cgroupfs], [escaped, escapedPath]]) {
    const root = await fixture([]);
    try {
      await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: { [name]: { cgroup_parent: placement } } }));
      await writeFile(join(root, run, 'cpu.mcap'), await cpuRecording([dictionary(actual), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]));
      // Act / Assert
      const facts = await cpuComparison(root, side());
      assert.equal(facts.get('service.' + name + '.cpu_seconds')?.value, '1.000000000');
    } finally { await rm(root, { recursive: true, force: true }); }
  }
});

test('native owned unit counters are matched within saved repository scope, including custom slices', async () => {
  // Arrange
  const root = await fixture([]), unitPath = '/custom.slice/doramcap-0123456789ab-worker.service';
  await rm(join(root, run, 'experiment.input.json'));
  await writeFile(join(root, run, 'system-app-cgroup.pb'), launch.SystemAppCgroup.encode({
    id: 1, path: '/sys/fs/cgroup/doramcap.slice/doramcap-0123456789ab.slice/doramcap-0123456789ab-system.slice',
  }).finish());
  await writeFile(join(root, run, 'cpu.mcap'), await cpuRecording([
    dictionary(unitPath), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second]),
  ]));
  try {
    // Act / Assert
    assert.equal(value(await cpuComparison(root, side()), 'cpu_seconds'), '1.000000000');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('shared, nested and unresolved parents cannot establish exclusive service CPU', async () => {
  // Arrange
  const directory = '/doramcap-compose-0123456789ab/fedcba987654/user/worker';
  for (const [placement, actual, other] of [
    [parent, path, parent], [parent, path, parent.replace('.slice', '-helper.slice')],
    [directory, directory, directory], [directory, directory, directory + '/.'],
    [directory, directory, directory + '/'], [directory, directory, directory + '//helper'],
    [directory, directory, directory + '/helper'], [directory, directory, directory + '/helper/../.'],
    [directory, directory, 'relative-parent'], [directory, directory, 'relative.slice'],
  ]) {
    const root = await fixture([dictionary(actual), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
    await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: {
      worker: { cgroup_parent: placement }, helper: { cgroup_parent: other },
    } }));
    try {
      // Act
      const facts = await cpuComparison(root, side());
      // Assert
      for (const service of ['worker', 'helper']) {
        assert.equal(facts.get('service.' + service + '.cpu_seconds')?.unavailable, true);
        assert.equal(facts.get('service.' + service + '.cpu_coverage')?.unavailable, true);
      }
    } finally { await rm(root, { recursive: true, force: true }); }
  }
});

test('separate sibling service parents preserve exclusive inclusive-counter attribution', async () => {
  // Arrange
  const directory = '/doramcap-compose-0123456789ab/fedcba987654/user/worker';
  for (const [placement, actual, other] of [
    [parent, path, parent.replace('worker', 'helper')], [directory, directory, directory + '-helper'],
  ]) {
    const root = await fixture([dictionary(actual), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
    await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: {
      worker: { cgroup_parent: placement }, helper: { cgroup_parent: other },
    } }));
    try {
      // Act
      const facts = await cpuComparison(root, side());
      // Assert
      assert.equal(value(facts, 'cpu_seconds'), '1.000000000');
    } finally { await rm(root, { recursive: true, force: true }); }
  }
});

test('gaps retain measured cumulative deltas, and old snapshots cannot count an interval twice', () => {
  // Arrange
  const reduction = new CpuComparison([{ service: 'worker', path }]);
  reduction.receive(dictionary(path));
  reduction.receive(snapshot(1, second, [0n]));
  // Act: native cumulative counters still cover this interval despite skipped samples.
  reduction.receive(snapshot(3, 3n * second, [2n * second], {
    snapshot: { schemaVersion: 1, sequence: 3, monotonicEndNs: String(3n * second), droppedBatches: 1,
      records: [{ cgroupId: '9007199254740993', parentId: 1, cpu: 0, flags: 5, runtimeNs: String(2n * second) }] },
  }));
  reduction.receive(snapshot(2, 2n * second, [second]));
  reduction.receive(snapshot(4, 4n * second, [3n * second]));
  // Assert
  assert.equal(value(facts(reduction.result()), 'cpu_seconds'), '3.000000000');
  assert.equal(value(facts(reduction.result()), 'cpu_observed_seconds'), '3.000000000');
  assert.match(value(facts(reduction.result()), 'cpu_coverage')!, /Partial.*out-of-order.*snapshots dropped.*snapshots skipped/);
});

test('truncated and empty MCAP, ambiguous outputs and malformed placement leave saved exits intact', async () => {
  // Arrange
  const root = await fixture([]);
  try {
    const runSide = side();
    // Act / Assert
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /no CPU snapshots/i);
    const recording = await cpuRecording([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
    await writeFile(join(root, run, 'cpu.mcap'), recording.subarray(0, recording.length - 1));
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /incomplete/);
    runSide.detail.status!.recordings!.push({ id: 'other', path: 'other.mcap', cpu: true });
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /multiple CPU recordings/i);
    runSide.detail.status!.recordings!.pop();
    await writeFile(join(root, run, 'cpu.mcap'), recording);
    await writeFile(join(root, run, 'experiment.input.json'), '{broken');
    const cpu = await cpuComparison(root, runSide);
    const comparison = compareRuns(runSide, side());
    assert.equal(comparison.rows.find(row => row.name === 'service.worker.exit')?.cells?.[0].values?.[0], 'Exit 137; killed');
    assert.equal(value(cpu, 'cpu_seconds'), 'Unavailable');
    assert.match(value(cpu, 'cpu_coverage')!, /Unavailable/);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('saved output paths cannot enumerate or open escaped or symlinked parents', async () => {
  // Arrange
  const root = await fixture([]), external = await mkdtemp(join(tmpdir(), 'compare-outside-'));
  await writeFile(join(external, 'cpu.mcap'), await cpuRecording([]));
  await symlink(external, join(root, run, 'linked'));
  try {
    for (const source of ['../cpu.mcap', 'linked/cpu.mcap']) {
      const runSide = side();
      runSide.detail.status!.recordings![0].path = source;
      // Act / Assert
      const facts = await cpuComparison(root, runSide);
      assert.equal(facts.get('cpu_capture')!.unavailable, true);
      assert.equal(value(facts, 'cpu_seconds'), 'Unavailable');
    }
  } finally { await rm(root, { recursive: true, force: true }); await rm(external, { recursive: true, force: true }); }
});


test('CPU comparison distinguishes exact ratios above Number precision and equal ratios share an outcome', () => {
  // Arrange
  const result = (runtime: bigint, elapsed: bigint) => ({ services: new Map([['worker', { runtime, elapsed, span: elapsed, issues: [] as string[] }]]) });
  const runtime = 1n << 60n, a = result(runtime, runtime), b = result(runtime + 1n, runtime);
  // Act
  const changed = compareCpu(a, b).find(row => row.name.endsWith('cpu_mean_percent'))!;
  const same = compareCpu(a, result(runtime * 2n, runtime * 2n)).find(row => row.name.endsWith('cpu_mean_percent'))!;
  // Assert
  assert.equal(changed.result, wire.ComparisonResult.COMPARISON_CHANGED);
  assert.notEqual(changed.cells[0].displayValues![0], changed.cells[1].displayValues![0]);
  assert.equal(same.result, wire.ComparisonResult.COMPARISON_SAME);
});


test('a saved recording cannot read CPU evidence from an existing sibling run', async () => {
  // Arrange
  const root = await fixture([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
  const other = 'b'.repeat(32);
  await mkdir(join(root, other));
  await writeFile(join(root, other, 'cpu.mcap'), await cpuRecording([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [99n * second])]));
  const runSide = side();
  runSide.detail.status!.recordings![0].path = '../' + other + '/cpu.mcap';
  try {
    // Act
    const result = await analyzeCpu(root, runSide);
    // Assert
    assert.match(result.unavailable!, /could not be read/);
    assert.equal(result.services.get('worker')?.elapsed, 0n);
    assert.equal(result.services.get('worker')?.runtime, 0n);
  } finally { await rm(root, { recursive: true, force: true }); }
});


test('private per-run protobuf preserves exact counters, zero and unavailable evidence', () => {
  // Arrange
  const value = { services: new Map([['worker', { runtime: 9007199254740993n, elapsed: 0n, span: 18446744073709551615n, issues: ['partial'], unavailable: 'missing' }]]) };
  // Act
  const decoded = decodeAnalysis(encodeAnalysis(value));
  // Assert
  assert.deepEqual(decoded.services, value.services);
  assert.equal(decoded.unavailable, undefined);
});
