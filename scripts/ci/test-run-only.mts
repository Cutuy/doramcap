import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer, type AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { Readable } from 'node:stream';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';
import { artifactNames } from './worker-artifacts.mts';
import { validateWorkerPlatform } from './run-only-worker.mts';
import { runCommand, WorkerInterrupted } from './worker-process.mts';

const TEST_TIMEOUT_MS = 10_000;
const workerURL = new URL('./run-only-worker.mts', import.meta.url).href;
const workerScript = fileURLToPath(workerURL);
const executable = (body: string) => `#!${process.execPath}\n${body}\n`;

function workerFixture(t: TestContext, failingSuite = '', runningBody = '') {
  const root = mkdtempSync(join(tmpdir(), 'run-only-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const job = join(root, 'job'), cache = join(root, 'cache'), home = join(root, 'home');
  const artifacts = join(job, 'artifacts'), source = join(job, 'source'), trace = join(root, 'commands');
  for (const path of [artifacts, source, cache, home]) mkdirSync(path, { recursive: true });
  writeFileSync(join(source, 'SOURCE.json'), '{"fixture":true}');
  writeFileSync(join(source, '.node-version'), process.versions.node + '\n');
  for (const name of artifactNames) writeFileSync(join(artifacts, name), 'fixture');
  assert.equal(spawnSync('tar', ['-czf', join(artifacts, 'source.tar.gz'), '-C', source, '.']).status, 0);
  writeFileSync(join(artifacts, 'cargo-nextest'), executable(`
const { appendFileSync } = require('node:fs');
if (process.argv[2] === '--version') { console.log('fixture-nextest'); process.exit(0); }
appendFileSync(${JSON.stringify(trace)}, process.argv.slice(2).join(' ') + '\\n');
${runningBody || `process.exit(process.argv.includes(${JSON.stringify(join(artifacts, failingSuite))}) ? 7 : 0);`}
`));
  for (const name of ['docker', 'docker-compose']) writeFileSync(join(artifacts, name), executable('process.exit(0);'));
  const manifest = artifactNames.map(name => createHash('sha256').update(readFileSync(join(artifacts, name))).digest('hex') + '  ' + name).join('\n') + '\n';
  writeFileSync(join(artifacts, 'SHA256SUMS'), manifest);
  const options = { mode: 'run', job, cache, home };
  const harness = join(root, 'harness.mts');
  writeFileSync(harness, `import {runWorker} from ${JSON.stringify(workerURL)};\nprocess.exitCode = await runWorker(${JSON.stringify(options)});\n`);
  return { root, job, cache, home, artifacts, trace, manifest, harness, options };
}
function runWorkerFixture(fixture: ReturnType<typeof workerFixture>) {
  return spawnSync(process.execPath, [fixture.harness], { encoding: 'utf8', timeout: TEST_TIMEOUT_MS });
}
function start(t: TestContext, program: string, args: string[], extraPipe = false) {
  const child = spawn(program, args, { detached: true, stdio: extraPipe ? ['pipe', 'pipe', 'pipe', 'pipe'] : ['pipe', 'pipe', 'pipe'] });
  const done = once(child, 'close');
  t.after(() => {
    if (child.exitCode === null && child.signalCode === null) {
      try { process.kill(-child.pid!, 'SIGKILL'); } catch {}
    }
  });
  return { child, done };
}
async function notification(t: TestContext) {
  const server = createServer(); server.listen(0, '127.0.0.1'); await once(server, 'listening');
  t.after(() => server.close());
  const received = once(server, 'connection').then(async ([socket]) => {
    const [data] = await once(socket, 'data'); socket.destroy(); return Number(data.toString());
  });
  const port = (server.address() as AddressInfo).port;
  const code = `require('node:net').connect(${port}, '127.0.0.1', function() { this.end(String(process.pid)); });`;
  return { received, code };
}
function log(fixture: ReturnType<typeof workerFixture>) { return readFileSync(join(fixture.job, 'run.log'), 'utf8'); }

for (const failed of ['', 'all-features.tar.zst']) {
  test(`worker attempts every suite when ${failed || 'all suites pass'}`, t => {
    // Arrange
    const fixture = workerFixture(t, failed);
    // Act
    const result = runWorkerFixture(fixture);
    // Assert
    assert.equal(result.status, failed ? 1 : 0, result.stderr + log(fixture));
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), failed ? '1\n' : '0\n');
    assert.equal(readFileSync(join(fixture.job, 'results.tsv'), 'utf8'), `default\t0\nall-features\t${failed ? 7 : 0}\nstorage-no-default\t0\n`);
    const commands = readFileSync(fixture.trace, 'utf8').trim().split('\n');
    assert.equal(commands.length, 3);
    for (const command of commands) assert.match(command, /--test-threads 1 --no-fail-fast/);
  });
}
for (const problem of ['missing', 'duplicate', 'extra', 'malformed', 'corrupt']) {
  test(`worker rejects ${problem} artifacts before installation or cache replacement`, t => {
    // Arrange
    const fixture = workerFixture(t), lines = fixture.manifest.trimEnd().split('\n');
    mkdirSync(join(fixture.cache, 'source'));
    const sentinel = join(fixture.cache, 'source/keep'); writeFileSync(sentinel, 'unchanged');
    if (problem === 'missing') lines.pop();
    if (problem === 'duplicate') lines.push(lines[0]);
    if (problem === 'extra') lines.push('0'.repeat(64) + '  unexpected');
    if (problem === 'malformed') lines[0] = 'not a checksum';
    if (problem === 'corrupt') writeFileSync(join(fixture.artifacts, 'docker'), 'changed');
    writeFileSync(join(fixture.artifacts, 'SHA256SUMS'), lines.join('\n') + '\n');
    // Act
    const result = runWorkerFixture(fixture);
    // Assert
    assert.equal(result.status, 1, result.stderr);
    assert.equal(readFileSync(sentinel, 'utf8'), 'unchanged');
    assert.equal(existsSync(join(fixture.home, '.local/bin/cargo-nextest')), false);
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), '1\n');
  });
}
test('worker accepts a complete reordered inventory', t => {
  // Arrange
  const fixture = workerFixture(t);
  writeFileSync(join(fixture.artifacts, 'SHA256SUMS'), fixture.manifest.trimEnd().split('\n').reverse().join('\n') + '\n');
  // Act / Assert
  assert.equal(runWorkerFixture(fixture).status, 0, log(fixture));
});
for (const [signal, status] of [['SIGHUP', 129], ['SIGINT', 130], ['SIGTERM', 143]] as const) {
  test(`${signal} cleans up an active suite and records interruption`, { timeout: TEST_TIMEOUT_MS }, async t => {
    // Arrange
    const notice = await notification(t);
    const fixture = workerFixture(t, '', notice.code + '\nsetInterval(() => {}, 1000);');
    const worker = start(t, process.execPath, [fixture.harness]);
    const testPid = await notice.received;
    // Act
    worker.child.kill(signal); await worker.done;
    // Assert
    assert.equal(worker.child.exitCode, status, log(fixture));
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), `${status}\n`);
    assert.equal(readFileSync(join(fixture.job, 'results.tsv'), 'utf8'), '');
    assert.throws(() => process.kill(testPid, 0), /ESRCH/);
  });
}
test('cancellation while waiting for the cache lock replaces old success and reaps the waiter', { timeout: TEST_TIMEOUT_MS }, async t => {
  // Arrange
  const fixture = workerFixture(t), lock = join(fixture.cache, 'worker.lock');
  const holder = start(t, 'flock', [lock, process.execPath, '-e', "require('node:fs').writeSync(3, 'ready'); process.stdin.resume();"], true);
  await once(holder.child.stdio[3] as Readable, 'data');
  const notice = await notification(t), tools = join(fixture.home, '.local/bin'); mkdirSync(tools, { recursive: true });
  writeFileSync(join(tools, 'flock'), executable(notice.code + `\nconst child = require('node:child_process').spawn('/usr/bin/flock', process.argv.slice(2), {stdio: ['ignore', 1, 2, 3]}); child.on('exit', code => process.exit(code));`), { mode: 0o755 });
  writeFileSync(join(fixture.job, 'exit-code'), '0\n'); writeFileSync(join(fixture.job, 'results.tsv'), 'old\t0\n');
  const worker = start(t, process.execPath, [fixture.harness]);
  const waiterPid = await notice.received;
  assert.equal(existsSync(join(fixture.job, 'exit-code')), false);
  assert.equal(existsSync(join(fixture.job, 'results.tsv')), false);
  // Act
  worker.child.kill('SIGTERM'); await worker.done;
  // Assert
  assert.equal(worker.child.exitCode, 143, log(fixture));
  assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), '143\n');
  assert.throws(() => process.kill(waiterPid, 0), /ESRCH/);
  holder.child.stdin!.end(); await holder.done;
});
test('process cancellation escalates when a child ignores termination', { timeout: TEST_TIMEOUT_MS }, async t => {
  // Arrange
  const notice = await notification(t), controller = new AbortController();
  const pending = runCommand(process.execPath, ['-e', `process.on('SIGTERM', () => {}); ${notice.code} setInterval(() => {}, 1000);`], {
    signal: controller.signal, cwd: tmpdir(), env: process.env, output: 1, shutdownGraceMs: 50,
  });
  const pid = await notice.received;
  // Act / Assert
  const rejected = assert.rejects(pending, WorkerInterrupted);
  controller.abort(new WorkerInterrupted('SIGTERM')); await rejected;
  assert.throws(() => process.kill(pid, 0), /ESRCH/);
});
test('cancellation reaches descendants before the root command exits', { timeout: TEST_TIMEOUT_MS }, async t => {
  // Arrange
  const notice = await notification(t), controller = new AbortController();
  const descendant = notice.code + 'setInterval(() => {}, 1000);';
  const root = `process.on('SIGTERM', () => {}); const child = require('node:child_process').spawn(process.execPath, ['-e', ${JSON.stringify(descendant)}], {stdio: 'inherit'}); child.on('close', () => process.exit(0));`;
  const pending = runCommand(process.execPath, ['-e', root], { signal: controller.signal, cwd: tmpdir(), env: process.env, output: 1 });
  const descendantPid = await notice.received;
  // Act / Assert
  const rejected = assert.rejects(pending, WorkerInterrupted);
  controller.abort(new WorkerInterrupted('SIGTERM')); await rejected;
  assert.throws(() => process.kill(descendantPid, 0), /ESRCH/);
});
test('an active cache consumer retains the lock after its controller is killed', { timeout: TEST_TIMEOUT_MS }, async t => {
  // Arrange
  const notice = await notification(t);
  const fixture = workerFixture(t, '', notice.code + 'setInterval(() => {}, 1000);');
  const worker = start(t, process.execPath, [fixture.harness]);
  const consumerPid = await notice.received;
  t.after(() => { try { process.kill(-consumerPid, 'SIGKILL'); } catch {} });
  // Act
  worker.child.kill('SIGKILL'); await worker.done;
  // Assert
  assert.equal(existsSync(join(fixture.job, 'exit-code')), false);
  assert.equal(spawnSync('flock', ['--nonblock', join(fixture.cache, 'worker.lock'), 'true']).status, 1);
  const nextOwner = start(t, 'flock', [join(fixture.cache, 'worker.lock'), 'true']);
  process.kill(consumerPid, 'SIGTERM'); await nextOwner.done;
  assert.equal(nextOwner.child.exitCode, 0);
});
test('process startup failure and already-cancelled work fail without hanging', async () => {
  // Arrange
  const controller = new AbortController();
  const context = { signal: controller.signal, cwd: tmpdir(), env: process.env, output: 1 };
  // Act / Assert
  await assert.rejects(runCommand('/missing-worker-command', [], context), { code: 'ENOENT' });
  controller.abort(new WorkerInterrupted('SIGINT'));
  await assert.rejects(runCommand(process.execPath, ['-e', 'process.exit(0)'], context), WorkerInterrupted);
});
test('invalid invocation explains the expected mode and platform', () => {
  // Arrange / Act / Assert
  for (const args of [[], ['invalid']]) {
    const invalid = spawnSync(process.execPath, [workerScript, ...args], { encoding: 'utf8' });
    assert.equal(invalid.status, 2); assert.match(invalid.stderr, /Usage:.*build\|run/);
  }
  assert.throws(() => validateWorkerPlatform('darwin', 'arm64'), /requires Linux arm64/);
  assert.throws(() => validateWorkerPlatform('linux', 'x64'), /requires Linux arm64/);
  validateWorkerPlatform('linux', 'arm64');
});
for (const problem of ['node-version', 'compose-plugin']) {
  test(`build explains ${problem} failure before preparing source or installing tools`, t => {
    // Arrange
    const fixture = workerFixture(t);
    if (problem === 'node-version') writeFileSync(join(fixture.job, 'source/.node-version'), '0.0.0\n');
    writeFileSync(fixture.harness, `import {runWorker} from ${JSON.stringify(workerURL)};\nprocess.exitCode = await runWorker(${JSON.stringify({ ...fixture.options, mode: 'build', composePlugin: join(fixture.root, 'missing-compose') })});`);
    // Act
    const result = runWorkerFixture(fixture);
    // Assert
    assert.equal(result.status, 1);
    assert.match(result.stdout + result.stderr, problem === 'node-version' ? /Node version .*; expected v0\.0\.0/ : /Missing Docker Compose plugin:/);
    assert.equal(existsSync(join(fixture.cache, 'source')), false);
    assert.equal(existsSync(join(fixture.home, '.local/bin/cargo-nextest')), false);
  });
}
