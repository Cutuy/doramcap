import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { analysis, execution, wire } from '../src/native.js';
import { createAnalysisTask as registerAnalysis } from '../src/analysis/module.js';
import { AnalysisRegistry } from '../src/analysis/registry.js';
import { AnalysisRunner } from '../src/analysis/runner.js';
import { comparisonResponse, diffFacts, compareReview, boundReviewComparison, boundMachineChanges, type Side } from '../src/records/compare.js';
import { LIMITS } from '../src/generated/transport.js';

const side = (id: string): Side => ({ id, manifest: null, capture: execution.RunCapture.create(), detail: wire.HistoryDetail.create() });
const pair = [analysis.AnalysisRun.create({ directory: '/runs/a', detail: side('a').detail, capture: side('a').capture }), analysis.AnalysisRun.create({ directory: '/runs/b', detail: side('b').detail, capture: side('b').capture })] as const;
const gate = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };

test('one stalled module does not hold another module response and a failed task can be invoked again', async () => {
  // Arrange
  const held = gate(); let attempts = 0;
  const slow = registerAnalysis({ id: 'slow', label: 'Slow', analyze: async () => { await held.promise; return 1; }, compare: () => [] });
  const independent = registerAnalysis({ id: 'independent', label: 'Independent', analyze: async () => { if (++attempts === 1) throw new Error('source failed'); return 1; }, compare: () => [] });
  let completed = false;
  const pending = slow.run(...pair, new AbortController().signal).then(() => { completed = true; });
  try {
    // Act / Assert
    await assert.rejects(independent.run(...pair, new AbortController().signal), /source failed/);
    const response = await independent.run(...pair, new AbortController().signal);
    assert.equal(response.moduleId, 'independent');
    assert.equal(completed, false);
  } finally { held.resolve(); await pending; }
});

test('cancelling a task waits for the active analyzer cleanup and never starts the other side', async () => {
  // Arrange
  const entered = gate(), cleanup = gate(), controller = new AbortController();
  let readers = 0, starts = 0;
  const module = registerAnalysis({ id: 'owned', label: 'Owned',
    analyze: async (_run, signal) => {
      starts++; readers++; entered.resolve();
      try { await new Promise<void>(resolve => signal.addEventListener('abort', () => resolve(), { once: true })); signal.throwIfAborted(); }
      finally { await cleanup.promise; readers--; }
      return 1;
    }, compare: () => { throw new Error('must not compare'); },
  });
  let settled = false;
  const task = module.run(...pair, controller.signal).finally(() => { settled = true; });
  const rejected = assert.rejects(task, { name: 'AbortError' });
  await entered.promise;
  // Act
  controller.abort();
  await Promise.resolve();
  // Assert
  assert.equal(readers, 1);
  assert.equal(settled, false);
  cleanup.resolve(); await rejected;
  assert.equal(readers, 0);
  assert.equal(starts, 1);
});

test('pre-cancelled tasks and cancellation between analyzer and comparison do no later work', async () => {
  // Arrange
  const controller = new AbortController(); let starts = 0, compares = 0;
  const module = registerAnalysis({ id: 'checkpoints', label: 'Checkpoints', analyze: async () => { starts++; if (starts === 2) controller.abort(); return 1; }, compare: () => { compares++; return []; } });
  const cancelled = new AbortController(); cancelled.abort();
  // Act / Assert
  await assert.rejects(module.run(...pair, cancelled.signal), { name: 'AbortError' });
  assert.equal(starts, 0);
  await assert.rejects(module.run(...pair, controller.signal), { name: 'AbortError' });
  assert.equal(starts, 2);
  assert.equal(compares, 0);
});

test('legacy protobuf rows retain raw values and additive presentation defaults', () => {
  // Arrange: independently encoded group=3, name=units, values=[1], result=changed.
  const bytes = Buffer.from('08031205756e6974731a030a01312002', 'hex');
  // Act
  const row = wire.ComparisonRow.decode(bytes);
  // Assert
  assert.equal(row.name, 'units');
  assert.deepEqual(row.cells[0].values, ['1']);
  assert.equal(row.label, '');
  assert.deepEqual(row.cells[0].displayValues, []);
});

test('comparison transport accepts its byte boundary and rejects oversized UTF-8 presentation explicitly', () => {
  // Arrange
  const codec = { encode: (bytes: Uint8Array) => ({ finish: () => bytes }) };
  const facts = new Map(Array.from({ length: 2000 }, (_, index) => [String(index), { value: '界'.repeat(512) }]));
  const rows = diffFacts(wire.ComparisonGroup.COMPARISON_SETUP, facts, facts);
  // Act / Assert
  assert.equal(comparisonResponse(codec, new Uint8Array(LIMITS.frameBytes)).length, LIMITS.frameBytes);
  assert.throws(() => comparisonResponse(codec, new Uint8Array(LIMITS.frameBytes + 1)), error => /response limit/.test((error as { detail: string }).detail));
  assert.throws(() => comparisonResponse(wire.HistoryComparison, { rows }), error => /response limit/.test((error as { detail: string }).detail));
});


test('large display-ready facts retain Review capacity and oversized optional comparisons preserve their envelope', () => {
  // Arrange
  const environment = Object.fromEntries(Array.from({ length: 2000 }, (_, index) => ['VALUE_' + index, 'x'.repeat(512)]));
  const baseline = { ...side('a'), manifest: { services: { worker: { environment } } } };
  const comparison = compareReview(baseline, { ...baseline, profiles: [] });
  const review = wire.Review.create({ preparedId: 'review', comparison });
  // Act / Assert: ordinary display-ready text has no second copy in its cell.
  assert.equal(comparison.rows.find(row => row.name === 'environment.worker.VALUE_0')?.cells?.[0]?.displayValues?.length, 0);
  assert.ok(wire.Review.encode(review).finish().length <= LIMITS.frameBytes);
  assert.ok(wire.Status.encode({ machineChanges: comparison.rows }).finish().length <= LIMITS.frameBytes);
  // Act: UTF-8 values and long labels can exhaust the real encoded envelope.
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


test('an empty registry rejects unknown modules and duplicate registrations', async () => {
  // Arrange
  const registry = new AnalysisRegistry(), runner = new AnalysisRunner(registry);
  assert.deepEqual(registry.list(), []);
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
