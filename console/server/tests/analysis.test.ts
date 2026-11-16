import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { wire } from '../src/native.js';
import type { AnalysisModule } from '../src/analysis/module.js';
import { AnalysisRegistry, analysisRegistry } from '../src/analysis/registry.js';
import { AnalysisRunner } from '../src/analysis/runner.js';
import { boundRows, comparisonResponse } from '../src/analysis/rows.js';
import { LIMITS } from '../src/generated/transport.js';
import '../src/analysis/modules/index.js';

const pair = [wire.AnalysisRun.create({ directory: '/runs/a' }), wire.AnalysisRun.create({ directory: '/runs/b' })] as const;
const gate = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };

function runnerFor<Analysis>(module: AnalysisModule<Analysis>) {
  const registry = new AnalysisRegistry();
  registry.register(module);
  return new AnalysisRunner(registry);
}

test('one stalled module does not hold another module response and a failed task can be invoked again', async () => {
  // Arrange
  const held = gate(), registry = new AnalysisRegistry(); let attempts = 0;
  registry.register({ id: 'slow', label: 'Slow', analyze: async () => { await held.promise; return 1; }, compare: () => [] });
  registry.register({ id: 'independent', label: 'Independent', analyze: async () => { if (++attempts === 1) throw new Error('source failed'); return 1; }, compare: () => [] });
  const runner = new AnalysisRunner(registry);
  let completed = false;
  const pending = runner.run('slow', ...pair, new AbortController().signal).then(() => { completed = true; });
  try {
    // Act / Assert
    await assert.rejects(runner.run('independent', ...pair, new AbortController().signal), /source failed/);
    const response = await runner.run('independent', ...pair, new AbortController().signal);
    assert.equal(response.moduleId, 'independent');
    assert.equal(completed, false);
  } finally { held.resolve(); await pending; await runner.close(); }
});

test('both sides start together and comparison preserves A/B order when B finishes first', { timeout: 5000 }, async () => {
  // Arrange
  const entered = pair.map(() => gate()), release = pair.map(() => gate()), finished = pair.map(() => gate());
  const runner = runnerFor({ id: 'parallel', label: 'Parallel',
    analyze: async (run) => {
      const index = run.directory === pair[0].directory ? 0 : 1;
      entered[index].resolve();
      await release[index].promise;
      finished[index].resolve();
      return run.directory;
    },
    compare: (left, right) => [wire.ComparisonFact.create({ name: 'order', cells: [{ values: [left] }, { values: [right] }] })],
  });
  let settled = false;
  // Act
  const pending = runner.run('parallel', ...pair, new AbortController().signal).finally(() => { settled = true; });
  try {
    await Promise.all(entered.map(value => value.promise));
    release[1].resolve(); await finished[1].promise;
    // Assert: B can finish while A is still active.
    assert.equal(settled, false);
    // Act
    release[0].resolve();
    const response = await pending;
    // Assert
    assert.deepEqual(response.rows[0].cells!.map(cell => cell.values), [['/runs/a'], ['/runs/b']]);
  } finally { release.forEach(value => value.resolve()); await pending; await runner.close(); }
});

test('cancelling a task waits for both analyzers to clean up before returning', { timeout: 5000 }, async () => {
  // Arrange
  const entered = pair.map(() => gate()), cleaning = pair.map(() => gate()), cleanup = pair.map(() => gate()), finished = pair.map(() => gate());
  const controller = new AbortController(); let readers = 0, compares = 0;
  const runner = runnerFor({ id: 'owned', label: 'Owned',
    analyze: async (run, signal) => {
      const index = run.directory === pair[0].directory ? 0 : 1;
      readers++; entered[index].resolve();
      try { await new Promise<void>(resolve => signal.addEventListener('abort', () => resolve(), { once: true })); signal.throwIfAborted(); }
      finally { cleaning[index].resolve(); await cleanup[index].promise; readers--; finished[index].resolve(); }
      return 1;
    }, compare: () => { compares++; return []; },
  });
  let settled = false;
  const task = runner.run('owned', ...pair, controller.signal).finally(() => { settled = true; });
  const rejected = assert.rejects(task, { name: 'AbortError' });
  try {
    await Promise.all(entered.map(value => value.promise));
    // Act
    controller.abort();
    await Promise.all(cleaning.map(value => value.promise));
    // Assert
    assert.equal(readers, 2);
    assert.equal(settled, false);
    // Act / Assert: one completed cleanup still cannot release the task.
    cleanup[0].resolve(); await finished[0].promise;
    assert.equal(readers, 1);
    assert.equal(settled, false);
    cleanup[1].resolve(); await rejected;
    assert.equal(readers, 0);
    assert.equal(compares, 0);
  } finally { controller.abort(); cleanup.forEach(value => value.resolve()); await rejected; await runner.close(); }
});

