// Behavioral fixtures for dependency reachability, rule validation, and CLI exit codes.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { violations, type Metadata } from './check-boundaries.mts';

function graph(kind: string | null = null): Metadata {
  return { workspace_members: ['core', 'adapter'], packages: [
    { id: 'core', name: 'core', manifest_path: '/fixture/core/Cargo.toml', dependencies: [
      { name: 'adapter', kind, path: '/fixture/adapter' }] },
    { id: 'adapter', name: 'adapter', manifest_path: '/fixture/adapter/Cargo.toml', dependencies: [
      { name: 'bollard', kind: null }] },
  ] };
}

test('transitive and renamed dependencies use the package name', () => {
  // Arrange
  const metadata = graph();
  Object.assign(metadata.packages[0].dependencies[0], { rename: 'renamed' });
  // Act / Assert
  assert.deepEqual(violations(metadata, { rules: [{ from: 'core', forbid: ['bollard'] }] }), ['core -> adapter -> bollard']);
});

test('dependency kinds scope traversal and direct matches', () => {
  // Arrange
  const metadata = graph('dev');
  const rule = { from: 'core', forbid: ['adapt*'] };
  // Act / Assert
  assert.deepEqual(violations(metadata, { rules: [rule] }), []);
  assert.deepEqual(violations(metadata, { rules: [{ ...rule, kinds: ['dev'] }] }), ['core -> adapter']);
});

test('external name collisions are not traversed', () => {
  // Arrange
  const metadata = graph();
  delete metadata.packages[0].dependencies[0].path;
  // Act / Assert
  assert.deepEqual(violations(metadata, { rules: [{ from: 'core', forbid: ['bollard'] }] }), []);
});

test('invalid policies fail', () => {
  // Arrange / Act / Assert
  for (const rule of [{ from: 'missing', forbid: ['x'] }, { from: 'core', forbid: ['x'], kind: ['dev'] },
    { from: 'core', forbid: [] }, { from: 'core', forbid: ['x'], kinds: ['runtime'] }]) {
    assert.throws(() => violations(graph(), { rules: [rule] }));
  }
  for (const config of [null, [], {}, { rules: [] }, { rules: [], extra: true }]) {
    assert.throws(() => violations(graph(), config));
  }
});

test('CLI distinguishes success, violations, and malformed inputs', t => {
  // Arrange
  const root = mkdtempSync(join(tmpdir(), 'boundary-cli-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const metadata = join(root, 'metadata.json');
  const rules = join(root, 'rules.json');
  writeFileSync(metadata, JSON.stringify(graph()));
  const run = () => spawnSync(process.execPath, [fileURLToPath(new URL('./check-boundaries.mts', import.meta.url)),
    '--metadata', metadata, '--rules', rules], { encoding: 'utf8' });
  // Act / Assert
  writeFileSync(rules, JSON.stringify({ rules: [{ from: 'core', forbid: ['absent'] }] }));
  assert.equal(run().status, 0);
  writeFileSync(rules, JSON.stringify({ rules: [{ from: 'core', forbid: ['bollard'] }] }));
  assert.equal(run().status, 1);
  assert.match(run().stdout, /core -> adapter -> bollard/);
  writeFileSync(rules, '{}');
  assert.equal(run().status, 2);
});
