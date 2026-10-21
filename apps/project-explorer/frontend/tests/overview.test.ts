import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import data from '../../overview.json';
import { validateOverview } from '../scripts/validate-overview';
import { directDependencies, dependencyChain, type Catalog } from '../src/model';
const catalog = JSON.parse(readFileSync(new URL('../src/.generated/catalog.json', import.meta.url), 'utf8')) as Catalog;

test('canonical overview references exist in the generated repository', () => {
  // Arrange / Act / Assert
  assert.doesNotThrow(() => validateOverview(data, catalog));
});

for (const [name, mutate, expected] of [
  ['unsupported version', (d: typeof data) => { d.version = 2; }, /schema/],
  ['extra field', (d: any) => { d.useCases[0].silentOverride = true; }, /schema/],
  ['unknown component', (d: typeof data) => { d.useCases[0].components[0] = 'apps/missing'; }, /unknown component/],
  ['unknown capability', (d: typeof data) => { d.useCases[0].capabilities[0] = 'missing'; }, /unknown capability/],
  ['unknown integration', (d: typeof data) => { d.useCases[0].integrations[0] = 'missing'; }, /unknown integration/],
  ['duplicate ID', (d: typeof data) => { d.useCases[1].id = d.useCases[0].id; }, /Duplicate/],
  ['missing evidence', (d: typeof data) => { d.capabilities[0].evidence[0] = 'docs/missing.md'; }, /unknown evidence/],
  ['unrelated component', (d: typeof data) => { d.useCases[0].components.push('apps/project-explorer'); }, /outside its capabilities/],
  ['unrelated integration', (d: typeof data) => { d.useCases[0].capabilities = ['configuration']; d.useCases[0].components = ['crates/pbtxt-config']; }, /outside its capabilities/],
] as const) {
  test(`overview validation rejects ${name}`, () => {
    // Arrange
    const altered = structuredClone(data);
    mutate(altered);
    // Act / Assert
    assert.throws(() => validateOverview(altered, catalog), expected);
  });
}

test('direct dependencies exclude incoming and transitive declarations', () => {
  // Arrange
  const fixture = { ...catalog, components: [], packages: ['a', 'b', 'c', 'd'].map(name => ({ name, component: name, directory: name })), dependencies: [
    { source: 'a', target: 'b', kind: 'normal', optional: true, condition: 'cfg(unix)', features: [] },
    { source: 'b', target: 'c', kind: 'normal', optional: false, condition: null, features: [] },
    { source: 'd', target: 'a', kind: 'dev', optional: false, condition: null, features: [] },
  ] } as Catalog;
  // Act
  const result = directDependencies(fixture, 'a');
  // Assert
  assert.equal(result.dependencies.length, 1);
  assert.equal(result.dependencies[0].target, 'b');
  assert.equal(result.dependencies[0].condition, 'cfg(unix)');
});

test('app dependency view includes declarations from every nested package', () => {
  // Arrange
  const app = catalog.components.find(c => c.id === 'apps/system-monitor')!;
  // Act
  const result = directDependencies(catalog, app.id);
  // Assert
  assert.equal(result.packages.length, 2);
  assert.deepEqual(result.dependencies, catalog.dependencies.filter(d => app.packages.includes(d.source)));
});


test('full dependency closure deduplicates shared nodes and terminates cycles', () => {
  // Arrange
  const fixture = { ...catalog, components: [], packages: ['a', 'b', 'c', 'd'].map(name => ({ name, component: name, directory: name })), dependencies: [
    { source: 'a', target: 'b', kind: 'normal', optional: false, condition: null, features: [] },
    { source: 'b', target: 'c', kind: 'normal', optional: true, condition: 'cfg(unix)', features: [] },
    { source: 'a', target: 'c', kind: 'normal', optional: false, condition: null, features: [] },
    { source: 'c', target: 'a', kind: 'dev', optional: false, condition: null, features: [] },
    { source: 'd', target: 'a', kind: 'normal', optional: false, condition: null, features: [] },
  ] } as Catalog;
  // Act
  const graph = dependencyChain(fixture, 'a', '', ['normal', 'dev']);
  // Assert
  assert.deepEqual(graph.packages.map(p => p.name), ['a', 'b', 'c', 'd']);
  assert.equal(graph.dependencies.length, 5);
  assert.ok(graph.dependencies.some(d => d.optional && d.condition === 'cfg(unix)'));
});


test('unmapped platform placeholders cannot claim an existing integration', () => {
  // Arrange
  const altered = structuredClone(data);
  const placeholder = altered.integrations.find(i => i.id === 'nvsci-nvmedia')!;
  // Act / Assert
  assert.doesNotThrow(() => validateOverview(altered, catalog));
  // Act
  placeholder.status = 'integrated';
  // Assert
  assert.throws(() => validateOverview(altered, catalog), /schema/);
});


test('focused graph follows both directions without pulling in sibling branches', () => {
  // Arrange
  const names = ['app', 'parent', 'focus', 'dependency', 'leaf', 'sibling', 'other'];
  const fixture = { ...catalog, components: [], packages: names.map(name => ({ name, component: name, directory: name })), dependencies: [
    ['app', 'parent'], ['parent', 'focus'], ['focus', 'dependency'], ['dependency', 'leaf'],
    ['parent', 'sibling'], ['other', 'dependency'],
  ].map(([source, target]) => ({ source, target, kind: 'normal', optional: false, condition: null, features: [] })) } as Catalog;
  // Act
  const graph = dependencyChain(fixture, 'focus', '', ['normal']);
  // Assert
  assert.deepEqual(graph.roots, ['focus']);
  assert.deepEqual(graph.packages.map(p => p.name), ['app', 'parent', 'focus', 'dependency', 'leaf']);
  assert.equal(graph.dependencies.length, 4);
});
