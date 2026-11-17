import { strict as assert } from 'node:assert';
import { createHash, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { access, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as pause } from 'node:timers/promises';
import { capabilities, execution, holoscan, launch, workflow } from '../../../console/server/src/native.js';
import { doramcap } from '../../../console/server/src/generated/console.js';
import { sourceState } from '../../../console/server/src/records/source.js';
import { exportFileChanges } from '../../../console/server/src/records/file-changes.js';

const [cli, source, store, output, monitorImage, scenario = 'complete', doraImage, recoverFrom] = process.argv.slice(2);
assert.ok([cli, source, store, output].every(path => path && isAbsolute(path)) && monitorImage,
  'usage: tsx system-real.mts /absolute/cli /absolute/source /absolute/builds /absolute/output cached-monitor-image [complete|stop|cancel-system|start-fail|system-fail|dora-stop] [cached-dora-image] [/absolute/failed-case-output]');
assert.ok(['complete', 'stop', 'cancel-system', 'start-fail', 'system-fail', 'dora-stop'].includes(scenario));
const doraStop = scenario === 'dora-stop';
const doraRoles = doraStop ? ['dora-coordinator', 'dora-daemon'] : [];
if (doraStop) assert.ok(doraImage, 'dora-stop requires an installed cached Dora image');
const partial = ['cancel-system', 'start-fail'].includes(scenario);
const recordingGood = !['start-fail', 'system-fail'].includes(scenario);
const exec = promisify(execFile);
const POLL_MS = 20, CASE_MS = 180_000;
assert.match((await exec('docker', ['image', 'inspect', '--format', '{{.Id}}', monitorImage])).stdout.trim(), /^sha256:/, 'reuse an installed monitor image');
if (doraStop) assert.match((await exec('docker', ['image', 'inspect', '--format', '{{.Id}}', doraImage!])).stdout.trim(), /^sha256:/, 'reuse an installed Dora image');
await mkdir(output, { recursive: true, mode: 0o700 });
const doraProjects: doramcap.compose.v1.ComposeProject.$Properties[] = [];
for (const role of doraRoles) {
  const override = join(output, `${role}-image.json`);
  await writeFile(override, JSON.stringify({ services: { [role]: { image: doraImage } } }));
  doraProjects.push({ key: role, directory: fileURLToPath(new URL(`../../workload-docker/deploy/${role}/`, import.meta.url)),
    files: ['compose.yaml', override], manifest: `${role}.pbtxt`, removeVolumes: true });
}
await mkdir(join(output, 'runs'), { recursive: true, mode: 0o700 });
await mkdir(join(output, 'reviews'), { recursive: true, mode: 0o700 });
const data = join(output, 'shared-data');
await mkdir(data, { recursive: true, mode: 0o755 });
const systemDirectory = join(output, 'monitor');
await mkdir(systemDirectory, { recursive: true, mode: 0o700 });
const deployment = fileURLToPath(new URL('../../../apps/system-monitor/deploy/', import.meta.url));
const originalRecipe = await readFile(join(deployment, 'recording.yaml'), 'utf8');
const recipe = scenario === 'start-fail' ? originalRecipe.replace('command: ["--config", "/config/monitor.pbtxt"]', 'entrypoint: ["/bin/sh", "-c"]\n    command: ["exit 17"]') : originalRecipe;
if (scenario === 'start-fail') assert.notEqual(recipe, originalRecipe);
await writeFile(join(systemDirectory, 'monitor.yaml'), recipe + `      - type: bind\n        source: ${JSON.stringify(data)}\n        target: /shared\n`);
await writeFile(join(systemDirectory, 'recording.pbtxt'), await readFile(join(deployment, 'recording.pbtxt')));
const initial = await sourceState(source);
assert.equal(initial.sourceState, launch.SourceState.SOURCE_STATE_CLEAN);
const id = `m6-${randomUUID().replaceAll('-', '')}`;
const artifacts = join(output, 'runs', id), reviewArtifacts = join(output, 'reviews', id);
const selection = { application: 'first_pipeline', language: 'python', mode: 'finite' };
const request = (directory: string) => execution.RunRequest.create({
  run: {
    id, repositoryRoot: source, artifacts: directory, readinessSeconds: 30, stopSeconds: 15,
    hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
    writePolicy: doramcap.filesystem.v1.WritePolicy.WRITE_POLICY_DISCARD,
    systemApps: ['monitor'], mcapApps: ['monitor'], doraApps: doraRoles,
    recordings: [{ stream: join(directory, 'cpu.pb'), recording: join(directory, 'cpu.mcap') }],
  },
  holoscan: {
    plan: { cli, selection, extraDataFolders: [data], systemProjects: [{
      key: 'monitor', directory: systemDirectory, files: ['monitor.yaml'],
      environment: { DORAMCAP_MONITOR_IMAGE: monitorImage, DORAMCAP_ARTIFACTS: directory },
      removeVolumes: true,
    }, ...doraProjects] },
    currentSource: initial, buildStoreRoot: store,
  },
});
// Retain failed facts before the next admitted Start resets the same repository's old resources.
const digest = async (path: string) => createHash('sha256').update(await readFile(path)).digest('hex');
let previous: { artifacts: string; hashes: Record<string, string>; containers: string[]; scope: string; helper: string } | undefined;
if (recoverFrom) {
  assert.ok(isAbsolute(recoverFrom));
  const results = (await readdir(recoverFrom)).filter(name => name.endsWith('-result.json'));
  assert.equal(results.length, 1, 'one retained failed case is required');
  const failed = holoscan.HoloscanRunReport.fromObject(JSON.parse(await readFile(join(recoverFrom, results[0]), 'utf8')).holoscanReport);
  assert.equal(failed.cleanup!.verified, false, 'recovery starts from explicit failed cleanup');
  const oldArtifacts = join(recoverFrom, 'runs', failed.id);
  const files = ['holoscan-run.pb', 'holoscan-stdout.log', 'holoscan-stderr.log', 'cpu.pb', 'cpu.mcap'];
  const hashes = Object.fromEntries(await Promise.all(files.map(async name => [name, await digest(join(oldArtifacts, name))])));
  const containers = (await exec('docker', ['ps', '-aq', '--filter', `label=io.doramcap.artifacts=${oldArtifacts}`])).stdout.trim().split('\n').filter(Boolean);
  assert.ok(containers.length, 'retained failed resources exist before next admission');
  const info = JSON.parse((await exec('docker', ['inspect', failed.containerId])).stdout)[0];
  const scope = info.Config.Labels['io.doramcap.controller'];
  const helper = holoscan.CliProcessOwnership.decode(await readFile(join(scope, 'holoscan-cli-process.pb')));
  previous = { artifacts: oldArtifacts, hashes, containers, scope, helper: helper.helperPath };
}
const owner = await capabilities.open('/var/run/docker.sock', 1);
try {
  const inspected = await owner.inspect(request(reviewArtifacts));
  const review = holoscan.Preparation.fromObject(inspected.holoscan!);
  assert.deepEqual(review.rejections, [], review.rejections.join('; '));
  assert.ok(review.snapshot?.systemConfiguration?.manifestSha256);
  assert.ok(inspected.plan?.name);
  assert.ok(inspected.writableFolders.some(folder => folder.source === data));
  await assert.rejects(access(artifacts), { code: 'ENOENT' }, 'Review leaves fresh execution artifacts absent');
  await writeFile(join(output, `${id}-review.json`), JSON.stringify(inspected, null, 2), { mode: 0o600 });
  await owner.start(execution.StartRequest.create({ run: request(artifacts), holoscanExpected: review.snapshot! }));
  let update = owner.poll(), wrote = false, stopped = false;
  const events: typeof update.events = [];
  const deadline = Date.now() + CASE_MS;
  while (!update.result) {
    events.push(...update.events);
    if (scenario === 'cancel-system' && !stopped && update.events.some(event => event.step === 'system-apps.cgroups' && event.state === workflow.Event.State.SUCCEEDED)) {
      await assert.rejects(access(join(artifacts, 'holoscan-container.cid')), { code: 'ENOENT' });
      await owner.stop(id); stopped = true;
    }
    if (!partial && update.ready && !wrote) {
      const cid = (await readFile(join(artifacts, 'holoscan-container.cid'), 'utf8')).trim();
      const listed: string[] = (await exec('docker', ['ps', '-q', '--filter', `label=com.docker.compose.project=${inspected.plan!.name}`, '--filter', 'label=com.docker.compose.service=cpu-monitor'])).stdout.trim().split('\n').filter(Boolean);
      assert.equal(listed.length, 1, 'the one actual system monitor is running');
      const monitor = listed[0];
      if (doraStop) for (const role of doraRoles) {
        const ids: string[] = (await exec('docker', ['ps', '-q', '--filter', `label=com.docker.compose.project=${inspected.plan!.name}`,
          '--filter', `label=com.docker.compose.service=${role}`])).stdout.trim().split('\n').filter(Boolean);
        assert.equal(ids.length, 1, `one actual ${role} is running before Stop`);
        assert.equal(JSON.parse((await exec('docker', ['inspect', '--format', '{{json .State}}', ids[0]])).stdout).Running, true);
      }
      await exec('docker', ['exec', monitor, '/bin/sh', '-c', 'printf system > /shared/system-write']);
      await exec('docker', ['exec', cid, 'python3', '-c', 'from pathlib import Path; p=Path("/workspace/volumes/shared-data"); assert (p/"system-write").read_text()=="system"; (p/"native-write").write_text("native")']);
      assert.equal((await exec('docker', ['exec', monitor, '/bin/cat', '/shared/native-write'])).stdout, 'native');
      wrote = true;
      if (scenario === 'stop' || doraStop || scenario === 'system-fail') {
        assert.equal(JSON.parse((await exec('docker', ['inspect', '--format', '{{json .State}}', cid])).stdout).Running, true, 'failure/Stop occurs while the native application is live');
        if (scenario === 'stop' || doraStop) { await owner.stop(id); stopped = true; }
        else await exec('docker', ['kill', '--signal', 'KILL', monitor]);
      }
    }
    assert.ok(Date.now() < deadline, 'combined native/system case deadline expired');
    await pause(POLL_MS);
    update = owner.poll();
  }
  events.push(...update.events);
  await writeFile(join(output, `${id}-events.json`), JSON.stringify(events, null, 2), { mode: 0o600 });
  await writeFile(join(output, `${id}-result.json`), JSON.stringify(update.result, null, 2), { mode: 0o600 });
  const report = holoscan.HoloscanRunReport.fromObject(update.result.holoscanReport!);
  console.log(JSON.stringify({ phase: 'terminal', scenario, id, executionFailed: !!report.executionError,
    writersStopped: report.stopped?.writersStopped, evidenceDurable: report.stopped?.evidenceDurable,
    cleanupVerified: report.cleanup?.verified, recordingFinalized: report.permit?.recordingFinalized,
    systemStopped: report.systemShutdown?.stopped }));
  if (scenario === 'start-fail' || scenario === 'system-fail') assert.ok(report.executionError, 'real system failure becomes execution failure');
  else assert.equal(report.executionError, '', report.diagnostics.join('; '));
  assert.equal(report.reportError, '');
  assert.equal(report.stopped?.writersStopped, true);
  assert.equal(report.stopped?.evidenceDurable, true);
  assert.equal(report.cleanup?.verified, true);
  assert.equal(report.systemShutdown?.stopped, true);
  assert.equal(report.systemShutdown.exits!.length, 1 + doraRoles.length);
  const monitorExit = report.systemShutdown.exits!.find(exit => exit.service === 'cpu-monitor')!;
  assert.ok(monitorExit);
  assert.equal(monitorExit.killed, scenario === 'system-fail');
  if (scenario === 'start-fail') assert.equal(monitorExit.exitCode, 17);
  assert.equal(report.permit?.recordingFinalized, recordingGood);
  if (!partial) assert.ok(wrote, 'both actual containers used the shared writable view');
  if (scenario === 'stop' || doraStop || scenario === 'cancel-system') assert.ok(stopped);
  const start = (step: string) => events.findIndex(event => event.step === step && event.state === workflow.Event.State.STARTED);
  const firstSystem = events.findIndex(event => event.step?.endsWith('StartCompose') && event.state === workflow.Event.State.STARTED);
  assert.ok(firstSystem >= 0);
  if (doraStop) {
    const runtimeReady = events.findIndex(event => event.step?.endsWith('StartDoraRuntime') && event.state === workflow.Event.State.SUCCEEDED);
    assert.ok(runtimeReady >= 0 && runtimeReady < firstSystem, 'actual Dora readiness precedes monitor startup');
    assert.ok(events.some(event => event.step?.endsWith('StopDoraRuntime') && event.state === workflow.Event.State.SUCCEEDED));
    assert.ok(doraRoles.every(role => report.systemShutdown!.exits!.some(exit => exit.service === role && !exit.killed)));
  }
  if (partial) {
    assert.equal(start('holoscan.launch-application'), -1, 'cancelled after real system placement and before native helper preparation');
    assert.equal(report.containerId, '');
  } else assert.ok(start('holoscan.launch-application') > firstSystem, 'systems start before the native application');
  if (scenario === 'start-fail') assert.ok(events.some(event => event.step === 'system-apps.cgroups' && event.state === workflow.Event.State.SUCCEEDED), 'shared group removal succeeds even before native/helper placement');
  else {
    const group = launch.SystemAppCgroup.decode(await readFile(join(artifacts, 'system-app-cgroup.pb')));
    await assert.rejects(access(group.path), { code: 'ENOENT' }, 'shared accounting hierarchy is removed even without a native helper');
  }
  const mcap = scenario === 'start-fail' ? Buffer.alloc(0) : await readFile(join(artifacts, 'cpu.mcap'));
  if (recordingGood) {
    assert.ok(mcap.length >= 45);
    assert.equal(mcap.subarray(0, 8).toString('hex'), '894d434150300d0a');
    assert.ok(mcap.subarray(-8).equals(mcap.subarray(0, 8)), 'real recording retains terminal magic');
    assert.ok((await readFile(join(artifacts, 'cpu.pb'))).length > 0);
  }
  const dockerEvents = (await readFile(join(artifacts, 'holoscan-events.jsonl'), 'utf8')).trim().split('\n').map(line => JSON.parse(line));
  const action = (event: any) => event.Action ?? event.action ?? event.status;
  const actor = (event: any) => event.Actor ?? event.actor ?? {};
  const actorId = (event: any) => actor(event).ID ?? actor(event).Id ?? actor(event).id ?? event.id;
  const systemEvent = (event: any) => (actor(event).Attributes ?? actor(event).attributes ?? {})['com.docker.compose.service'] === monitorExit.service;
  const nativeDie = dockerEvents.findIndex(event => action(event) === 'die' && actorId(event) === report.containerId);
  const systemDie = dockerEvents.findIndex(event => action(event) === 'die' && systemEvent(event));
  if (!partial && scenario !== 'system-fail') assert.ok(nativeDie >= 0 && systemDie > nativeDie, 'actual native process exits before the system monitor');
  const doraDeaths = doraRoles.map(role => dockerEvents.findIndex(event => action(event) === 'die'
    && (actor(event).Attributes ?? actor(event).attributes ?? {})['com.docker.compose.service'] === role));
  if (doraStop) assert.ok(doraDeaths.every(index => index > systemDie), 'actual native die precedes monitor die, then both Dora containers die');
  assert.ok(dockerEvents.some(event => action(event) === 'create' && systemEvent(event)), 'one retained event capture includes system startup');
  const rows: Record<string, any>[] = [];
  for await (const line of exportFileChanges(artifacts, report.fileChanges!)) rows.push(JSON.parse(line.toString()));
  for (const path of partial ? [] : ['system-write', 'native-write']) {
    const row = rows.find(row => row.type === 'change' && row.pathDisplay === path);
    assert.ok(row, `${path} retained in the one combined change list`);
    assert.equal(row.appScope, 'APP_SCOPE_USER_AND_SYSTEM');
    assert.equal(row.sourceKind, 'SOURCE_KIND_OVERLAY');
    assert.ok(row.consumers.some((consumer: string) => consumer.includes('holoscan:first_pipeline')));
    assert.ok(row.consumers.some((consumer: string) => consumer.includes('cpu-monitor')));
    await assert.rejects(access(join(data, path)), { code: 'ENOENT' }, 'Discard preserves the canonical original');
  }
  const saved = holoscan.HoloscanRunReport.decode(await readFile(join(artifacts, 'holoscan-run.pb')));
  assert.deepEqual(saved.toJSON(), report.toJSON());
  if (report.containerId) await assert.rejects(exec('docker', ['inspect', report.containerId]), /no such/i);
  assert.equal((await exec('docker', ['ps', '-aq', '--filter', `label=com.docker.compose.project=${inspected.plan!.name}`])).stdout.trim(), '');
  assert.deepEqual(await sourceState(source), initial);
  if (previous) {
    for (const cid of previous.containers) await assert.rejects(exec('docker', ['inspect', cid]), /no such/i);
    for (const path of [previous.helper, join(previous.scope, 'holoscan-cli-process.pb'), join(previous.scope, 'overlays')])
      await assert.rejects(access(path), { code: 'ENOENT' });
    assert.ok(!(await readFile('/proc/self/mountinfo', 'utf8')).split('\n').some(line => line.split(' ')[4]?.startsWith(`${previous.scope}/overlays/`)));
    for (const [name, hash] of Object.entries(previous.hashes)) assert.equal(await digest(join(previous.artifacts, name)), hash, 'reset preserves failed evidence bytes');
  }
  const summary = { scenario, id, verified: true, artifacts, previousRecovery: previous,
 doraRuntimeServices: doraRoles, nativeDie, monitorDie: systemDie, doraDeaths, mixedWrites: wrote ? 2 : 0, recordingBytes: mcap.length,
    nativeExit: report.containerExitCode?.toString(), nativeCliSignal: report.cliSignal,
    systemExits: report.systemShutdown!.exits!.map(exit => ({ service: exit.service, code: exit.exitCode, killed: exit.killed, oomKilled: exit.oomKilled })),
    executionFailed: !!report.executionError, writersStopped: report.stopped?.writersStopped, evidenceDurable: report.stopped?.evidenceDurable,
    recordingFinalized: report.permit?.recordingFinalized, cleanupVerified: report.cleanup?.verified, eventStreamHealthy: report.eventStreamHealthy };
  await writeFile(join(output, `${id}-summary.json`), JSON.stringify(summary, null, 2), { mode: 0o600 });
  console.log(JSON.stringify(summary));
} finally { await owner.close(); }
