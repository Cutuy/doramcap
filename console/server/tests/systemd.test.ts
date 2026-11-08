import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { capabilities } from '../src/native.js';
import { releaseConfig } from '../src/release.js';
import { selectApps } from '../src/system-apps.js';
import { systemdPlan } from '../src/systemd.js';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const systemApps = join(repository, 'console/release/system-apps');

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
    const unit = (id: string) => plan.systemd!.appUnits!.find((app) => app.file === join(systemApps, id, `${id}.service`))!;
    assert.equal(plan.systemd!.binaries, '/opt/doramcap/bin');
    assert.deepEqual(unit('cpu-monitor').binds!.map(({ source, target, readOnly }) => [source, target, readOnly]), [
      [join(systemApps, 'cpu-monitor/monitor.pbtxt'), '/config/monitor.pbtxt', true],
      [artifacts, '/results', false],
    ]);
    assert.ok(unit('data-recorder').binds!.some((bind) => bind.source === join(systemApps, 'data-recorder/telemetry_descriptor.bin') && bind.target === '/config/telemetry_descriptor.bin'));
    assert.equal(unit('live-bridge').binds![0].source, join(directory, 'bridge.pbtxt'));
    assert.deepEqual(plan.run!.recordings!.map((r) => r.recording), [join(artifacts, 'cpu.mcap'), join(artifacts, 'messages.mcap')]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('systemd needs absolute binaries and a unit file for every selected app', async () => {
  // Arrange
  const template = (text: string) => capabilities.parseConfig(text);
  // Act / Assert: binaries are required.
  assert.throws(() => releaseConfig(template('form: SYSTEMD'), '/playground', systemApps, '/state'), /binaries must be absolute/);
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
