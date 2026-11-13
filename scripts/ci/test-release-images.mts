import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, type TestContext } from 'node:test';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const names = ['dora', 'system-monitor', 'mcap-recorder', 'live-bridge', 'fault-watchdog', 'cpu-burn'];
const registry = 'example.test/console';
const version = '0.0.1-rc6';
const digest = (name: string) => `sha256:${String(names.indexOf(name) + 1).repeat(64)}`;

function fixture(t: TestContext, failure = '') {
  const temp = mkdtempSync(join(tmpdir(), 'console-promotion-'));
  t.after(() => rmSync(temp, { recursive: true, force: true }));
  const state = join(temp, 'registry.json');
  writeFileSync(state, JSON.stringify({ calls: [], latest: {} }));
  writeFileSync(join(temp, 'docker'), `#!${process.execPath}
const fs = require('node:fs');
const state = ${JSON.stringify(state)};
const value = JSON.parse(fs.readFileSync(state));
const args = process.argv.slice(2); value.calls.push(args);
const reference = args[3];
const name = reference?.split('/').at(-1)?.split(/[:@]/)[0];
const digests = ${JSON.stringify(Object.fromEntries(names.map(name => [name, digest(name)])))};
const failure = ${JSON.stringify(failure)};
if (args[2] === 'create') {
  value.latest[args[4]] = args[5].split('@')[1];
} else if (args[2] === 'inspect') {
  if (args.includes('--raw')) {
    const architectures = failure === 'missing-arm64' && name === 'cpu-burn' ? ['amd64'] : ['amd64', 'arm64'];
    console.log(JSON.stringify({manifests: architectures.map(architecture => ({platform: {os:'linux',architecture}}))}));
  } else {
    const digest = reference.endsWith(':latest')
      ? failure === 'wrong-latest' ? 'sha256:'+'f'.repeat(64) : value.latest[reference]
      : digests[name];
    console.log(JSON.stringify({digest}));
  }
} else { throw new Error('Unexpected Docker operation: '+args); }
fs.writeFileSync(state, JSON.stringify(value));
`, { mode: 0o755 });
  return {
    run: () => spawnSync(process.execPath, [join(root, 'console/scripts/release-images.mts'), 'promote', '--registry', registry, '--version', version], {
      encoding: 'utf8', env: { ...process.env, PATH: `${temp}:${process.env.PATH}` },
    }),
    state: () => JSON.parse(readFileSync(state, 'utf8')) as { calls: string[][]; latest: Record<string, string> },
  };
}

test('latest uses published digests after every architecture is checked and can be retried', t => {
  // Arrange
  const registry = fixture(t);
  // Act
  const first = registry.run();
  // Assert
  assert.equal(first.status, 0, first.stderr);
  const published = registry.state();
  assert.equal(published.calls.findIndex(args => args[2] === 'create'), names.length * 2);
  assert.equal(Object.keys(published.latest).length, names.length);
  for (const name of names) {
    const tag = `example.test/console/${name}:latest`;
    assert.equal(published.latest[tag], digest(name));
    assert(published.calls.some(args => args[2] === 'create' && args[4] === tag && args[5] === `example.test/console/${name}@${digest(name)}`));
  }
  // Act / Assert: repeating a failed CI job is safe for already promoted images.
  const second = registry.run();
  assert.equal(second.status, 0, second.stderr);
  assert.deepEqual(registry.state().latest, published.latest);
});

test('a missing published architecture leaves every latest tag unchanged', t => {
  // Arrange
  const registry = fixture(t, 'missing-arm64');
  // Act
  const result = registry.run();
  // Assert
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /lacks linux\/arm64/);
  assert.deepEqual(registry.state().latest, {});
  assert(!registry.state().calls.some(args => args[2] === 'create'));
});

test('a registry that serves the wrong promoted digest fails the job', t => {
  // Arrange
  const registry = fixture(t, 'wrong-latest');
  // Act
  const result = registry.run();
  // Assert
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Promoted digest differs/);
});
