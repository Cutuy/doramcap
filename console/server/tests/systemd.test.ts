import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { capabilities, execution, launch } from '../src/native.js';
import { releaseConfig } from '../src/release.js';
import { bindOutputs, selectApps, validateApps } from '../src/system-apps.js';
import { systemdPlan } from '../src/systemd.js';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const systemApps = join(repository, 'console/release/system-apps');
const DORA_MEMLOCK_BYTES = 64 * 1024 * 1024;

async function systemdRelease() {
  const text = await readFile(join(repository, 'console/release/console.pbtxt'), 'utf8');
  return releaseConfig(
    capabilities.parseConfig(text.replace('form: COMPOSE', 'form: SYSTEMD binaries: "/opt/doramcap/bin"')),
    '/playground', systemApps, '/state',
  );
}

test('release manifests resolve to a systemd plan with container paths bound', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-systemd-'));
  const project = join(directory, 'project'), artifacts = join(directory, 'runs/r1');
  try {
    await mkdir(project);
    await writeFile(join(project, 'planner.service'), '[Service]\nExecStart=/bin/true\n');
    await writeFile(join(project, 'perception.service'), '[Service]\nExecStart=/bin/true\n');
    await writeFile(join(project, 'compose.yaml'), 'services: {}\n');
    const config = await systemdRelease();
    const apps = selectApps(config.apps, ['cpu-monitor', 'data-recorder', 'live-bridge'], { 'cpu-monitor': 'record', 'data-recorder': 'cpu' });
    // Act
    const plan = await systemdPlan(config, {
      id: 'r1', project, artifacts, workspace: directory, stage: 'execution', apps,
      graph: join(directory, 'graph.json'), relaySettings: join(directory, 'bridge.pbtxt'),
    });
    // Assert
    assert.ok(plan.systemd && !plan.compose);
    assert.deepEqual(plan.run!.doraApps, ['dora-coordinator', 'dora-daemon']);
    assert.equal(plan.run!.dataflowFile, join(directory, 'graph.json'));
    assert.deepEqual(plan.run!.systemApps, ['cpu-monitor', 'data-recorder', 'live-bridge']);
    assert.deepEqual(plan.run!.mcapApps, ['cpu-monitor', 'data-recorder']);
    assert.deepEqual(plan.systemd!.unitFiles, [join(project, 'perception.service'), join(project, 'planner.service')]);
    const unit = (id: string) => plan.systemd!.programs!.find((app) => app.id === id)!;
    assert.deepEqual(plan.systemd!.appUnits, []);
    assert.deepEqual(unit('cpu-monitor').argv, ['/opt/doramcap/bin/system-monitor', '--config', '/config/monitor.pbtxt']);
    assert.equal(Number(unit('cpu-monitor').memlockBytes), 268435456);
    assert.equal(unit('cpu-monitor').killSignal, 2);
    assert.equal(Number(unit('cpu-monitor').stopSeconds), 15);
    for (const id of ['dora-coordinator', 'dora-daemon']) {
      assert.equal(Number(unit(id).memlockBytes), DORA_MEMLOCK_BYTES);
      assert.equal(unit(id).workingDirectory, '/runtime');
      assert.ok(unit(id).binds!.some((bind) =>
        bind.source === join(artifacts, 'dora/runtime') && bind.target === '/runtime' && !bind.readOnly));
    }
    assert.equal(plan.systemd!.binaries, '/opt/doramcap/bin');
    assert.equal(unit('cpu-monitor').id, 'cpu-monitor');
    assert.deepEqual(unit('cpu-monitor').binds!.map(({ source, target, readOnly }) => [source, target, readOnly]), [
      [join(systemApps, 'cpu-monitor/cpu-monitor-record.pbtxt'), '/config/monitor.pbtxt', true],
      [artifacts, '/results', false],
    ]);
    assert.ok(unit('data-recorder').binds!.some((bind) => bind.source === join(systemApps, 'data-recorder/telemetry_descriptor.bin') && bind.target === '/config/telemetry_descriptor.bin'));
    assert.equal(unit('live-bridge').binds![0].source, join(directory, 'bridge.pbtxt'));
    assert.deepEqual(plan.run!.recordings!.map((r) => r.recording), [join(artifacts, 'cpu.mcap'), join(artifacts, 'messages.mcap')]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('native Dora units bind the shared runtime directory', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-native-dora-'));
  try {
    const config = await systemdRelease();
    for (const id of config.doraApps) {
      const app = config.apps.find((app) => app.id === id)!;
      app.process = null;
      app.systemd = { unit: join(repository, 'libs/workload-systemd/tests/fixtures', `${id}.service`) };
    }
    const artifacts = join(directory, 'artifacts');
    // Act
    const request = await systemdPlan(config, {
      id: 'r1', project: directory, artifacts, workspace: directory,
      stage: 'test', apps: [], graph: '',
    });
    // Assert
    assert.deepEqual(request.systemd!.programs, []);
    assert.deepEqual(request.systemd!.appUnits!.map((unit) => unit.id), ['dora-coordinator', 'dora-daemon']);
    for (const unit of request.systemd!.appUnits!) {
      assert.ok(unit.binds!.some((bind) =>
        bind.source === join(artifacts, 'dora/runtime') && bind.target === '/runtime' && !bind.readOnly));
    }
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('systemd needs absolute binaries and a unit file for every selected app', async () => {
  // Arrange
  const template = (text: string) => capabilities.parseConfig(text);
  // Act / Assert: binaries are required.
  assert.throws(() => releaseConfig(template('host_shm_access: PASSTHROUGH form: SYSTEMD'), '/playground', systemApps, '/state'), /binaries must be absolute/);
  // Act / Assert: an inline app without a unit file cannot launch.
  const config = releaseConfig(
    template('form: SYSTEMD binaries: "/opt/bin" apps {id:"inline" input {directory:"cpu-monitor" files:"compose.yaml"}}'),
    '/playground', systemApps, '/state',
  );
  await assert.rejects(
    systemdPlan(config, { id: 'r1', project: tmpdir(), artifacts: '/state/r1', workspace: tmpdir(), stage: 'test', apps: config.apps, graph: '' }),
    /needs a unit file/,
  );
});

test('systemd launch inputs preserve manifest IDs independently of native filenames', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-systemd-ids-'));
  const project = join(directory, 'project'), apps = join(directory, 'apps');
  await mkdir(project);
  await mkdir(apps);
  await writeFile(join(apps, 'app.pbtxt'), 'id:"capture" compose { files:"compose.yaml" } output_targets:"/results" systemd { unit:"worker.service" }');
  await writeFile(join(apps, 'worker.service'), '[Service]\nExecStart=/bin/true\n');
  try {
    const config = releaseConfig(capabilities.parseConfig('host_shm_access:PASSTHROUGH form:SYSTEMD binaries:"/opt/bin" runtime {readiness_seconds:60 stop_seconds:30} apps {manifest:"app.pbtxt"}'), project, apps, '/state');
    // Act
    const request = await systemdPlan(config, {
      id: 'r1', project, artifacts: '/state/r1', workspace: directory,
      stage: 'test', apps: selectApps(config.apps, ['capture']), graph: '',
    });
    // Assert
    assert.deepEqual(request.run!.systemApps, ['capture']);
    assert.equal(request.systemd!.appUnits![0].id, 'capture');
    assert.equal(request.systemd!.appUnits![0].file, join(apps, 'worker.service'));
    assert.deepEqual(request.systemd!.appUnits![0].binds, [
      { source: '/state/r1', target: '/results', readOnly: false },
    ]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});


test('one selected variant binds the same config for Compose and generated systemd units', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-shared-variant-'));
  const config = await systemdRelease();
  try {
    for (const variant of ['record', 'live']) {
      const apps = selectApps(config.apps, ['cpu-monitor'], { 'cpu-monitor': variant });
      const run = { id: 'r1', project: directory, artifacts: join(directory, 'artifacts'), workspace: directory, stage: variant, apps, graph: '' };
      const compose = execution.RunRequest.create({ run: { artifacts: run.artifacts }, compose: { projects: [] } });
      // Act
      const native = await systemdPlan(config, run);
      await bindOutputs(apps, compose, directory, variant);
      const overlay = JSON.parse(await readFile(compose.compose!.projects![0].files!.at(-1)!, 'utf8'));
      const bind = native.systemd!.programs!.find((app) => app.id === 'cpu-monitor')!.binds![0];
      // Assert
      assert.ok(overlay.services['cpu-monitor'].volumes.some((mount: any) => mount.source === bind.source && mount.target === bind.target && mount.read_only === bind.readOnly));
    }
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('common process settings support systemd without Compose files and reject native-unit layering', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-shared-process-'));
  const path = join(directory, 'capture.pbtxt');
  const manifest = 'id:"capture" process { argv:"capture" argv:"--rate" argv:"7" working_directory:"/results" memlock_bytes:8388608 cpus:"0" stop_signal:2 stop_seconds:17 }';
  const template = () => capabilities.parseConfig('host_shm_access:PASSTHROUGH form:SYSTEMD binaries:"/opt/bin" runtime {readiness_seconds:60 stop_seconds:30} apps {manifest:"capture.pbtxt"}');
  try {
    await writeFile(path, manifest);
    // Act
    const config = releaseConfig(template(), directory, directory, '/state');
    validateApps(config.apps, launch.Form.SYSTEMD);
    const request = await systemdPlan(config, { id:'r', project:directory, artifacts:'/state/r', workspace:directory, stage:'test', apps:config.apps, graph:'' });
    // Assert
    const app = request.systemd!.programs![0];
    assert.deepEqual(app.argv, ['/opt/bin/capture', '--rate', '7']);
    assert.equal(app.workingDirectory, '/results');
    assert.equal(Number(app.memlockBytes), 8388608);
    assert.equal(app.cpus, '0');
    assert.equal(app.killSignal, 2);
    assert.equal(Number(app.stopSeconds), 17);
    // Act / Assert: an external unit remains a separate launch input.
    await writeFile(path, manifest + ' systemd {unit:"capture.service"}');
    assert.throws(() => releaseConfig(template(), directory, directory, '/state'), /cannot be combined/);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
