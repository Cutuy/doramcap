import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer, type AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { PREPARE_PATH, RUNNER_PATH } from '../prepare-build.mts';
import { createSnapshot } from '../snapshot.mts';

const TEST_TIMEOUT_MS = 10_000;
const workerURL = new URL('./worker.mts', import.meta.url).href;
const executable = (body: string) => `#!${process.execPath}\n${body}\n`;
async function fixture(t: TestContext, failure = '', blocked = '') {
  const root = mkdtempSync(join(tmpdir(), 'native worker '));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const source = join(root, 'source'), job = join(root, 'job'), home = join(root, 'home'), trace = join(root, 'trace.ndjson');
  const put = (name: string, text: string) => {
    const path = join(source, name); mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, text);
  };
  put('.node-version', process.versions.node);
  put('Cargo.toml', '[workspace]');
  put('apps/system-monitor/tests/validate_native.ts', '// fixture');
  put('libs/resources/cgroup-bpf/examples/native_allocations.rs', '// fixture');
  for (const path of [PREPARE_PATH, RUNNER_PATH, 'scripts/ci/worker-process.mts']) put(path, readFileSync(new URL('../' + path.slice('scripts/ci/'.length), import.meta.url), 'utf8'));
  const record = `require('node:fs').appendFileSync(${JSON.stringify(trace)}, JSON.stringify({name, args:process.argv.slice(2), cwd:process.cwd(), target:process.env.CARGO_TARGET_DIR, debug:process.env.CARGO_PROFILE_DEV_DEBUG, toolchain:process.env.RUSTUP_TOOLCHAIN, clang:process.env.CLANG}) + '\\n');`;
  const binary = (name: string) => executable(`const name=${JSON.stringify(name)}; ${record}`);
  const target = join(home, 'target');
  const fakeTool = executable(`
const {basename, join} = require('node:path');
const {writeFileSync, mkdirSync} = require('node:fs');
const name=basename(process.argv[1]), args=process.argv.slice(2); ${record}
if (name === 'cargo' && args[0] === 'test') { ${blocked} if (${JSON.stringify(failure)} === 'cargo-test') process.exit(7); }
if (name === 'cargo' && args[0] === 'build') {
  mkdirSync(${JSON.stringify(join(target, 'debug/examples'))}, {recursive:true});
  for (const [name, text] of ${JSON.stringify(['policy_overhead', 'storage_benchmark'].map(name => [name, binary(name)]))}) writeFileSync(join(${JSON.stringify(target)}, 'debug/examples', name), text, {mode:0o755});
}
if (name === 'rustc' && args.includes('-o')) writeFileSync(args[args.indexOf('-o')+1], 'compiled fixture');
if (name === 'sudo') {
  if (args[0] !== '-n' || args[1] !== '/usr/local/libexec/doramcap-ci-native-tests' || !require('node:fs').existsSync(${JSON.stringify(join(target, 'debug/supervisor-test-fixture'))})) process.exit(9);
  writeFileSync(${JSON.stringify(join(root, 'native-passed'))}, 'passed');
}
`);
  for (const [name, label] of [['check-boundaries.mts', 'boundaries'], ['test-node.mts', 'node'], ['test-mcap.mts', 'mcap']]) {
    put('scripts/ci/' + name, `import {createRequire} from 'node:module'; const require=createRequire(import.meta.url); const name=${JSON.stringify(label)}; ${record}`);
  }
  mkdirSync(join(home, '.local/bin'), { recursive: true });
  for (const tool of ['cargo', 'rustc', 'rustup', 'npm', 'uname', 'sudo']) {
    writeFileSync(join(home, '.local/bin', tool), fakeTool, { mode: 0o755 });
  }
  for (const args of [['init', '-q', source], ['-C', source, 'add', '.'], ['-C', source, '-c', 'user.name=CI Fixture', '-c', 'user.email=ci@example.invalid', '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture']]) {
    assert.equal(spawnSync('git', args).status, 0);
  }
  const archive = join(root, 'source.tar.gz'); await createSnapshot(source, archive);
  mkdirSync(job); assert.equal(spawnSync('tar', ['-xzf', archive, '-C', job]).status, 0);
  const harness = join(root, 'harness.mts');
  writeFileSync(harness, `import {runNativeWorker} from ${JSON.stringify(workerURL)}; process.exitCode=await runNativeWorker(${JSON.stringify(job)}, ${JSON.stringify(home)});`);
  return { root, job, home, trace, harness, cache: join(home, 'jobs/.build-cache'), target,
    commands: () => readFileSync(trace, 'utf8').trim().split('\n').map(line => JSON.parse(line)) };
}
function start(t: TestContext, harness: string) {
  const child = spawn(process.execPath, [harness], { detached: true, stdio: ['ignore', 'ignore', 'pipe'] });
  const done = once(child, 'close');
  let stderr = ''; child.stderr!.on('data', bytes => { stderr += bytes; });
  t.after(() => { if (child.exitCode === null && child.signalCode === null) { try { process.kill(-child.pid!, 'SIGKILL'); } catch {} } });
  return { child, done, stderr: () => stderr };
}
async function notification(t: TestContext) {
  const server = createServer(); server.listen(0, '127.0.0.1'); await once(server, 'listening');
  t.after(() => server.close());
  const received = once(server, 'connection').then(async ([socket]) => {
    const [data] = await once(socket, 'data'); socket.destroy(); return Number(data.toString());
  });
  const port = (server.address() as AddressInfo).port;
  return { received, code: `require('node:net').connect(${port}, '127.0.0.1', function(){this.end(String(process.pid));}); setInterval(()=>{},1000);` };
}

