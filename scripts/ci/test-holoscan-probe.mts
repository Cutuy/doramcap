import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { createServer, type ServerResponse } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { cliArguments, probeOptions, UPSTREAM, watchEvents } from './holoscan/probe.mts';

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
  const build = cliArguments(root, 'build', 'finite', image, true, '--label doramcap.holoscan.probe=owner-build');
  const run = cliArguments(root, 'run', 'finite', image, true, '--label probe=test', '/tmp/payload');
  const real = cliArguments(root, 'run', 'finite', image, false);
  // Assert
  assert(build.includes('--docker-opts=--label doramcap.holoscan.probe=owner-build'));
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

for (const ending of ['end', 'abort'] as const) {
  test(`an event stream that ${ending}s after subscription fails pending observations immediately`, { timeout: 2_000 }, async t => {
    // Arrange: use the actual Unix-socket HTTP transport and end it after headers.
    const directory = mkdtempSync(join(tmpdir(), 'holoscan-events-'));
    let response: ServerResponse | undefined;
    const server = createServer((_request, incoming) => {
      response = incoming; incoming.writeHead(200, { 'Content-Type': 'application/json' }); incoming.flushHeaders();
    });
    t.after(() => { server.closeAllConnections(); server.close(); rmSync(directory, { recursive: true, force: true }); });
    const socket = join(directory, 'engine.sock');
    await new Promise<void>(resolve => server.listen(socket, resolve));
    const events = await watchEvents('owner-run', join(directory, 'events.jsonl'), socket);
    t.after(() => events.close());
    const observed = events.wait('die');
    // Act
    if (ending === 'end') response!.end(); else response!.destroy();
    // Assert: this must complete inside the test deadline, without the 30-second operation timer.
    await assert.rejects(observed, /Engine event (stream|request)/);
  });
}
