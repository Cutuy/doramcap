import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { cliArguments, probeOptions, UPSTREAM } from './holoscan/probe.mts';

test('real probes require an explicit phase, finite mode and absolute paths', () => {
  // Arrange
  const args = ['--phase', 'preview', '--mode', 'finite', '--cli', '/tmp/holoscan', '--output', '/tmp/evidence'];
  // Act / Assert
  assert.equal(probeOptions(args).phase, 'preview');
  assert.equal(probeOptions(args.map(value => value === 'preview' ? 'real' : value)).phase, 'real');
  assert.throws(() => probeOptions(args.slice(2)), /--phase/);
  assert.throws(() => probeOptions(args.map(value => value === 'finite' ? 'ci' : value)), /--mode finite/);
  assert.throws(() => probeOptions(args.map(value => value === '/tmp/evidence' ? 'relative' : value)), /--output/);
  assert.throws(() => probeOptions(args.map(value => value === '/tmp/evidence' ? '/tmp/path,option' : value)), /--output/);
});

test('CLI previews cannot build or launch; run commands independently disable both builds', () => {
  // Arrange
  const root = '/tmp/project', image = 'probe:test';
  // Act
  const build = cliArguments(root, 'build', 'finite', image, true);
  const run = cliArguments(root, 'run', 'finite', image, true, '--label probe=test', '/tmp/payload');
  const real = cliArguments(root, 'run', 'finite', image, false);
  // Assert
  assert(build.includes('--dryrun')); assert(run.includes('--dryrun'));
  assert(run.includes('--no-docker-build')); assert(run.includes('--no-local-build'));
  assert(real.includes('--no-docker-build')); assert(real.includes('--no-local-build'));
  assert(!real.includes('--dryrun'));
  assert.deepEqual(run.slice(-3), ['--add-volume', '/tmp/payload', '--dryrun']);
  assert.deepEqual(run.slice(0, 5), ['--project-root', root, 'run', 'first_pipeline', 'finite']);
});

test('the vendored official example remains byte-for-byte upstream source', () => {
  // Arrange
  const source = readFileSync(new URL('./holoscan/first_pipeline/applications/first_pipeline/first_pipeline.py', import.meta.url));
  // Act / Assert
  assert.equal(createHash('sha256').update(source).digest('hex'), UPSTREAM.sha256);
});
