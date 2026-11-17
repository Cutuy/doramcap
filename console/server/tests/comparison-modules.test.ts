import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { compose, execution, wire } from '../src/native.js';
import type { doramcap } from '../src/generated/console.js';
import { filesystem } from '../src/records/file-changes.js';
import { readSetupRecord, STOP_GRACE_PERIOD_VERSION } from '../src/records/setup.js';
import { AnalysisRegistry, analysisRegistry } from '../src/analysis/registry.js';
import { AnalysisRunner } from '../src/analysis/runner.js';
import { runModule, RUN_MODULE_ID } from '../src/analysis/modules/run/module.js';
import { machineModule, MACHINE_MODULE_ID } from '../src/analysis/modules/machine/module.js';
import { LIMITS } from '../src/generated/transport.js';
import { compareReview, type Side } from './comparison-fixtures.js';
import { reviewRows, machineChangeRows, boundReviewComparison, boundMachineChanges } from '../src/analysis/comparisons.js';
import '../src/analysis/modules/index.js';

const gate = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };
async function saved(root: string, id: string, governor: string) {
  const directory = join(root, id);
  await mkdir(directory);
  const invocation = { command: { arguments: { values: ['worker'] } }, stopSignal: 'SIGINT', stopGracePeriod: '5s' };
  const setup = compose.SetupRecord.create({ version: STOP_GRACE_PERIOD_VERSION, complete: true, runId: id,
    services: { worker: { captured: invocation, normalized: invocation, environment: { OUTPUT: `/data/runs/${id}/output`, API_KEY: 'SECRET_' + id } } },
  });
  await writeFile(join(directory, 'experiment.setup.pb'), compose.SetupRecord.encode(setup).finish());
  return wire.AnalysisRun.create({ directory, snapshotHistory: { entry: { runId: id, state: wire.HistoryState.HISTORY_COMPLETED },
    status: { outcome: { execution: 'Completed', serviceExits: [{ service: 'worker', exitCode: 0 }] }, writePolicy: filesystem.WritePolicy.WRITE_POLICY_WRITE_BACK } },
    capture: { before: { configuration: { kernelRelease: '7.0', governor: [{ value: governor, cpus: '0-3' }] }, condition: { load_1m: 1 } } },
  });
}
const row = (result: { rows: doramcap.console.v1.ComparisonFact.$Properties[] }, name: string) => result.rows.find(value => value.name === name);
const runner = () => { const registry = new AnalysisRegistry(); registry.register(runModule); registry.register(machineModule); return new AnalysisRunner(registry); };

test('the production registry contains the run and machine modules', () => {
  // Arrange / Act
  const modules = analysisRegistry.list();
  // Assert
  assert.deepEqual(modules.map(value => value.id), [RUN_MODULE_ID, MACHINE_MODULE_ID]);
});

test('run and machine modules normalize original saved evidence and return while an optional module is stalled', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'comparison-runs-'));
  const a = await saved(root, 'a'.repeat(32), 'powersave'), b = await saved(root, 'b'.repeat(32), 'performance');
  const held = gate(), entered = gate(), registry = new AnalysisRegistry();
  registry.register(runModule); registry.register(machineModule);
  registry.register({ id: 'held', label: 'Held', analyze: async () => { entered.resolve(); await held.promise; return 1; }, compare: () => [] });
  const tasks = new AnalysisRunner(registry);
  let optionalDone = false;
  const optional = tasks.run('held', a, b, new AbortController().signal).then(() => { optionalDone = true; });
  try {
    await entered.promise;
    // Act
    const result = await tasks.run(RUN_MODULE_ID, a, b, new AbortController().signal);
    // Assert
    assert.equal(optionalDone, false);
    const machine = await tasks.run(MACHINE_MODULE_ID, a, b, new AbortController().signal);
    assert.equal(row(machine, 'governor')?.result, wire.ComparisonResult.COMPARISON_RESULT_CHANGED);
    assert.equal(row(result, 'governor'), undefined);
    assert.equal(row(result, 'environment.worker.OUTPUT')?.result, wire.ComparisonResult.COMPARISON_RESULT_SAME);
    assert.equal(row(result, 'environment.worker.OUTPUT')?.cells?.[0].values?.[0], '/data/runs/@run/output');
    assert.equal(readSetupRecord(root, 'a'.repeat(32))?.services.worker.environment?.OUTPUT, `/data/runs/${'a'.repeat(32)}/output`);
    assert.equal(row(result, 'environment.worker.API_KEY')?.cells?.[0].withheld, true);
    assert.ok(!Buffer.from(wire.AnalysisResponse.encode(result).finish()).toString('latin1').includes('SECRET_'));
  } finally { held.resolve(); await optional; await tasks.close(); await rm(root, { recursive: true, force: true }); }
});

