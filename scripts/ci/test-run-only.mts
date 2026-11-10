import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { Readable } from 'node:stream';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';
import { artifactNames, successfulBuild, verifyArtifactInventory, workerHost } from './submit-run-only.mts';

test('SSH workers accept account/host names and reject command and option injection', () => {
  // Arrange / Act / Assert
  assert.equal(workerHost('mac-mini-ubuntu24@100.64.0.22'), 'mac-mini-ubuntu24@100.64.0.22');
  assert.equal(workerHost('arm-builder'), 'arm-builder');
  for (const host of ['-oProxyCommand=id', '-user@host', 'user@host;id', 'user@$(id)', 'host name', 'user@-host']) {
    assert.throws(() => workerHost(host), /Invalid SSH worker/);
  }
});
test('only a successful build of the exact queued task permits transfer', () => {
  // Arrange
  const status = JSON.stringify({ tasks: { 7: { status: { Done: { result: 'Success' } } },
    8: { status: { Done: { result: { Failed: 1 } } } }, 9: { status: 'Running' } } });
  // Act / Assert
  successfulBuild(status, '7');
  for (const id of ['8', '9', '10']) assert.throws(() => successfulBuild(status, id), /did not succeed/);
  assert.throws(() => successfulBuild(status, '7;id'), /Invalid Pueue task ID/);
});

// Exercise the real shell functions with private job/cache paths and fixture tools.
const TEST_TIMEOUT_MS = 5_000;
const workerScript = fileURLToPath(new URL('./run-only-worker.sh', import.meta.url));
const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";

function workerFixture(t: TestContext, failingSuite = '') {
  const root = mkdtempSync(join(tmpdir(), 'run-only-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const job = join(root, 'job'), cache = join(root, 'cache'), tools = join(root, 'tools');
  const artifacts = join(job, 'artifacts'), source = join(root, 'source'), trace = join(root, 'commands');
  for (const path of [artifacts, source, tools, cache]) mkdirSync(path, { recursive: true });
  writeFileSync(join(source, 'SOURCE.json'), '{"fixture":true}');
  for (const name of artifactNames) writeFileSync(join(artifacts, name), 'fixture');
  assert.equal(spawnSync('tar', ['-czf', join(artifacts, 'source.tar.gz'), '-C', source, '.']).status, 0);
  writeFileSync(join(artifacts, 'cargo-nextest'), `#!/bin/bash
if [[ $1 == --version ]]; then echo fixture-nextest; exit 0; fi
printf '%s\\n' "$*" >> ${quote(trace)}
if [[ -n ${quote(failingSuite)} && $* == *${quote(failingSuite)}* ]]; then exit 7; fi
`);
  for (const name of ['docker', 'docker-compose']) writeFileSync(join(artifacts, name), '#!/bin/sh\nexit 0\n');
  const manifest = artifactNames.map(name => createHash('sha256').update(readFileSync(join(artifacts, name))).digest('hex') + '  ' + name).join('\n') + '\n';
  writeFileSync(join(artifacts, 'SHA256SUMS'), manifest);
  const harness = (extra = '') => {
    const path = join(root, 'harness.sh');
    writeFileSync(path, `source ${quote(workerScript)}
export PATH=${quote(tools)}:$PATH
install() { command install "$1" "$2" "$3" ${quote(tools)}/"\${4##*/}"; }
mkdir() {
  if [[ $* == "-p $HOME/.local/bin $HOME/.docker/cli-plugins" ]]; then return 0; fi
  command mkdir "$@"
}
${extra}
run_worker run ${quote(job)} ${quote(cache)}
`);
    return path;
  };
  return { root, job, cache, artifacts, trace, manifest, harness };
}

function runShell(path: string) {
  return spawnSync('bash', [path], { encoding: 'utf8', timeout: TEST_TIMEOUT_MS });
}
function startShell(t: TestContext, path: string) {
  const child = spawn('bash', [path], { detached: true, stdio: ['pipe', 'pipe', 'pipe', 'pipe'] });
  const done = once(child, 'close');
  t.after(() => {
    if (child.exitCode === null && child.signalCode === null) process.kill(-child.pid!, 'SIGKILL');
  });
  return { child, done };
}
function ready(child: ChildProcess): Promise<void> {
  return new Promise((resolve, reject) => {
    (child.stdio[3] as Readable).once('data', () => resolve());
    child.once('error', reject);
    child.once('exit', code => reject(new Error(`Worker exited before readiness: ${code}`)));
  });
}

for (const failed of ['', 'all-features.tar.zst']) {
  test(`worker attempts every suite when ${failed || 'all suites pass'}`, { timeout: TEST_TIMEOUT_MS }, t => {
    // Arrange
    const fixture = workerFixture(t, failed);
    // Act
    const result = runShell(fixture.harness());
    // Assert
    assert.equal(result.status, failed ? 1 : 0, result.stderr + readFileSync(join(fixture.job, 'run.log'), 'utf8'));
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), failed ? '1\n' : '0\n');
    assert.equal(readFileSync(join(fixture.job, 'results.tsv'), 'utf8'), `default\t0\nall-features\t${failed ? 7 : 0}\nstorage-no-default\t0\n`);
    assert.equal(readFileSync(fixture.trace, 'utf8').trim().split('\n').length, 3);
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
    const manifest = lines.join('\n') + '\n'; writeFileSync(join(fixture.artifacts, 'SHA256SUMS'), manifest);
    const installTrace = join(fixture.root, 'installed');
    // Act
    const result = runShell(fixture.harness(`install() { touch ${quote(installTrace)}; }`));
    // Assert
    assert.notEqual(result.status, 0);
    assert.equal(readFileSync(sentinel, 'utf8'), 'unchanged');
    assert.equal(existsSync(installTrace), false);
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), '1\n');
    if (problem !== 'corrupt') assert.throws(() => verifyArtifactInventory(manifest));
  });
}

