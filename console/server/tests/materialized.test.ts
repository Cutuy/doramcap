import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { compose, execution, wire } from '../src/native.js';
import { filesystem } from '../src/records/file-changes.js';
import { readSetupRecord, STOP_GRACE_PERIOD_VERSION } from '../src/records/setup.js';
import { AnalysisRegistry, analysisRegistry } from '../src/analysis/registry.js';
import { AnalysisRunner } from '../src/analysis/runner.js';
import { materializedModule, MATERIALIZED_MODULE_ID } from '../src/analysis/modules/materialized/module.js';
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
const row = (result: typeof wire.AnalysisResponse.prototype, name: string) => result.rows.find(value => value.name === name);
const runner = () => { const registry = new AnalysisRegistry(); registry.register(materializedModule); return new AnalysisRunner(registry); };

test('the production registry contains the default materialized module', () => {
  // Arrange / Act
  const modules = analysisRegistry.list();
  // Assert
  assert.deepEqual(modules.map(value => value.id), [MATERIALIZED_MODULE_ID]);
});

test('the default module normalizes original saved evidence and returns while an optional module is stalled', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'materialized-runs-'));
  const a = await saved(root, 'a'.repeat(32), 'powersave'), b = await saved(root, 'b'.repeat(32), 'performance');
  const held = gate(), entered = gate(), registry = new AnalysisRegistry();
  registry.register(materializedModule);
  registry.register({ id: 'held', label: 'Held', analyze: async () => { entered.resolve(); await held.promise; return 1; }, compare: () => [] });
  const tasks = new AnalysisRunner(registry);
  let optionalDone = false;
  const optional = tasks.run('held', a, b, new AbortController().signal).then(() => { optionalDone = true; });
  try {
    await entered.promise;
    // Act
    const result = await tasks.run(MATERIALIZED_MODULE_ID, a, b, new AbortController().signal);
    // Assert
    assert.equal(optionalDone, false);
    assert.equal(row(result, 'governor')?.result, wire.ComparisonResult.COMPARISON_CHANGED);
    assert.equal(row(result, 'environment.worker.OUTPUT')?.result, wire.ComparisonResult.COMPARISON_SAME);
    assert.equal(row(result, 'environment.worker.OUTPUT')?.cells?.[0].values?.[0], '/data/runs/@run/output');
    assert.equal(readSetupRecord(root, 'a'.repeat(32))?.services.worker.environment?.OUTPUT, `/data/runs/${'a'.repeat(32)}/output`);
    assert.equal(row(result, 'environment.worker.API_KEY')?.hidden, true);
    assert.ok(!Buffer.from(wire.AnalysisResponse.encode(result).finish()).toString('latin1').includes('SECRET_'));
  } finally { held.resolve(); await optional; await tasks.close(); await rm(root, { recursive: true, force: true }); }
});

test('the same registered module compares resolved Review settings and live machine observations', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'materialized-observations-')), tasks = runner();
  const a = await saved(root, 'a'.repeat(32), 'powersave'), b = await saved(root, 'b'.repeat(32), 'performance');
  const review = wire.AnalysisRun.create({ directory: b.directory, capture: b.capture,
    snapshotCurrent: { phase: wire.Phase.REVIEWED, runId: 'b'.repeat(32), profiles: ['fast'],
      systemApps: { ids: ['clock'], profiles: { clock: 'record' } }, writePolicy: filesystem.WritePolicy.WRITE_POLICY_DISCARD },
  });
  try {
    // Act
    const prepared = await tasks.run(MATERIALIZED_MODULE_ID, a, review, new AbortController().signal);
    // Assert
    assert.equal(row(prepared, 'execution'), undefined);
    assert.equal(row(prepared, 'write_policy')?.result, wire.ComparisonResult.COMPARISON_CHANGED);
    assert.deepEqual(row(prepared, 'system_apps')?.cells?.[1].values, ['clock']);
    assert.deepEqual(row(prepared, 'write_policy')?.cells?.[1].displayValues, ['Discard']);
    assert.equal(row(prepared, 'load_1m')?.result, wire.ComparisonResult.COMPARISON_UNAVAILABLE);
    // Act
    const machine = await tasks.run(MATERIALIZED_MODULE_ID, wire.AnalysisRun.create({ capture: a.capture }), wire.AnalysisRun.create({ capture: b.capture }), new AbortController().signal);
    // Assert
    assert.deepEqual(machine.rows.map(value => value.name), ['governor']);
    // Act
    const missing = await tasks.run(MATERIALIZED_MODULE_ID, wire.AnalysisRun.create({ capture: a.capture }),
      wire.AnalysisRun.create({ capture: execution.RunCapture.create({ beforeError: 'Observation failed' }) }), new AbortController().signal);
    // Assert
    assert.equal(row(missing, 'machine_capture_error')?.alwaysVisible, true);
    assert.deepEqual(row(missing, 'machine_capture_error')?.cells?.[1].values, ['Observation failed']);
  } finally { await tasks.close(); await rm(root, { recursive: true, force: true }); }
});

test('resolved Review inputs reject incompatible snapshots before analysis', async () => {
  // Arrange
  const tasks = runner(), signal = new AbortController().signal;
  const invalid = wire.AnalysisRun.create({ directory: '/unavailable', snapshotCurrent: { phase: wire.Phase.RUNNING, runId: 'a'.repeat(32) } });
  try {
    // Act / Assert
    await assert.rejects(tasks.run(MATERIALIZED_MODULE_ID, wire.AnalysisRun.create(), invalid, signal), /InvalidInput/);
  } finally { await tasks.close(); }
});
