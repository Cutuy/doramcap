/** Real packaged Console acceptance. Uses only explicitly supplied local images. */
import { strict as assert } from 'node:assert';
import { randomBytes } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';
import { Readable } from 'node:stream';
import { setTimeout as delay } from 'node:timers/promises';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const packageDir = resolve(process.env.CONSOLE_PACKAGE ?? `${root}/console/package`);
const socket = process.env.CONSOLE_DOCKER_SOCKET ?? '/var/run/docker.sock';
const monitor = process.env.CONSOLE_MONITOR_IMAGE;
const worker = process.env.CONSOLE_WORKER_IMAGE ?? 'node:22-bookworm-slim';
const verifier = process.env.CONSOLE_MCAP_VERIFY ?? `${root}/target/ci/examples/verify_mcap`;
if (!monitor) throw new Error('CONSOLE_MONITOR_IMAGE must identify a prebuilt native CPU monitor image');
const output = process.env.CONSOLE_ACCEPTANCE_OUTPUT ? resolve(process.env.CONSOLE_ACCEPTANCE_OUTPUT) : await mkdtemp(join(tmpdir(), 'console-acceptance-'));
await mkdir(output, { recursive: true, mode: 0o700 });
const project = join(output, 'project'); const data = join(output, 'data');
await mkdir(project, { recursive: true, mode: 0o700 });
const compose = await readFile(`${root}/console/tests/fixtures/basic/compose.yaml`, 'utf8');
const envText = `CONSOLE_WORKER_IMAGE=${worker}\nCONSOLE_TEST_VALUE=before\n`;
await writeFile(join(project, 'compose.yaml'), compose);
await writeFile(join(project, '.env'), envText);
const secret = randomBytes(24).toString('hex');
const secretPath = join(output, 'secret'); await writeFile(secretPath, secret, { mode: 0o600 });
const probe = createServer(); await new Promise<void>(ok => probe.listen(0, '127.0.0.1', ok));
const address = probe.address(); assert.ok(address && typeof address !== 'string');
const port = address.port; await new Promise<void>(ok => probe.close(() => ok()));
const origin = `http://127.0.0.1:${port}`;
const quote = (value: string) => JSON.stringify(value);
const configPath = join(output, 'console.pbtxt');
const configText = (dataRoot: string) => `listen_address: ${quote(`127.0.0.1:${port}`)}\npublic_origin: ${quote(origin)}\nproject_root: ${quote(output)}\ndata_root: ${quote(dataRoot)}\ndocker_socket: ${quote(socket)}\naccess_secret_file: ${quote(secretPath)}\nsystem_apps { cpu_monitor { image: ${quote(monitor)} } }\n`;
await writeFile(configPath, configText(data), { mode: 0o600 });
const auth = `Basic ${Buffer.from(`console:${secret}`).toString('base64')}`;
const stdout = createWriteStream(join(output, 'server.stdout.log'), { mode: 0o600 });
const stderr = createWriteStream(join(output, 'server.stderr.log'), { mode: 0o600 });
function spawnServer() {
  const process = spawn(processExec, [`${packageDir}/server/dist/main.js`, configPath, `${packageDir}/frontend`], { stdio: ['ignore', 'pipe', 'pipe'] });
  process.stdout!.pipe(stdout, { end: false }); process.stderr!.pipe(stderr, { end: false });
  return process;
}
const processExec = process.execPath;
let child = spawnServer();
let interruptedRun = '';
async function stopServer(signal: NodeJS.Signals) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  child.kill(signal);
  await new Promise<void>((ok, fail) => {
    const timer = setTimeout(() => { child.kill('SIGKILL'); fail(new Error('server did not shut down')); }, 125_000);
    child.once('exit', () => { clearTimeout(timer); ok(); });
  });
}
function cleanupRun(runId: string) {
  assert.match(runId, /^[a-f0-9]{32}$/);
  const docker = (...args: string[]) => execFileSync('docker', ['--host', `unix://${socket}`, ...args], { encoding: 'utf8' });
  for (const kind of ['container', 'network', 'volume']) {
    const args = kind === 'container' ? ['ps', '-aq'] : [kind, 'ls', '-q'];
    const ids = docker(...args, '--filter', `label=io.doramcap.run=${runId}`).trim().split(/\s+/).filter(Boolean);
    for (const id of ids) docker(...(kind === 'container' ? ['rm', '-fv', id] : [kind, 'rm', id]));
  }
}
async function call(path: string, payload?: object) {
  const response = await fetch(origin + path, { method: payload ? 'POST' : 'GET', headers: { authorization: auth, ...(payload ? { origin, 'content-type': 'application/json' } : {}) }, body: payload ? JSON.stringify(payload) : undefined });
  const value = await response.json() as Record<string, any>;
  return { response, value };
}
async function eventually<T>(work: () => Promise<T>, accept: (value: T) => boolean, ms = 90_000): Promise<T> {
  const end = Date.now() + ms; let last: T | undefined;
  while (Date.now() < end) {
    if (child.exitCode !== null || child.signalCode !== null) throw new Error(`packaged server exited; inspect ${output}/server.stderr.log`);
    try { last = await work(); if (accept(last)) return last; } catch {}
    await delay(250);
  }
  throw new Error(`timeout waiting for Console state; last=${JSON.stringify(last)}`);
}
async function download(path: string, target: string) {
  const response = await fetch(origin + path, { headers: { authorization: auth } });
  assert.equal(response.status, 200);
  await pipeline(Readable.fromWeb(response.body as never), createWriteStream(target, { mode: 0o600 }));
}
try {
  await eventually(() => call('/api/current'), r => r.response.ok, 20_000);
  const captureSessions: string[] = [];
  for (let index = 0; index < 2; index++) {
    const first = await call('/api/prepare', { directory: project, profiles: [] });
    assert.equal(first.response.status, 200); assert.ok(first.value.availableProfiles.includes('extra'));
    const review = await call('/api/prepare', { directory: project, profiles: ['extra'] });
    assert.equal(review.response.status, 200); assert.equal(review.value.services.length, 2);
    const started = await call('/api/start', { preparedId: review.value.preparedId, requestKey: `acceptance-${index}-${randomBytes(8).toString('hex')}` });
    assert.equal(started.response.status, 202); const runId = started.value.runId;
    const observed = await eventually(() => call('/api/current'), r => r.value.phase === 'RUNNING' && r.value.steps.some((step: {step: string}) => step.step.endsWith('ObserveProjects')));
    assert.ok(observed.value.steps.some((step: {step: string}) => step.step.endsWith('PrepareRun')));
    assert.ok(observed.value.steps.some((step: {step: string}) => step.step.endsWith('LaunchProjects')));
    await delay(2000);
    assert.equal((await call('/api/stop', { runId })).response.status, 200);
    const finished = await eventually(() => call('/api/current'), r => ['FINISHED', 'BLOCKED'].includes(r.value.phase));
    assert.equal(finished.value.phase, 'FINISHED');
    assert.equal(finished.value.outcome.recordingFinalized, true); assert.equal(finished.value.outcome.boundaryClean, true);
    const mcap = join(output, `capture-${index}.mcap`);
    await download(`/api/artifacts/${runId}/cpu`, mcap);
    await download(`/api/artifacts/${runId}/report`, join(output, `report-${index}.json`));
    const verified = execFileSync(verifier, [mcap], { encoding: 'utf8' }).trim();
    captureSessions.push(/session=(\S+)/.exec(verified)![1]);
    console.log(`run=${runId} ${verified}`);
  }
  assert.notEqual(captureSessions[0], captureSessions[1]);
  // Graceful server shutdown during capture must await finalization and persist the result.
  const shutdownReview = await call('/api/prepare', { directory: project, profiles: [] });
  assert.equal(shutdownReview.response.status, 200);
  const shutdownStart = await call('/api/start', { preparedId: shutdownReview.value.preparedId, requestKey: 'shutdown-test' });
  assert.equal(shutdownStart.response.status, 202);
  await eventually(() => call('/api/current'), r => r.value.phase === 'RUNNING');
  await stopServer('SIGTERM');
  child = spawnServer();
  const shutdownResult = await eventually(() => call('/api/current'), r => r.response.ok, 20_000);
  assert.equal(shutdownResult.value.runId, shutdownStart.value.runId);
  assert.equal(shutdownResult.value.phase, 'FINISHED');
  assert.equal(shutdownResult.value.outcome.recordingFinalized, true);
  assert.equal(shutdownResult.value.outcome.boundaryClean, true);
  const review = await call('/api/prepare', { directory: project, profiles: [] });
  assert.equal(review.response.status, 200);
  await writeFile(join(project, '.env'), `CONSOLE_WORKER_IMAGE=${worker}\nCONSOLE_TEST_VALUE=after\n`);
  const stale = await call('/api/start', { preparedId: review.value.preparedId, requestKey: 'stale-check' });
  assert.equal(stale.response.status, 409);
  await writeFile(join(project, '.env'), 'CONSOLE_WORKER_IMAGE=console-missing-image:never\n');
  assert.equal((await call('/api/prepare', { directory: project, profiles: [] })).response.status, 400);
  await writeFile(join(project, '.env'), envText);
  // A worker that exits early reports execution failure independently of cleanup.
  const early = compose.replace("trap 'exit 0' TERM INT; while :; do sleep 1; done", 'exit 7');
  assert.notEqual(early, compose);
  await writeFile(join(project, 'compose.yaml'), early);
  const failedReview = await call('/api/prepare', { directory: project, profiles: [] });
  assert.equal(failedReview.response.status, 200);
  const failedStart = await call('/api/start', { preparedId: failedReview.value.preparedId, requestKey: 'early-exit' });
  assert.equal(failedStart.response.status, 202);
  const failed = await eventually(() => call('/api/current'), r => ['FINISHED', 'BLOCKED'].includes(r.value.phase));
  assert.equal(failed.value.outcome.execution, 'Failed');
  assert.equal(failed.value.phase, failed.value.outcome.boundaryClean ? 'FINISHED' : 'BLOCKED');
  if (!failed.value.outcome.boundaryClean) {
    assert.equal((await call('/api/prepare', { directory: project, profiles: [] })).response.status, 409);
  }
  await writeFile(join(project, 'compose.yaml'), compose);
  // A killed server must restore a blocked marker rather than relaunching.
  await stopServer('SIGTERM');
  await writeFile(configPath, configText(join(output, 'crash-data')), { mode: 0o600 });
  child = spawnServer();
  await eventually(() => call('/api/current'), r => r.response.ok, 20_000);
  const crashReview = await call('/api/prepare', { directory: project, profiles: [] });
  assert.equal(crashReview.response.status, 200);
  const crashStart = await call('/api/start', { preparedId: crashReview.value.preparedId, requestKey: 'crash-test' });
  assert.equal(crashStart.response.status, 202);
  interruptedRun = crashStart.value.runId;
  await eventually(() => call('/api/current'), r => r.value.phase === 'RUNNING');
  await stopServer('SIGKILL');
  child = spawnServer();
  const restored = await eventually(() => call('/api/current'), r => r.response.ok, 20_000);
  assert.equal(restored.value.phase, 'BLOCKED');
  assert.equal(restored.value.runId, interruptedRun);
  assert.equal((await call('/api/prepare', { directory: project, profiles: [] })).response.status, 409);
  cleanupRun(interruptedRun);
  interruptedRun = '';
  assert.equal(await readFile(join(project, 'compose.yaml'), 'utf8'), compose);
  assert.equal(await readFile(join(project, '.env'), 'utf8'), envText);
  console.log(`PASS: two captures, stale/missing inputs, early exit and interrupted restart; evidence ${output}`);
} finally {
  try { await stopServer('SIGTERM'); }
  finally {
    if (interruptedRun) cleanupRun(interruptedRun);
    stdout.end(); stderr.end();
  }
}
