import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { readSystemAppChoice, saveSystemAppChoice, systemAppPreferenceKey } from './system-app-preferences';

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
  const choice = { ids: ['cpu-monitor', 'data-recorder'], profiles: { 'cpu-monitor': 'record', 'data-recorder': 'cpu' } };
  // Act / Assert
  saveSystemAppChoice('project', choice);
  assert.deepEqual(readSystemAppChoice('project'), choice);
  assert.equal(readSystemAppChoice('another-project'), undefined);
  saveSystemAppChoice('project');
  assert.equal(readSystemAppChoice('project'), undefined);
  // Act / Assert: selecting no optional apps remains an explicit choice.
  saveSystemAppChoice('project', { ids: [], profiles: {} });
  assert.deepEqual(readSystemAppChoice('project'), { ids: [], profiles: {} });
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
  assert.doesNotThrow(() => saveSystemAppChoice('project', { ids: [], profiles: {} }));
  assert.doesNotThrow(() => saveSystemAppChoice('project'));
  Reflect.deleteProperty(globalThis, 'window');
  assert.equal(readSystemAppChoice('project'), undefined);
});