test('native CI builds unprivileged before invoking the existing sudo launcher and benchmarks', async t => {
  // Arrange
  const f = await fixture(t);
  // Act
  const result = spawnSync(process.execPath, [f.harness], { encoding: 'utf8', timeout: TEST_TIMEOUT_MS });
  // Assert
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.equal(readFileSync(join(f.root, 'native-passed'), 'utf8'), 'passed');
  const commands = f.commands(), sudo = commands.findIndex(c => c.name === 'sudo'), build = commands.findIndex(c => c.name === 'cargo' && c.args[0] === 'build');
  assert(build >= 0 && sudo > build);
  assert.equal(commands.at(-1).name, 'storage_benchmark');
  assert.equal(commands.filter(c => c.name === 'sudo').length, 1);
  assert.deepEqual(commands[sudo].args, ['-n', '/usr/local/libexec/doramcap-ci-native-tests', join(f.cache, 'source')]);
  assert.deepEqual(commands.filter(c => c.name === 'cargo' && c.args[0] === 'test').map(c => c.args), [
    ['test', '--locked', '-p', 'storage-persistence', '--no-default-features'], ['test', '--locked', '--workspace'], ['test', '--locked', '--workspace', '--all-features']]);
  for (const c of commands) {
    assert.equal(c.cwd, join(f.cache, 'source')); assert.equal(c.target, f.target);
    assert.equal(c.debug, 'line-tables-only'); assert.equal(c.toolchain, '1.99.1'); assert.equal(c.clang, 'clang-18');
  }
  assert.equal(spawnSync('flock', ['--nonblock', join(f.cache, 'worker.lock'), 'true']).status, 0);
});
test('a failed build/test preserves its exit code and stops before privilege or benchmarks', async t => {
  // Arrange
  const f = await fixture(t, 'cargo-test');
  // Act
  const result = spawnSync(process.execPath, [f.harness], { encoding: 'utf8', timeout: TEST_TIMEOUT_MS });
  // Assert
  assert.equal(result.status, 7, result.stderr);
  assert.equal(existsSync(join(f.root, 'native-passed')), false);
  assert.equal(f.commands().at(-1).name, 'cargo');
  assert.equal(spawnSync('flock', ['--nonblock', join(f.cache, 'worker.lock'), 'true']).status, 0);
});
test('wrong Node versions fail before cache preparation or commands', async t => {
  // Arrange
  const f = await fixture(t); writeFileSync(join(f.job, '.node-version'), '0.0.0');
  // Act
  const result = spawnSync(process.execPath, [f.harness], { encoding: 'utf8', timeout: TEST_TIMEOUT_MS });
  // Assert
  assert.equal(result.status, 1); assert.match(result.stderr, /Node version .*; expected v0\.0\.0/);
  assert.equal(existsSync(f.cache), false); assert.equal(existsSync(f.trace), false);
});
for (const [signal, status] of [['SIGHUP', 129], ['SIGINT', 130], ['SIGTERM', 143]] as const) {
  test(`${signal} stops active native CI work and releases its cache`, { timeout: TEST_TIMEOUT_MS }, async t => {
    // Arrange
    const notice = await notification(t), f = await fixture(t, '', notice.code), worker = start(t, f.harness);
    const consumer = await notice.received;
    // Act
    worker.child.kill(signal); await worker.done;
    // Assert
    assert.equal(worker.child.exitCode, status, worker.stderr());
    assert.throws(() => process.kill(consumer, 0), /ESRCH/);
    assert.equal(existsSync(join(f.root, 'native-passed')), false);
    assert.equal(spawnSync('flock', ['--nonblock', join(f.cache, 'worker.lock'), 'true']).status, 0);
  });
}
