import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { readSystemAppChoice, saveSystemAppChoice, systemAppPreferenceKey } from './system-app-preferences';
import { LIMITS } from './generated/transport';
import { wire } from './protocol';

test('browser choices stay within one page, embedded Console and project', () => {
  // Arrange / Act
  const scope = systemAppPreferenceKey('/', '', '/project/a');
  // Assert
  assert.ok(scope);
  assert.notEqual(scope, systemAppPreferenceKey('/other', '', '/project/a'));
  assert.notEqual(scope, systemAppPreferenceKey('/', '/console', '/project/a'));
  assert.notEqual(scope, systemAppPreferenceKey('/', '', '/project/b'));
  assert.equal(systemAppPreferenceKey('/', '', ''), undefined);
});

test('accepted browser choices round-trip and can be forgotten', t => {
  // Arrange
  const values = new Map<string, string>();
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'window');
  Object.defineProperty(globalThis, 'window', { configurable: true, value: { localStorage: {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  } } });
  t.after(() => { if (previous) Object.defineProperty(globalThis, 'window', previous); else Reflect.deleteProperty(globalThis, 'window'); });
  const fields = { ids: ['cpu-monitor', 'data-recorder'], profiles: { 'cpu-monitor': 'record', 'data-recorder': 'cpu' } };
  const choice = wire.SystemAppSelection.create(fields);
  // Act / Assert: existing plain JSON records load as generated protobuf messages.
  values.set('project', JSON.stringify(fields));
  assert.deepEqual(readSystemAppChoice('project'), choice);
  // Act / Assert
  saveSystemAppChoice('project', choice);
  assert.deepEqual(JSON.parse(values.get('project')!), fields);
  assert.deepEqual(readSystemAppChoice('project'), choice);
  assert.equal(readSystemAppChoice('another-project'), undefined);
  saveSystemAppChoice('project');
  assert.equal(readSystemAppChoice('project'), undefined);
  // Act / Assert: selecting no optional apps remains an explicit choice.
  const empty = wire.SystemAppSelection.create();
  saveSystemAppChoice('project', empty);
  assert.deepEqual(JSON.parse(values.get('project')!), { ids: [], profiles: {} });
  assert.deepEqual(readSystemAppChoice('project'), empty);
});

test('malformed and unavailable browser storage use defaults', t => {
  // Arrange
  let stored: string | null = null;
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'window');
  Object.defineProperty(globalThis, 'window', { configurable: true, value: { localStorage: {
    getItem: () => stored,
    setItem: () => { throw new Error('Storage full'); },
    removeItem: () => { throw new Error('Storage blocked'); },
  } } });
  t.after(() => { if (previous) Object.defineProperty(globalThis, 'window', previous); else Reflect.deleteProperty(globalThis, 'window'); });
  // Act / Assert
  for (const value of ['{', 'null', '[]', '{"ids":[],"profiles":[]}', '{"ids":["x","x"],"profiles":{}}', '{"ids":[3],"profiles":{}}', '{"ids":["x"],"profiles":{"other":"live"}}', '{"ids":["x"],"profiles":{"x":4}}']) {
    stored = value;
    assert.equal(readSystemAppChoice('project'), undefined, value);
  }
  // Act / Assert: browser inputs use the transport's supported selection limits.
  const ids = Array.from({ length: LIMITS.selectionLength }, (_, index) => `app-${index}`);
  stored = JSON.stringify({ ids, profiles: {} });
  assert.deepEqual(readSystemAppChoice('project')?.ids, ids);
  stored = JSON.stringify({ ids: [...ids, 'one-more'], profiles: {} });
  assert.equal(readSystemAppChoice('project'), undefined);
  stored = JSON.stringify({ ids: ['app'], profiles: { app: 'x'.repeat(LIMITS.stringLength) } });
  assert.equal(readSystemAppChoice('project')?.profiles.app.length, LIMITS.stringLength);
  stored = JSON.stringify({ ids: ['app'], profiles: { app: 'x'.repeat(LIMITS.stringLength + 1) } });
  assert.equal(readSystemAppChoice('project'), undefined);
  assert.doesNotThrow(() => saveSystemAppChoice('project', wire.SystemAppSelection.create()));
  assert.doesNotThrow(() => saveSystemAppChoice('project'));
  Reflect.deleteProperty(globalThis, 'window');
  assert.equal(readSystemAppChoice('project'), undefined);
});