test('coordinator and shell accept a complete reordered inventory', t => {
  // Arrange
  const fixture = workerFixture(t), reversed = fixture.manifest.trimEnd().split('\n').reverse().join('\n') + '\n';
  writeFileSync(join(fixture.artifacts, 'SHA256SUMS'), reversed);
  // Act / Assert
  verifyArtifactInventory(reversed);
  assert.equal(runShell(fixture.harness()).status, 0);
});

for (const [signal, status] of [['SIGHUP', 129], ['SIGINT', 130], ['SIGTERM', 143]] as const) {
  test(`${signal} records interruption before any suite completes`, { timeout: TEST_TIMEOUT_MS }, async t => {
    // Arrange
    const fixture = workerFixture(t);
    const { child, done } = startShell(t, fixture.harness('sha256sum() { printf ready >&3; read -r ignored; }'));
    await ready(child);
    // Act
    child.kill(signal);
    await done;
    // Assert
    assert.equal(child.exitCode, status);
    assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), `${status}\n`);
    assert.equal(existsSync(join(fixture.job, 'results.tsv')), false);
  });
}

test('cancellation while waiting for the cache lock replaces old success and reaps the waiter', { timeout: TEST_TIMEOUT_MS }, async t => {
  // Arrange
  const fixture = workerFixture(t), lock = join(fixture.cache, 'worker.lock');
  const holderPath = join(fixture.root, 'holder.sh');
  writeFileSync(holderPath, `exec 9>${quote(lock)}\nflock 9\nprintf ready >&3\nread -r release\n`);
  const holder = startShell(t, holderPath); await ready(holder.child);
  writeFileSync(join(fixture.job, 'exit-code'), '0\n'); writeFileSync(join(fixture.job, 'results.tsv'), 'old\t0\n');
  const waiter = join(fixture.root, 'waiter.pid');
  const worker = startShell(t, fixture.harness(`flock() { printf '%s\\n' "$BASHPID" > ${quote(waiter)}; printf ready >&3; exec /usr/bin/flock "$@"; }`));
  await ready(worker.child);
  assert.equal(existsSync(join(fixture.job, 'exit-code')), false);
  assert.equal(existsSync(join(fixture.job, 'results.tsv')), false);
  // Act
  worker.child.kill('SIGTERM'); await worker.done;
  // Assert
  assert.equal(worker.child.exitCode, 143);
  assert.equal(readFileSync(join(fixture.job, 'exit-code'), 'utf8'), '143\n');
  assert.throws(() => process.kill(Number(readFileSync(waiter, 'utf8')), 0), /ESRCH/);
  holder.child.stdin!.end('release\n'); await holder.done;
});

test('invalid worker invocation explains the expected mode and platform', t => {
  // Arrange / Act / Assert
  for (const args of [[], ['invalid']]) {
    const invalid = spawnSync('bash', [workerScript, ...args], { encoding: 'utf8' });
    assert.equal(invalid.status, 2); assert.match(invalid.stderr, /Usage:.*build\|run/);
  }
  const fixture = workerFixture(t), environment = join(fixture.root, 'bash-env');
  writeFileSync(environment, 'uname() { echo unsupported; }\n');
  const wrongPlatform = spawnSync('bash', [workerScript, 'run'], { encoding: 'utf8', env: { ...process.env, BASH_ENV: environment } });
  assert.notEqual(wrongPlatform.status, 0); assert.match(wrongPlatform.stderr, /requires Linux aarch64/);
  const installer = fileURLToPath(new URL('./install-nextest.sh', import.meta.url));
  const wrongInstallerPlatform = spawnSync('bash', [installer], { encoding: 'utf8', env: { ...process.env, BASH_ENV: environment } });
  assert.notEqual(wrongInstallerPlatform.status, 0); assert.match(wrongInstallerPlatform.stderr, /requires Linux aarch64/);
});

for (const problem of ['node-version', 'compose-plugin']) {
  test(`build explains ${problem} failure before preparing source or installing tools`, t => {
    // Arrange
    const fixture = workerFixture(t), prepare = join(fixture.root, 'prepared'), script = join(fixture.root, 'build.sh');
    mkdirSync(join(fixture.job, 'source')); writeFileSync(join(fixture.job, 'source/.node-version'), '1.2.3\n');
    writeFileSync(script, `source ${quote(workerScript)}
compose_plugin=${quote(join(fixture.root, 'missing-compose'))}
node() { if [[ $1 == --version ]]; then echo ${problem === 'node-version' ? 'v0.0.0' : 'v1.2.3'}; else touch ${quote(prepare)}; fi; }
rustc() { :; }; cargo() { :; }; docker() { :; }
run_worker build ${quote(fixture.job)} ${quote(fixture.cache)}
`);
    // Act
    const result = runShell(script);
    // Assert
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, problem === 'node-version' ? /Node version v0\.0\.0; expected v1\.2\.3/ : /Missing Docker Compose plugin:/);
    assert.equal(existsSync(prepare), false);
  });
}