for (const failedSide of [0, 1]) test(`side ${failedSide === 0 ? 'A' : 'B'} failure cancels its sibling and waits for cleanup`, { timeout: 5000 }, async () => {
  // Arrange
  const entered = gate(), fail = gate(), cleaning = gate(), cleanup = gate();
  const failure = new Error('source failed'); let settled = false, compares = 0;
  const runner = runnerFor({ id: 'failure', label: 'Failure',
    analyze: async (run, signal) => {
      if (run.directory === pair[failedSide].directory) { await fail.promise; throw failure; }
      entered.resolve();
      try { await new Promise<void>(resolve => signal.addEventListener('abort', () => resolve(), { once: true })); signal.throwIfAborted(); }
      finally { cleaning.resolve(); await cleanup.promise; }
      return 1;
    }, compare: () => { compares++; return []; },
  });
  const pending = runner.run('failure', ...pair, new AbortController().signal).finally(() => { settled = true; });
  const rejected = assert.rejects(pending, error => error === failure);
  try {
    await entered.promise;
    // Act
    fail.resolve(); await cleaning.promise;
    // Assert
    assert.equal(settled, false);
    assert.equal(compares, 0);
    // Act / Assert
    cleanup.resolve(); await rejected;
    assert.equal(compares, 0);
  } finally { fail.resolve(); cleanup.resolve(); await rejected; await runner.close(); }
});

test('pre-cancelled tasks and cancellation between analyzer and comparison do no later work', async () => {
  // Arrange
  const controller = new AbortController(); let starts = 0, compares = 0;
  const runner = runnerFor({ id: 'checkpoints', label: 'Checkpoints', analyze: async () => { starts++; if (starts === 2) controller.abort(); return 1; }, compare: () => { compares++; return []; } });
  const cancelled = new AbortController(); cancelled.abort();
  try {
    // Act / Assert
    await assert.rejects(runner.run('checkpoints', ...pair, cancelled.signal), { name: 'AbortError' });
    assert.equal(starts, 0);
    await assert.rejects(runner.run('checkpoints', ...pair, controller.signal), { name: 'AbortError' });
    assert.equal(starts, 2);
    assert.equal(compares, 0);
  } finally { await runner.close(); }
});

test('saved-run inputs retain their evidence and wire bytes after the snapshot field rename', () => {
  // Arrange: input encoded with the original doramcap.console.analysis.v1 schema.
  const bytes = Buffer.from('0a072f72756e732f6112160a030a0161120f420d0a09436f6d706c6574656418011a0a0a080a06636f6d6d697422260a0d636f6e66696775726174696f6e12156578706572696d656e742e696e7075742e6a736f6e', 'hex');
  // Act
  const run = wire.AnalysisRun.decode(bytes);
  // Assert
  assert.equal(run.directory, '/runs/a');
  assert.equal(run.snapshot, 'snapshotHistory');
  assert.equal(run.snapshotHistory!.entry!.runId, 'a');
  assert.equal(run.snapshotHistory!.status!.outcome!.execution, 'Completed');
  assert.equal(run.snapshotHistory!.status!.outcome!.recordingFinalized, true);
  assert.equal(run.capture!.setup!.sourceCommit, 'commit');
  assert.deepEqual(run.artifacts.map(({ name, path }) => ({ name, path })), [{ name: 'configuration', path: 'experiment.input.json' }]);
  assert.deepEqual(Buffer.from(wire.AnalysisRun.encode(run).finish()), bytes);
});

test('existing comparison rows preserve their raw values after common enum relocation', () => {
  // Arrange: independently encoded group=3, name=units, values=[1], result=changed.
  const bytes = Buffer.from('08031205756e6974731a030a01312002', 'hex');
  // Act
  const row = wire.ComparisonFact.decode(bytes);
  // Assert
  assert.equal(row.name, 'units');
  assert.deepEqual(row.cells[0].values, ['1']);
  assert.equal(row.cells[0].withheld, false);
});

