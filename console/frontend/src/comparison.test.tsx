import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { MachineChanges, AnalysisSection } from './Comparison';
import { factLabel, comparedFactValue } from './generated/presentation';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const condition = (name: string, values: string[], fields: object = {}) => wire.ComparisonRow.fromObject({
  group: Group.COMPARISON_CONDITION, name, label: factLabel(name), result: Result.COMPARISON_CHANGED,
  cells: values.map(value => ({ values: [value], displayValues: [comparedFactValue(name, value, values)] })), ...fields,
});

test('Result shows the exact difference when byte and uptime labels would match', () => {
  // Arrange
  const rows = [condition('disk_free_bytes', ['22763326668', '22763322572']), condition('uptime_seconds', ['1440', '1441'])];
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={rows}/>);
  // Assert
  assert.match(html, /Before<\/span>22763326668 B/);
  assert.match(html, /After<\/span>22763322572 B/);
  assert.match(html, /Before<\/span>1440 s/);
  assert.match(html, /After<\/span>1441 s/);
  assert.match(html, /2 changed/);
  assert.equal((html.match(/class="comparison-result">Changed/g) ?? []).length, 2);
  assert.doesNotMatch(html, /21\.2 GiB|0\.4 hours/);
});

test('Result preserves readable equal values and changes already visible at normal precision', () => {
  // Arrange
  const rows = [
    condition('disk_free_bytes', ['1048576', '2097152']),
    condition('uptime_seconds', ['1440', '1440'], { result: Result.COMPARISON_SAME }),
  ];
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={rows}/>);
  // Assert
  assert.match(html, /Before<\/span>1\.0 MiB/);
  assert.match(html, /After<\/span>2\.0 MiB/);
  assert.equal((html.match(/0\.4 hours/g) ?? []).length, 2);
  assert.match(html, /comparison-result">Same/);
  assert.match(html, /1 changed/);
});

test('Result keeps hidden and missing facts private and explicit', () => {
  // Arrange
  const rows = [
    condition('disk_free_bytes', ['22763326668', '22763322572'], { hidden: true }),
    condition('uptime_seconds', ['', '1440']),
    condition('memory_available_bytes', ['', '1048576'], { result: Result.COMPARISON_UNAVAILABLE }),
  ];
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={rows}/>);
  // Assert
  assert.equal((html.match(/<\/span>Hidden/g) ?? []).length, 2);
  assert.doesNotMatch(html, /22763326668|22763322572|21\.2 GiB/);
  assert.match(html, /Before<\/span>Not set/);
  assert.match(html, /After<\/span>0\.4 hours/);
  assert.match(html, /Before<\/span>Unavailable/);
  assert.match(html, /After<\/span>1\.0 MiB/);
});


test('shared rows render supplied labels, display values and per-side evidence without key interpretation', () => {
  // Arrange
  const row = wire.ComparisonRow.fromObject({ group: Group.COMPARISON_CONDITION, name: 'opaque.key', label: 'Measured CPU time',
    result: Result.COMPARISON_UNAVAILABLE, cells: [{ values: ['1'], displayValues: ['1.000000000 s'], evidence: 'Partial: one interval missing' },
      { values: [''], displayValues: ['Unavailable'], evidence: 'Unavailable: service was not recorded' }] });
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={[row]}/>);
  const hidden = renderToStaticMarkup(<MachineChanges rows={[wire.ComparisonRow.create({ ...row, hidden: true })]}/>);
  // Assert
  assert.match(html, /Measured CPU time/);
  assert.match(html, /1.000000000 s/);
  assert.match(html, /Partial: one interval missing/);
  assert.match(html, /Unavailable: service was not recorded/);
  assert.doesNotMatch(html, /opaque.key/);
  assert.doesNotMatch(hidden, /1.000000000|Partial: one interval missing|service was not recorded/);
});
