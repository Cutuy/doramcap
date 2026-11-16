import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { wire } from './api';
import { MachineChanges } from './Comparison';

const { ComparisonGroup: Group, ComparisonResult: Result } = wire;
const condition = (name: string, values: string[], fields: object = {}) => wire.ComparisonFact.fromObject({
  group: Group.COMPARISON_GROUP_CONDITION, name, result: Result.COMPARISON_RESULT_CHANGED,
  cells: values.map(value => ({ values: [value] })), ...fields,
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
    condition('uptime_seconds', ['1440', '1440'], { result: Result.COMPARISON_RESULT_SAME }),
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
    condition('disk_free_bytes', ['', ''], { cells: [{ values: [], withheld: true }, { values: [], withheld: true }] }),
    condition('uptime_seconds', ['', '1440']),
    condition('memory_available_bytes', ['', '1048576'], { result: Result.COMPARISON_RESULT_UNAVAILABLE }),
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

test('the module presenter formats raw protobuf facts and preserves their evidence', () => {
  // Arrange
  const supplied = wire.ComparisonFact.fromObject({ group: Group.COMPARISON_GROUP_CONDITION, name: 'disk_free_bytes',
    result: Result.COMPARISON_RESULT_CHANGED, cells: [{ values: ['1048576'], evidence: 'Selected folder' }, { values: ['2097152'] }],
  });
  const legacy = wire.ComparisonFact.fromObject({ group: Group.COMPARISON_GROUP_CONDITION, name: 'external_fact', result: Result.COMPARISON_RESULT_CHANGED,
    cells: [{ values: ['alpha'] }, { values: ['beta'] }],
  });
  const original = wire.ComparisonFact.encode(supplied).finish();
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={[supplied, legacy]}/>);
  // Assert
  assert.match(html, /Disk free/);
  assert.match(html, /1\.0 MiB/);
  assert.match(html, /Selected folder/);
  assert.match(html, /External fact/);
  assert.match(html, /alpha/);
  assert.deepEqual(wire.ComparisonFact.encode(supplied).finish(), original);
});

test('withheld values never reach the view model, rendered values or evidence', () => {
  // Arrange
  const row = wire.ComparisonFact.fromObject({ group: Group.COMPARISON_GROUP_CONDITION, name: 'private_bytes',
    result: Result.COMPARISON_RESULT_CHANGED, cells: [{ values: ['SECRET_RAW'], withheld: true, evidence: 'SECRET_EVIDENCE' }, { values: [] }],
  });
  // Act
  const html = renderToStaticMarkup(<MachineChanges rows={[row]}/>);
  // Assert
  assert.match(html, /Hidden/);
  assert.doesNotMatch(html, /SECRET_/);
});

test('shared renderer consumes a TS view without interpreting fact keys', async () => {
  // Arrange
  const { ComparisonRows } = await import('./Comparison');
  const row = { group: Group.COMPARISON_CONDITION, name: 'opaque.key', label: 'Measured CPU time',
    result: Result.COMPARISON_UNAVAILABLE, hidden: false, alwaysVisible: true,
    cells: [{ values: ['1.000000000 s'], evidence: 'Partial: one interval missing' },
      { values: ['Unavailable'], evidence: 'Unavailable: service was not recorded' }],
  };
  // Act
  const html = renderToStaticMarkup(<ComparisonRows rows={[row]} sides={['A', 'B']} warn={false}/>);
  const hidden = renderToStaticMarkup(<ComparisonRows rows={[{ ...row, hidden: true }]} sides={['A', 'B']} warn={false}/>);
  // Assert
  assert.match(html, /Measured CPU time/);
  assert.match(html, /1.000000000 s/);
  assert.match(html, /Partial: one interval missing/);
  assert.doesNotMatch(html, /opaque.key/);
  assert.doesNotMatch(hidden, /1.000000000|Partial: one interval missing|service was not recorded/);
});