test('the Console selects Review and live facts from the two independent modules', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'comparison-observations-')), tasks = runner();
  const a = await saved(root, 'a'.repeat(32), 'powersave'), b = await saved(root, 'b'.repeat(32), 'performance');
  const review = wire.AnalysisRun.create({ directory: b.directory, capture: b.capture,
    snapshotCurrent: { phase: wire.Phase.REVIEWED, runId: 'b'.repeat(32), profiles: ['fast'],
      systemApps: { ids: ['clock'], profiles: { clock: 'record' } }, writePolicy: filesystem.WritePolicy.WRITE_POLICY_DISCARD },
  });
  try {
    // Act
    const runFacts = await tasks.run(RUN_MODULE_ID, a, review, new AbortController().signal);
    const machineFacts = await tasks.run(MACHINE_MODULE_ID, a, review, new AbortController().signal);
    const prepared = reviewRows(true, runFacts.rows, machineFacts.rows);
    // Assert
    assert.equal(row(prepared, 'execution'), undefined);
    assert.equal(row(prepared, 'write_policy')?.result, wire.ComparisonResult.COMPARISON_RESULT_CHANGED);
    assert.deepEqual(row(prepared, 'system_apps')?.cells?.[1].values, ['clock']);
    assert.deepEqual(row(prepared, 'write_policy')?.cells?.[1].values, ['WRITE_POLICY_DISCARD']);
    assert.equal(row(prepared, 'load_1m')?.result, wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE);
    // Act
    const machine = await tasks.run(MACHINE_MODULE_ID, wire.AnalysisRun.create({ capture: a.capture }), wire.AnalysisRun.create({ capture: b.capture }), new AbortController().signal);
    // Assert
    assert.deepEqual(machineChangeRows(machine.rows).map(value => value.name), ['governor']);
    // Act
    const missing = await tasks.run(MACHINE_MODULE_ID, wire.AnalysisRun.create({ capture: a.capture }),
      wire.AnalysisRun.create({ capture: execution.RunCapture.create({ beforeError: 'Observation failed' }) }), new AbortController().signal);
    // Assert
    assert.equal(missing.rows[0].name, 'machine_capture_error');
    assert.deepEqual(row(missing, 'machine_capture_error')?.cells?.[1].values, ['Observation failed']);
  } finally { await tasks.close(); await rm(root, { recursive: true, force: true }); }
});

test('resolved Review inputs reject incompatible snapshots before analysis', async () => {
  // Arrange
  const tasks = runner(), signal = new AbortController().signal;
  const invalid = wire.AnalysisRun.create({ directory: '/unavailable', snapshotCurrent: { phase: wire.Phase.RUNNING, runId: 'a'.repeat(32) } });
  try {
    // Act / Assert
    await assert.rejects(tasks.run(RUN_MODULE_ID, wire.AnalysisRun.create(), invalid, signal), /InvalidInput/);
  } finally { await tasks.close(); }
});

const side = (id: string): Side => ({ id, setup: null, capture: execution.RunCapture.create(), detail: wire.HistoryDetail.create() });

test('large raw facts retain Review capacity and oversized optional comparisons preserve their envelope', () => {
  // Arrange
  const environment = Object.fromEntries(Array.from({ length: 2000 }, (_, index) => ['VALUE_' + index, 'x'.repeat(512)]));
  const baseline = { ...side('a'), setup: compose.SetupRecord.fromObject({ services: { worker: { environment } } }) };
  const comparison = compareReview(baseline, { ...baseline, profiles: [] });
  const review = wire.Review.create({ preparedId: 'review', comparison });
  // Act / Assert: raw facts fit the ordinary encoded envelope.
  assert.deepEqual(comparison.rows.find(row => row.name === 'environment.worker.VALUE_0')?.cells?.[0]?.values, ['x'.repeat(512)]);
  assert.ok(wire.Review.encode(review).finish().length <= LIMITS.frameBytes);
  assert.ok(wire.Status.encode({ machineChanges: comparison.rows }).finish().length <= LIMITS.frameBytes);
  // Act: UTF-8 values can exhaust the real encoded envelope.
  for (const row of comparison.rows) for (const cell of row.cells ?? []) cell.values = ['界'.repeat(512)];
  assert.ok(wire.Review.encode(review).finish().length > LIMITS.frameBytes);
  const oversizedRows = comparison.rows;
  boundReviewComparison(review);
  // Assert
  assert.equal(review.preparedId, 'review');
  assert.equal(review.comparison?.truncated, true);
  assert.equal(review.comparison?.rows?.length, 0);
  assert.ok(wire.Review.encode(review).finish().length <= LIMITS.frameBytes);
  const status = wire.Status.create({ phase: wire.Phase.FINISHED, outcome: { execution: 'Completed' }, machineChanges: oversizedRows });
  boundMachineChanges(status);
  assert.equal(status.outcome?.execution, 'Completed');
  assert.equal(status.machineChanges[0].name, 'machine_capture_error');
  assert.match(status.machineChanges[0].cells![1].values![0], /too large/);
  assert.ok(wire.LiveMessage.encode({ status: wire.Status.encode(status).finish() }).finish().length <= LIMITS.frameBytes);
});

test('legacy machine comparison rows decode with identical field tags', () => {
  // Arrange: independently encoded Status.machine_changes field 25 with the old row shape.
  const bytes = Buffer.from('ca011008031205756e6974731a030a01312002', 'hex');
  // Act
  const status = wire.Status.decode(bytes);
  // Assert
  assert.equal(status.machineChanges[0].name, 'units');
  assert.deepEqual(status.machineChanges[0].cells?.[0].values, ['1']);
  assert.equal(status.machineChanges[0].cells?.[0].withheld, false);
  assert.deepEqual(Buffer.from(wire.Status.encode(status).finish()), bytes);
});
