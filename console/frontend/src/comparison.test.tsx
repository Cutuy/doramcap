import { factLabel, comparedFactValue } from './generated/materialized-presentation';
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { MachineChanges } from './Comparison';

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

test('rows display producer labels, values and evidence while legacy rows retain raw facts', () => {
  // Arrange
  const supplied = wire.ComparisonRow.fromObject({ group: Group.COMPARISON_CONDITION, name: 'disk_free_bytes', label: 'Producer storage',
    result: Result.COMPARISON_CHANGED, cells: [{ values: ['1048576'], displayValues: ['Producer value'], evidence: 'Selected folder' }, { values: ['2097152'], displayValues: ['Other value'] }],
  });
  const legacy = wire.ComparisonRow.fromObject({ group: Group.COMPARISON_CONDITION, name: 'legacy_bytes', result: Result.COMPARISON_CHANGED,
    cells: [{ values: ['12345'] }, { values: ['67890'] }],
  });
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={[supplied, legacy]}/>);
  // Assert
  assert.match(html, /Producer storage/);
  assert.match(html, /Producer value/);
  assert.match(html, /Selected folder/);
  assert.match(html, /legacy_bytes/);
  assert.match(html, /12345/);
  assert.doesNotMatch(html, /1\.0 MiB|Disk free|Legacy/);
});

test('hidden rows with supplied presentation never render values or evidence', () => {
  // Arrange
  const row = wire.ComparisonRow.fromObject({ group: Group.COMPARISON_CONDITION, name: 'private_bytes', hidden: true,
    result: Result.COMPARISON_CHANGED, cells: [{ values: ['SECRET_RAW'], displayValues: ['SECRET_DISPLAY'], evidence: 'SECRET_EVIDENCE' }, { values: [] }],
  });
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={[row]}/>);
  // Assert
  assert.match(html, /Hidden/);
  assert.doesNotMatch(html, /SECRET_/);
});
