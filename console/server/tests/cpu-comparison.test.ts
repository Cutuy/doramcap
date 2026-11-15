import { strict as assert } from 'node:assert';
import { fstatSync } from 'node:fs';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { CpuComparison, cpuComparison } from '../src/records/cpu-comparison.js';
import { compareRuns, type Side } from '../src/records/compare.js';
import { capabilities, execution, launch, wire } from '../src/native.js';
import { cpuPacket, cpuRecording, dictionary, snapshot } from './cpu-fixture.js';

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
const value = (facts: Awaited<ReturnType<typeof cpuComparison>>, field: string) => facts.get('service.worker.' + field)?.value;
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
  assert.equal(value(reduction.facts(), 'cpu_seconds'), '0.000000100');
  assert.match(value(reduction.facts(), 'cpu_coverage')!, /Partial.*monitor restarted/);
  // Act: restore a recorded name; then a counter reset cannot become negative or huge CPU.
  reduction.receive(dictionary(path, 'new', 2));
  reduction.receive(snapshot(3, 3n * second, [3_000n], { sessionId: 'new', dictionaryRevision: 2 }));
  reduction.receive(snapshot(4, 4n * second, [1n], { sessionId: 'new', dictionaryRevision: 2 }));
  reduction.receive(snapshot(5, 5n * second, [6n], { sessionId: 'new', dictionaryRevision: 2 }));
  // Assert
  assert.equal(value(reduction.facts(), 'cpu_seconds'), '0.000000105');
  assert.equal(value(reduction.facts(), 'cpu_observed_seconds'), '2.000000000');
  assert.match(value(reduction.facts(), 'cpu_coverage')!, /counter reset/);
});

test('measured zero stays zero; unavailable dictionaries and overflow never establish idle CPU', () => {
  // Arrange
  const reduction = new CpuComparison([{ service: 'worker', path }]);
  reduction.receive(dictionary(path));
  reduction.receive(snapshot(1, second, [0n]));
  // Act
  reduction.receive(snapshot(2, 2n * second, [0n]));
  // Assert
  assert.equal(value(reduction.facts(), 'cpu_seconds'), '0.000000000');
  assert.equal(value(reduction.facts(), 'cpu_mean_percent'), '0');
  // Act: the requested dictionary revision has not been recorded.
  reduction.receive(snapshot(3, 3n * second, [second], { dictionaryRevision: 2 }));
  reduction.receive(snapshot(4, 4n * second, [2n * second], { dictionaryRevision: 2 }));
  // Assert
  assert.equal(value(reduction.facts(), 'cpu_seconds'), '0.000000000');
  assert.match(value(reduction.facts(), 'cpu_coverage')!, /Partial/);
  const missing = new CpuComparison([{ service: 'worker', path }]);
  missing.receive(dictionary(path));
  for (const sequence of [1, 2]) missing.receive(snapshot(sequence, BigInt(sequence) * second, [0n], {
    snapshot: { schemaVersion: 1, sequence, monotonicEndNs: String(BigInt(sequence) * second), capacityOverflow: 1, records: [] },
  }));
  assert.equal(value(missing.facts(), 'cpu_seconds'), undefined);
  assert.equal(missing.facts().get('service.worker.cpu_coverage')?.unavailable, true);
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
  assert.equal(value(reduction.facts(), 'cpu_seconds'), undefined);
  assert.equal(reduction.facts().get('service.worker.cpu_coverage')?.unavailable, true);
});

test('missing, custom, unfinalized and malformed sources preserve exits and explicit unavailable cells', async () => {
  // Arrange
  const root = await fixture([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
  try {
    const a = side(), b = side();
    a.cpu = await cpuComparison(root, a);
    await writeFile(join(root, run, 'experiment.input.json'), JSON.stringify({ services: { worker: { cgroup_parent: '/shared' } } }));
    b.cpu = await cpuComparison(root, b);
    // Act
    const comparison = compareRuns(a, b);
    // Assert: other saved outcome facts do not turn missing CPU into Changed / Not set.
    const cpu = comparison.rows.find(row => row.name === 'service.worker.cpu_seconds')!;
    assert.equal(cpu.result, wire.ComparisonResult.COMPARISON_UNAVAILABLE);
    assert.equal(cpu.cells![1].values![0], 'Unavailable');
    assert.equal(comparison.rows.find(row => row.name === 'service.worker.exit')?.cells?.[0].values?.[0], 'Exit 137; killed');
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
    if (relative === run + '/cpu.mcap' && ++recordingOpens === 2) { descriptor = fd; queueMicrotask(() => controller.abort()); }
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
        assert.equal(facts.get('service.' + service + '.cpu_seconds'), undefined);
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
  assert.equal(value(reduction.facts(), 'cpu_seconds'), '3.000000000');
  assert.equal(value(reduction.facts(), 'cpu_observed_seconds'), '3.000000000');
  assert.match(value(reduction.facts(), 'cpu_coverage')!, /Partial.*out-of-order.*snapshots dropped.*snapshots skipped/);
});

test('truncated and empty MCAP, ambiguous outputs and malformed placement leave saved exits intact', async () => {
  // Arrange
  const root = await fixture([]);
  try {
    const runSide = side();
    // Act / Assert
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /no CPU snapshots/);
    const recording = await cpuRecording([dictionary(path), snapshot(1, second, [0n]), snapshot(2, 2n * second, [second])]);
    await writeFile(join(root, run, 'cpu.mcap'), recording.subarray(0, recording.length - 1));
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /incomplete/);
    runSide.detail.status!.recordings!.push({ id: 'other', path: 'other.mcap', cpu: true });
    assert.match((await cpuComparison(root, runSide)).get('cpu_capture')!.value, /multiple CPU recordings/);
    runSide.detail.status!.recordings!.pop();
    await writeFile(join(root, run, 'cpu.mcap'), recording);
    await writeFile(join(root, run, 'experiment.input.json'), '{broken');
    runSide.cpu = await cpuComparison(root, runSide);
    const comparison = compareRuns(runSide, side());
    assert.equal(comparison.rows.find(row => row.name === 'service.worker.exit')?.cells?.[0].values?.[0], 'Exit 137; killed');
    assert.equal(value(runSide.cpu, 'cpu_seconds'), undefined);
    assert.match(value(runSide.cpu, 'cpu_coverage')!, /Unavailable/);
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
      assert.equal(value(facts, 'cpu_seconds'), undefined);
    }
  } finally { await rm(root, { recursive: true, force: true }); await rm(external, { recursive: true, force: true }); }
});