test('comparison transport accepts its byte boundary and rejects oversized UTF-8 facts explicitly', () => {
  // Arrange
  const codec = { encode: (bytes: Uint8Array) => ({ finish: () => bytes }) };
  const rows = Array.from({ length: 2000 }, (_, index) => wire.ComparisonFact.create({ name: String(index), cells: [{ values: ['界'.repeat(512)] }, { values: ['界'.repeat(512)] }] }));
  // Act / Assert
  assert.equal(comparisonResponse(codec, new Uint8Array(LIMITS.frameBytes)).length, LIMITS.frameBytes);
  assert.throws(() => comparisonResponse(codec, new Uint8Array(LIMITS.frameBytes + 1)), error => /response limit/.test((error as { detail: string }).detail));
  assert.throws(() => comparisonResponse(wire.AnalysisResponse, { rows }), error => /response limit/.test((error as { detail: string }).detail));
});


test('an empty registry rejects unknown modules and duplicate registrations', async () => {
  // Arrange
  const registry = new AnalysisRegistry(), runner = new AnalysisRunner(registry);
  assert.deepEqual(registry.list(), []);
  assert.deepEqual(analysisRegistry.list().map(module => module.id), ['run', 'machine']);
  // Act / Assert
  await assert.rejects(runner.run('missing', ...pair, new AbortController().signal), /InvalidInput/);
  const module = { id: 'fixture', label: 'Fixture', analyze: async () => 1, compare: () => [] };
  registry.register(module);
  assert.deepEqual(registry.list().map(value => value.id), ['fixture']);
  assert.throws(() => registry.register(module), /Duplicate/);
  await runner.close();
});

test('runner shutdown cancels active work and awaits cleanup before rejecting new work', async () => {
  // Arrange
  const registry = new AnalysisRegistry(), entered = gate(), cleanup = gate();
  registry.register({ id: 'owned', label: 'Owned', analyze: async (_run, signal) => {
    entered.resolve();
    try { await new Promise<void>(resolve => signal.addEventListener('abort', () => resolve(), { once: true })); signal.throwIfAborted(); }
    finally { await cleanup.promise; }
    return 1;
  }, compare: () => [] });
  const runner = new AnalysisRunner(registry);
  const task = runner.run('owned', ...pair, new AbortController().signal);
  const rejected = assert.rejects(task, { name: 'AbortError' });
  await entered.promise;
  // Act
  let closed = false;
  const closing = runner.close().then(() => { closed = true; });
  // Assert
  await Promise.resolve();
  assert.equal(closed, false);
  await assert.rejects(runner.run('owned', ...pair, new AbortController().signal), /Closing/);
  cleanup.resolve();
  await rejected; await closing;
});

test('the service row budget accepts its boundary and keeps nonmatching facts in module order', () => {
  // Arrange
  const matching = Array.from({ length: 2000 }, (_, index) => wire.ComparisonFact.create({ name: String(index), result: wire.ComparisonResult.COMPARISON_RESULT_SAME }));
  const missing = wire.ComparisonFact.create({ name: 'missing-evidence', result: wire.ComparisonResult.COMPARISON_RESULT_UNAVAILABLE });
  const changed = wire.ComparisonFact.create({ name: 'changed', result: wire.ComparisonResult.COMPARISON_RESULT_CHANGED });
  // Act / Assert
  assert.deepEqual(boundRows(matching), { rows: matching, truncated: false });
  const result = boundRows([...matching, missing, changed]);
  assert.equal(result.truncated, true);
  assert.equal(result.rows.length, 2000);
  assert.deepEqual(result.rows.slice(0, 2).map(row => row.name), ['missing-evidence', 'changed']);
});

test('runner completion leaves workflow selection and transport bounds to the service', async () => {
  // Arrange
  const rows = Array.from({ length: 2001 }, (_, index) => wire.ComparisonFact.create({ name: String(index) }));
  const runner = runnerFor({ id: 'complete', label: 'Complete', analyze: async () => 1, compare: () => rows });
  try {
    // Act
    const result = await runner.run('complete', ...pair, new AbortController().signal);
    // Assert
    assert.equal(result.rows.length, rows.length);
    assert.equal(result.truncated, false);
    const transported = boundRows(result.rows);
    assert.equal(transported.rows.length, 2000);
    assert.equal(transported.truncated, true);
  } finally { await runner.close(); }
});
