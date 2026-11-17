/** Build uses native --no-docker-build with a cached image; CMake/builder execution remains real.
 * Actual controller SIGKILL/recovery and two-owner admission; retains isolated fixture/evidence. */
import { strict as assert } from 'node:assert';
import { createHash, randomUUID } from 'node:crypto';
import { execFile, fork } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { access, cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { request as httpRequest } from 'node:http';
import { basename, dirname, isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { setTimeout as pause } from 'node:timers/promises';
import { execution, holoscan, launch } from '../../../console/server/src/native.js';
import { doramcap } from '../../../console/server/src/generated/console.js';
import { sourceState } from '../../../console/server/src/records/source.js';

const [cli, fixture, image, output, socket = '/var/run/docker.sock'] = process.argv.slice(2);
assert.ok([cli, fixture, output, socket].every(value => value && isAbsolute(value)) && image,
  'usage: tsx recovery-real.mts /absolute/holoscan /absolute/first_pipeline-fixture cached-image-id /absolute/new-output [socket]');
const OBSERVE_MS = 20, CASE_MS = 120_000, START_SECONDS = 30, STOP_SECONDS = 5;
const prefix = `m7-${randomUUID()}`, project = join(output, 'source'), store = join(output, 'state/builds');
const controllers = new Set<Awaited<ReturnType<typeof controller>>>();
const cases: Record<string, unknown> = {};
let sentinel = '', scope = '';
await mkdir(output, { mode: 0o700 });
await mkdir(join(output, 'state'), { mode: 0o700 });
await mkdir(join(output, 'runs'), { mode: 0o700 });
await mkdir(join(output, 'data'), { mode: 0o755 });

async function engine(method: string, path: string, input?: unknown): Promise<any> {
  return new Promise((resolve, reject) => {
    const bytes = input === undefined ? undefined : Buffer.from(JSON.stringify(input));
    const request = httpRequest({ socketPath: socket, path, method, headers: bytes ? { 'Content-Type': 'application/json', 'Content-Length': bytes.length } : {} }, response => {
      const chunks: Buffer[] = [];
      response.on('data', chunk => chunks.push(chunk));
      response.on('end', () => {
        const body = Buffer.concat(chunks).toString();
        if ((response.statusCode ?? 500) >= 300) { reject(Object.assign(new Error(`${method} ${path}: ${response.statusCode} ${body}`), { status: response.statusCode })); return; }
        try { resolve(body ? JSON.parse(body) : null); } catch (error) { reject(error); }
      });
    });
    request.on('error', reject); request.end(bytes);
  });
}
async function wait<T>(label: string, observe: () => Promise<T | false>): Promise<T> {
  const deadline = Date.now() + CASE_MS;
  while (Date.now() < deadline) { const result = await observe(); if (result !== false) return result; await pause(OBSERVE_MS); }
  throw new Error(`${label}: observation deadline exceeded`);
}
async function controller(label: string, environment: NodeJS.ProcessEnv = {}) {
  const child = fork(fileURLToPath(new URL('./fixtures/recovery-controller.mts', import.meta.url)), [socket], { stdio: ['ignore', 'pipe', 'pipe', 'ipc'], env: { ...process.env, ...environment } });
  child.stdout!.pipe(createWriteStream(join(output, `${label}.stdout.log`), { mode: 0o600 }));
  child.stderr!.pipe(createWriteStream(join(output, `${label}.stderr.log`), { mode: 0o600 }));
  const pending = new Map<number, { resolve(value: any): void; reject(error: Error): void }>(); let next = 0;
  let started!: () => void, failed!: (error: Error) => void; const ready = new Promise<void>((resolve, reject) => { started = resolve; failed = reject; });
  let exited!: (value: { code: number | null; signal: string | null }) => void;
  const exit = new Promise<{ code: number | null; signal: string | null }>(resolve => exited = resolve);
  child.on('message', (message: any) => { if (message.ready) started(); else { const reply = pending.get(message.id); pending.delete(message.id); if (message.error) reply?.reject(new Error(message.error)); else reply?.resolve(message.value); } });
  child.on('error', failed);
  child.on('exit', (code, signal) => { failed(new Error(`controller exited before ready: ${signal ?? code}`)); for (const waiter of pending.values()) waiter.reject(new Error(`controller exited ${signal ?? code}`)); pending.clear(); exited({ code, signal }); });
  await ready;
  const result = {
    pid: child.pid!,
    call(operation: string, input?: unknown): Promise<any> { return new Promise((resolve, reject) => { const id = ++next; pending.set(id, { resolve, reject }); child.send({ id, operation, input }); }); },
    async kill() { child.kill('SIGKILL'); const ended = await exit; assert.equal(ended.signal, 'SIGKILL'); controllers.delete(result); },
    async close() { if (child.exitCode === null && child.signalCode === null) await result.call('close'); await exit; controllers.delete(result); },
  };
  controllers.add(result); return result;
}
const selection = (mode = 'finite') => holoscan.Selection.create({ application: 'first_pipeline', language: 'python', mode });
const key = (mode = 'finite') => holoscan.BuildStoreRequest.create({ root: store, key: { repositoryRoot: project, selection: selection(mode) } });
const lookup = async (owner: Awaited<ReturnType<typeof controller>>, mode = 'finite') => holoscan.BuildLookup.fromObject(await owner.call('lookup', key(mode).toJSON()));
const fingerprint = (value: Uint8Array) => createHash('sha256').update(value).digest('hex');
async function evidence(value: doramcap.holoscan.v1.BuildLookup) {
  return { latest: fingerprint(holoscan.BuildAttempt.encode(value.latest!).finish()), stdout: fingerprint(await readFile(value.logs!.stdout!)), stderr: fingerprint(await readFile(value.logs!.stderr!)) };
}
const shim = join(output, 'native-cli.mjs'), observed = join(output, 'cli-observed.json'), startupGate = join(output, 'hold-startup');
async function build(owner: Awaited<ReturnType<typeof controller>>, mode = 'finite') {
  const request = holoscan.BuildRequest.create({ attempt: { id: randomUUID(), store: key(mode), source: await sourceState(project) }, cli: shim,
    readinessSeconds: START_SECONDS, stopSeconds: STOP_SECONDS, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH });
  return holoscan.BuildOutcome.fromObject(await owner.call('build', request.toJSON()));
}
async function reviewed(owner: Awaited<ReturnType<typeof controller>>, mode = 'finite') {
  const id = `m7-${randomUUID()}`, artifacts = join(output, 'runs', id);
  const request = execution.RunRequest.create({ run: { id, repositoryRoot: project, artifacts, readinessSeconds: START_SECONDS, stopSeconds: STOP_SECONDS,
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH, writePolicy: doramcap.filesystem.v1.WritePolicy.WRITE_POLICY_DISCARD },
    holoscan: { plan: { cli: shim, selection: selection(mode), extraDataFolders: [join(output, 'data')] }, currentSource: await sourceState(project), buildStoreRoot: store } });
  const review = execution.Inspection.fromObject(await owner.call('inspect', request.toJSON())).holoscan!;
  assert.deepEqual(review.rejections, [], (review.rejections ?? []).join('; '));
  return { id, artifacts, request: execution.StartRequest.create({ run: request, holoscanExpected: review.snapshot! }) };
}
async function completed(owner: Awaited<ReturnType<typeof controller>>) {
  const result = await wait('native run completion', async () => { const update = execution.ExecutionUpdate.fromObject(await owner.call('poll')); return update.result || false; });
  assert.equal(result.holoscanReport!.cleanup!.verified, true, JSON.stringify(result));
  assert.equal(Number(result.holoscanReport!.containerExitCode), 0, JSON.stringify(result)); return result;
}
async function owned() { return (await engine('GET', '/containers/json?all=true')).filter((value: any) => value.Labels?.['io.doramcap.controller'] === scope); }
async function unrelated() { assert.equal((await engine('GET', `/containers/${sentinel}/json`)).State.Running, true, 'unrelated sentinel remains running'); }
async function resetVerified(helper: doramcap.holoscan.v1.CliProcessOwnership) {
  assert.deepEqual(await owned(), [], 'all prior owned containers removed');
  await assert.rejects(access(join(scope, 'holoscan-cli-process.pb')), { code: 'ENOENT' });
  await assert.rejects(access(join(scope, 'overlays')), { code: 'ENOENT' });
  await assert.rejects(access(helper.helperPath), { code: 'ENOENT' });
  assert.ok(!(await readFile('/proc/self/mountinfo', 'utf8')).split('\n').some(line => line.split(' ')[4]?.startsWith(`${scope}/overlays/`)));
  await unrelated();
}
async function killAndCheck(owner: Awaited<ReturnType<typeof controller>>) {
  const cliPid = Number(JSON.parse(await readFile(observed, 'utf8')).pid);
  const processIdentity = (stat: string) => { const fields = stat.slice(stat.lastIndexOf(') ') + 2).split(' '); return { state: fields[0], started: fields[19] }; };
  const identity = processIdentity(await readFile(`/proc/${cliPid}/stat`, 'utf8'));
  await owner.kill();
  await wait('parent-death CLI exit', async () => { const current = await readFile(`/proc/${cliPid}/stat`, 'utf8').catch(() => ''); return !current || processIdentity(current).state === 'Z' || processIdentity(current).started !== identity.started ? true : false; });
  return cliPid;
}

try {
  // Arrange: separate native namespace and aliases of an existing cached image; no image build/pull.
  const info = await engine('GET', `/images/${encodeURIComponent(image)}/json`); assert.equal(info.Id, image);
  for (const [repo, tag] of [[`${prefix}-first_pipeline`, 'main'], [`${prefix}-first_pipeline`, 'latest'], [prefix, 'ngc-v5.0.0-cuda13']])
    await engine('POST', `/images/${encodeURIComponent(image)}/tag?repo=${encodeURIComponent(repo!)}&tag=${tag}`);
  await cp(fixture, project, { recursive: true, filter: path => !['build', '.git'].includes(basename(path)) });
  const appPath = join(project, 'applications/first_pipeline/first_pipeline.py'), officialHash = fingerprint(await readFile(appPath));
  assert.equal(officialHash, 'c7f06121a93ee754b1843144f2bed7ae1fe810fd88c28abe298f381577645fcd');
  const metadataPath = join(project, 'applications/first_pipeline/metadata.json'), metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  metadata.application.modes.repeat = { run: { command: "/bin/sh -c 'while true; do python3 -u <holohub_app_bin>/first_pipeline.py || exit; done'", workdir: 'holohub_app_bin' } };
  metadata.application.modes['build-loss'] = metadata.application.modes.finite;
  await writeFile(metadataPath, JSON.stringify(metadata));
  await writeFile(join(project, 'metadata.json'), JSON.stringify({ $schema: 'urn:holoscan:module:v2', module: { name: prefix, version: '1.0.0', support_level: 'community', description: 'Controller loss qualification', authors: [{ name: 'Fixture', affiliation: 'Fixture' }], license: 'Apache-2.0', language: ['Python'], namespace: { python: 'holoscan_m7' }, holoscan_sdk: { minimum_required_version: '5.0.0', tested_versions: ['5.0.0'] }, platforms: ['x86_64'], tags: ['fixture'], operator_names: ['Sample'], dependencies: [], dockerfile: 'Dockerfile' } }));
  await writeFile(join(project, 'pyproject.toml'), `[tool.holoscan]\nrepo-prefix=${JSON.stringify(prefix)}\ncontainer-prefix=${JSON.stringify(prefix)}\nworkspace-name="holoscan_m7"\ndocker-run-args=["--init"]\n[tool.holoscan.base-images]\nx86_64="nvcr.io/nvidia/cuda@sha256:cf5ab24c3f5040a0ea5931658874c2158e606bf740f006809aa4bb9d3334016b"\n`);
  const cmake = join(project, 'CMakeLists.txt'); await writeFile(cmake, `if(EXISTS "\${CMAKE_SOURCE_DIR}/.m7-hold-build")\n message(STATUS "M7_BUILD_READY")\n execute_process(COMMAND "\${CMAKE_COMMAND}" -E sleep 300)\nendif()\n${await readFile(cmake, 'utf8')}`);
  await writeFile(join(project, '.gitignore'), '/build/\n/data/\n/.cache/\n/.cupy/\n/.m7-hold-build\n');
  const git = async (...args: string[]) => promisify(execFile)('git', ['-C', project, ...args]);
  await git('init', '-q', '-b', 'main'); await git('add', '.'); await git('-c', 'user.name=Recovery Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'Unchanged SDK graph and native test configuration');
  await writeFile(shim, `#!/usr/bin/node\nimport {existsSync,writeFileSync} from 'node:fs';\nconst args=process.argv.slice(2);\nconst runtime=(args.includes('build')||args.includes('run'))&&!args.includes('--dryrun');\nif(runtime){writeFileSync(${JSON.stringify(observed)},JSON.stringify({pid:process.pid,operation:args.includes('build')?'build':'run'}));if(args.includes('build'))args.splice(args.indexOf('build')+1,0,'--no-docker-build');}\nif(runtime&&args.includes('run')&&existsSync(${JSON.stringify(startupGate)})){setTimeout(()=>process.exit(8),300000);}else{process.execve(${JSON.stringify(cli)},[${JSON.stringify(cli)},...args],process.env);}\n`, { mode: 0o755 });
  sentinel = (await engine('POST', `/containers/create?name=${prefix}-unrelated`, { Image: image, Cmd: ['/bin/sleep', '300'], Labels: { 'io.doramcap.m7.fixture': prefix } })).Id;
  await engine('POST', `/containers/${sentinel}/start`);
  const setup = await controller('setup');
  for (const mode of ['finite', 'repeat']) { const result = await build(setup, mode); assert.ok(result.attempt!.result!.record, JSON.stringify(result.toJSON())); }
  const finiteBefore = await evidence(await lookup(setup));
  await setup.close();

  // Act / Assert: actual Build controller loss, plus Build -> valid independent Run refusal.
  let active = await controller('build-victim'), next = await controller('build-next');
  const startAfterBuild = await reviewed(next);
  await writeFile(join(project, '.m7-hold-build'), '');
  const pendingBuild = build(active, 'build-loss').catch(error => String(error));
  const progress = await wait('real native builder CMake gate', async () => { const update = execution.ExecutionUpdate.fromObject(await active.call('poll')); const progress = update.buildProgress;
    return progress?.logs?.stdout && (await readFile(progress.logs.stdout, 'utf8').catch(() => '')).includes('M7_BUILD_READY') ? progress : false; });
  const builder = (await readFile(join(dirname(progress.logs!.stdout!), 'builder.cid'), 'utf8')).trim();
  let container = await engine('GET', `/containers/${builder}/json`); assert.equal(container.State.Running, true);
  scope = container.Config.Labels['io.doramcap.controller']; assert.ok(isAbsolute(scope));
  const buildHelper = holoscan.CliProcessOwnership.decode(await readFile(join(scope, 'holoscan-cli-process.pb')));
  await assert.rejects(next.call('start', startAfterBuild.request.toJSON()), /another run owns this repository/);
  assert.equal((await engine('GET', `/containers/${builder}/json`)).State.Running, true, 'refused Run preserves competing Build');
  const buildCli = await killAndCheck(active); await pendingBuild;
  const interrupted = await lookup(next, 'build-loss'); assert.equal(interrupted.record, null); assert.equal(interrupted.latest!.result, null);
  const interruptedBefore = await evidence(interrupted); await rm(join(project, '.m7-hold-build'));
  await next.call('start', startAfterBuild.request.toJSON()); await completed(next); await resetVerified(buildHelper);
  assert.deepEqual(await evidence(await lookup(next, 'build-loss')), interruptedBefore);
  assert.deepEqual(await evidence(await lookup(next)), finiteBefore);
  cases.buildLoss = { controllerPid: active.pid, cliPid: buildCli, builder, incomplete: interruptedBefore, buildToRunRefused: true };
  await next.close();

  for (const phase of ['startup', 'running'] as const) {
    // Startup is after owned helper/view creation and before native execution/cidfile.
    active = await controller(`${phase}-victim`); next = await controller(`${phase}-next`);
    const request = await reviewed(active, phase === 'running' ? 'repeat' : 'finite');
    await rm(observed, { force: true });
    await rm(observed, { force: true });
    if (phase === 'startup') await writeFile(startupGate, '');
    const admitted = active.call('start', request.request.toJSON());
    if (phase === 'startup') await wait('startup CLI gate', async () => { try { const marker = JSON.parse(await readFile(observed, 'utf8')); return marker.operation === 'run' && marker.pid && await readFile(join(request.artifacts, 'holoscan-stdout.log')).then(() => true).catch(() => false); } catch { return false; } });
    else await wait('actual running graph', async () => { const update = execution.ExecutionUpdate.fromObject(await active.call('poll')); if (!update.ready) return false;
      const cid = (await readFile(join(request.artifacts, 'holoscan-container.cid'), 'utf8')).trim(); const info = await engine('GET', `/containers/${cid}/json`);
      return info.State.Running && (await readFile(join(request.artifacts, 'holoscan-stdout.log'), 'utf8')).includes('tick ') ? info : false; });
    await admitted;
    const helper = holoscan.CliProcessOwnership.decode(await readFile(join(scope, 'holoscan-cli-process.pb')));
    assert.ok((await readFile('/proc/self/mountinfo', 'utf8')).includes(`${scope}/overlays/`));
    if (phase === 'startup') await assert.rejects(access(join(request.artifacts, 'holoscan-container.cid')), { code: 'ENOENT' });
    else {
      const cid = (await readFile(join(request.artifacts, 'holoscan-container.cid'), 'utf8')).trim();
      const refused = await build(next); assert.match(refused.error, /another run owns this repository/); assert.equal(refused.attempt, null);
      assert.equal((await engine('GET', `/containers/${cid}/json`)).State.Running, true, 'refused Build preserves competing Run');
    }
    const cliPid = await killAndCheck(active);
    await assert.rejects(access(join(request.artifacts, 'holoscan-run.pb')), { code: 'ENOENT' });
    const logPath = join(request.artifacts, 'holoscan-stdout.log'), logHash = fingerprint(await readFile(logPath));
    if (phase === 'startup') await rm(startupGate);
    const fresh = await reviewed(next); await next.call('start', fresh.request.toJSON()); await completed(next); await resetVerified(helper);
    assert.equal(fingerprint(await readFile(logPath)), logHash, 'recovery preserves interrupted CLI evidence');
    assert.deepEqual(await evidence(await lookup(next)), finiteBefore);
    cases[phase] = { controllerPid: active.pid, cliPid, oldArtifacts: request.artifacts, recoveryArtifacts: fresh.artifacts, logHash, runToBuildRefused: phase === 'running' };
    await next.close();
  }
  assert.equal(fingerprint(await readFile(appPath)), officialHash);
  await unrelated();
  // Application-root Build/Run is distinct from Module-root; absent Git remains Unknown.
  const appRoot = join(output, 'application', 'first_pipeline'), appPrefix = `${prefix}-application`;
  await mkdir(dirname(appRoot));
  await cp(join(fixture, 'applications/first_pipeline'), appRoot, { recursive: true });
  await writeFile(join(appRoot, 'CMakeLists.txt'), 'cmake_minimum_required(VERSION 3.24)\nproject(m7_application LANGUAGES NONE)\nconfigure_file(first_pipeline.py first_pipeline.py COPYONLY)\n');
  for (const [repo, tag] of [[`${appPrefix}-first_pipeline`, 'latest'], [appPrefix, 'ngc-v5.0.0-cuda13']])
    await engine('POST', `/images/${encodeURIComponent(image)}/tag?repo=${encodeURIComponent(repo!)}&tag=${tag}`);
  const appOwner = await controller('application', {
    HOLOSCAN_CLI_REPO_PREFIX: appPrefix, HOLOSCAN_CLI_CONTAINER_PREFIX: appPrefix,
    HOLOSCAN_CLI_WORKSPACE_NAME: 'holoscan_m7_application',
    HOLOSCAN_CLI_BASE_IMAGE: 'nvcr.io/nvidia/cuda@sha256:cf5ab24c3f5040a0ea5931658874c2158e606bf740f006809aa4bb9d3334016b',
  });
  const appSource = await sourceState(appRoot);
  assert.equal(appSource.sourceState, launch.SourceState.SOURCE_STATE_UNKNOWN); assert.equal(appSource.sourceCommit, '');
  const appStore = holoscan.BuildStoreRequest.create({ root: store, key: { repositoryRoot: appRoot, selection: selection() } });
  const appBuild = holoscan.BuildOutcome.fromObject(await appOwner.call('build', holoscan.BuildRequest.create({
    attempt: { id: randomUUID(), store: appStore, source: appSource }, cli: shim, readinessSeconds: START_SECONDS, stopSeconds: STOP_SECONDS,
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
  }).toJSON()));
  assert.equal(appBuild.attempt!.result!.record!.imageId, image, JSON.stringify(appBuild));
  const appId = `m7-application-${randomUUID()}`, appArtifacts = join(output, 'runs', appId);
  const appRequest = execution.RunRequest.create({ run: { id: appId, repositoryRoot: appRoot, artifacts: appArtifacts,
    readinessSeconds: START_SECONDS, stopSeconds: STOP_SECONDS, hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
    writePolicy: doramcap.filesystem.v1.WritePolicy.WRITE_POLICY_DISCARD },
    holoscan: { plan: { cli: shim, selection: selection() }, currentSource: appSource, buildStoreRoot: store } });
  const appReview = execution.Inspection.fromObject(await appOwner.call('inspect', appRequest.toJSON())).holoscan!;
  assert.deepEqual(appReview.rejections, []); assert.equal(appReview.build!.state, holoscan.BuildState.BUILD_STATE_UNVERIFIED);
  await appOwner.call('start', execution.StartRequest.create({ run: appRequest, holoscanExpected: appReview.snapshot! }).toJSON());
  const appResult = await completed(appOwner);
  assert.equal(appResult.holoscanReport!.imageId, image);
  assert.equal(fingerprint(await readFile(join(appRoot, 'first_pipeline.py'))), officialHash);
  assert.deepEqual(await sourceState(appRoot), appSource);
  cases.application = { source: appSource, build: appBuild.toJSON(), result: execution.ExecutionResult.fromObject(appResult).toJSON() };
  await appOwner.close();
  cases.status = 'passed';
} finally {
  for (const owner of [...controllers]) await owner.close().catch(error => { cases.closeError = String(error); });
  if (sentinel) { await engine('POST', `/containers/${sentinel}/stop?t=0`).catch(() => {}); await engine('DELETE', `/containers/${sentinel}`).catch(() => {}); }
  await writeFile(join(output, 'report.json'), JSON.stringify({ prefix, image, source: project, scope, cases }, null, 2), { mode: 0o600 });
}
