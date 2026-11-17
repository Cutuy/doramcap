import assert from 'node:assert/strict';
import { test } from 'node:test';
import { wire } from './api';
import { presentCpuRows } from './analysis/modules/cpu/presenter';

const { ComparisonResult: Result } = wire;
const fact = (name: string, values: string[], result = Result.COMPARISON_RESULT_CHANGED) => wire.ComparisonFact.fromObject({
  group: wire.ComparisonGroup.COMPARISON_GROUP_OUTCOME, name, result, cells: values.map(value => ({ values: [value], evidence: 'Measured interval' })),
});

test('CPU presentation supplies labels and units without modifying exact measurements', () => {
  // Arrange
  const rows = [fact('service.worker.cpu_seconds', ['1.000000000', '2.000000000']),
    fact('service.worker.cpu_mean_percent', ['100000000000/1000000000', '200000000000/1000000000'])];
  const original = rows.map(row => wire.ComparisonFact.encode(row).finish());
  // Act
  const presented = presentCpuRows(rows);
  // Assert
  assert.equal(presented[0].label, 'worker · Measured CPU time');
  assert.deepEqual(presented[0].cells.map(cell => cell.values), [['1.000000000 s'], ['2.000000000 s']]);
  assert.deepEqual(presented[1].cells.map(cell => cell.values), [['100%'], ['200%']]);
  assert.deepEqual(rows.map(row => wire.ComparisonFact.encode(row).finish()), original);
});

test('CPU presentation exposes exact ratios when different percentages round to the same label', () => {
  // Arrange
  const divisor = 1n << 60n;
  const values = [divisor * 100n + '/' + divisor, (divisor + 1n) * 100n + '/' + divisor];
  const row = fact('service.worker.cpu_mean_percent', values);
  // Act
  const presented = presentCpuRows([row])[0];
  // Assert
  assert.deepEqual(presented.cells.map(cell => cell.values[0]), values.map(value => value + ' %'));
  // Act / Assert: ratios that are equal retain their readable label.
  row.result = Result.COMPARISON_RESULT_SAME;
  assert.deepEqual(presentCpuRows([row])[0].cells.map(cell => cell.values[0]), ['100%', '100%']);
});

test('CPU presentation keeps coverage visible and withheld values and evidence private', () => {
  // Arrange
  const row = fact('service.worker.cpu_coverage', ['SECRET_A', 'SECRET_B'], Result.COMPARISON_RESULT_SAME);
  row.cells[0].withheld = true;
  // Act
  const presented = presentCpuRows([row])[0];
  // Assert
  assert.equal(presented.alwaysVisible, true);
  assert.equal(presented.hidden, true);
  assert.deepEqual(presented.cells, [{ values: [], evidence: '' }, { values: [], evidence: '' }]);
});
