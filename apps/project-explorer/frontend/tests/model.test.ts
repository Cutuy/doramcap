import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { displayName, parseRoute, resolveLink, directDependencies, type Catalog } from '../src/model';

const catalog = JSON.parse(readFileSync(fileURLToPath(new URL('../src/.generated/catalog.json', import.meta.url)), 'utf8')) as Catalog;

test('readable titles retain technical acronyms', () => {
  // Arrange
  const identifiers = ['activity-tracing', 'cpu-viewer', 'cuda-shm-demo', 'mcap-recorder'];
  // Act
  const titles = identifiers.map(displayName);
  // Assert
  assert.deepEqual(titles, ['Activity Tracing', 'CPU Viewer', 'CUDA SHM Demo', 'MCAP Recorder']);
});

test('component, document, image, source and heading links retain their destinations under a subpath', () => {
  // Arrange
  const doc = 'libs/activity-tracing/README.md';
  const resolve = (url: string) => resolveLink(url, doc, catalog, '/doramcap/');
  // Act / Assert
  assert.equal(resolve('../../apps/activity-demo/'), '#/apps/activity-demo');
  assert.equal(resolve('docs/architecture.svg'), '/doramcap/content/libs/activity-tracing/docs/architecture.svg');
  assert.equal(resolve('docs/validation.md#some-heading'), '#/docs?heading=some-heading&path=libs%2Factivity-tracing%2Fdocs%2Fvalidation.md');
  assert.equal(resolve('#ownership'), '#/crates/activity-tracing?heading=ownership');
  assert.equal(resolve('src/lib.rs'), `${catalog.repository}/blob/${catalog.revision}/libs/activity-tracing/src/lib.rs`);
  assert.equal(resolve('https://example.com/docs#x'), 'https://example.com/docs#x');
  assert.equal(resolve('javascript:alert(1)'), '');
  assert.equal(resolve('../../../../secret'), '');
});

test('nested Cargo packages map to a logical app and dependencies are generated', () => {
  // Arrange
  const app = catalog.components.find(c => c.id === 'apps/system-monitor')!;
  // Act
  const graph = directDependencies(catalog, 'apps/provenance-demo');
  // Assert
  assert.equal(app.directory, 'apps/system-monitor');
  assert.equal(app.packages.length, 2);
  assert.ok(graph.dependencies.some(d => d.target === 'step-provenance'));
  assert.ok(graph.dependencies.some(d => d.target === 'activity-tracing'));
  assert.ok(graph.dependencies.every(d => d.kind === 'normal'));
  assert.ok(catalog.components.find(c => c.id === 'apps/project-explorer' && c.packages.length === 0));
});

test('deep links preserve search, categories and document anchors', () => {
  // Arrange
  const hash = '#/crates?category=Observability&q=activity+tracing&heading=ownership';
  // Act
  const route = parseRoute(hash);
  // Assert
  assert.equal(route.path, '/crates');
  assert.equal(route.params.get('q'), 'activity tracing');
  assert.equal(route.params.get('category'), 'Observability');
  assert.equal(route.params.get('heading'), 'ownership');
});

test('catalog is portable and only includes repository image assets', () => {
  // Arrange
  const paths = [...catalog.assets, ...catalog.files, ...catalog.packages.map(p => p.directory)];
  // Act
  const assets = catalog.assets;
  // Assert
  assert.ok(paths.every(path => !path.startsWith('/') && !path.includes('..')));
  assert.ok(assets.every(path => /\.(svg|png|jpe?g|webp|gif)$/i.test(path)));
  assert.ok(!Object.keys(catalog.documents).some(path => path.includes('AGENTS.md')));
});


test('image-first READMEs contribute prose summaries', () => {
  // Arrange
  const ids = ['apps/fault-demo', 'apps/fault-watchdog'];
  // Act
  const summaries = ids.map(id => catalog.components.find(c => c.id === id)!.summary);
  // Assert
  assert.ok(summaries[0].startsWith('Two synthetic reporters'));
  assert.ok(summaries[1].startsWith('Production Dora host'));
});
