import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { loadConfig, validateConfig } from '../src/config.js';
import { capabilities, live, wire } from '../src/native.js';
import { selectApps, validateApps, appOptions, discover } from '../src/system-apps.js';
import { mkdtemp, writeFile, mkdir, symlink, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const config = (extra = '') =>
  capabilities.parseConfig(
    `listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"/projects" data_root:"/data" docker_socket:"/docker.sock" access_secret_file:"/secret" ${extra}`,
  );

test('TypeScript applies protobuf configuration defaults and rejects invalid deployment policy', () => {
  // Arrange
  const defaults = config(),
    custom = config('runtime { readiness_seconds:90 stop_seconds:25 sample_interval_ms:250 }');
  // Act
  validateConfig(defaults);
  validateConfig(custom);
  // Assert
  assert.equal(defaults.runtime!.readinessSeconds, 60);
  assert.equal(custom.runtime!.sampleIntervalMs, 250);
  for (const extra of [
    'runtime { stop_seconds:0 }',
    'runtime { stop_seconds:301 }',
    'runtime { sample_interval_ms:0 }',
  ])
    assert.throws(() => validateConfig(config(extra)));
  assert.throws(() =>
    validateConfig(wire.ConsoleConfig.create({ ...defaults, publicOrigin: 'https://example.com/path' })),
  );
});

test('catalog selection distinguishes defaults from an explicit empty choice', () => {
  // Arrange
  const apps = [
    {
      id: 'recorder',
      label: 'Recorder',
      defaultSelected: true,
      input: { directory: '/apps', files: ['compose.yaml'] },
      recordings: [{ id: 'messages', path: 'messages.mcap' }],
    },
  ];
  // Act / Assert
  validateApps(apps);
  assert.equal(selectApps(apps).length, 1);
  assert.equal(selectApps(apps, []).length, 0);
  assert.throws(() => selectApps(apps, ['missing']));
  assert.throws(() => selectApps(apps, ['recorder', 'recorder']));
  assert.equal(appOptions(apps, []).filter((app) => app.required && app.selected).length, 2);
  assert.throws(() => validateApps([{ ...apps[0], recordings: [{ id: 'messages', path: '../outside' }] }]));
  assert.throws(() => validateApps([{ ...apps[0], id: 'live-bridge' }]));
  for (const liveOutputs of [
    [{ source: 'monitor', kind: live.Kind.CPU_ACCOUNTING }],
    [{ source: 'monitor/snapshots' }],
    [{ source: 'monitor/snapshots', kind: live.Kind.CPU_ACCOUNTING }, { source: 'monitor/snapshots', kind: live.Kind.CPU_ACCOUNTING }],
  ])
    assert.throws(() => validateApps([{ ...apps[0], liveOutputs }]));
  validateApps([{ ...apps[0], liveOutputs: [{ source: 'monitor/snapshots', kind: live.Kind.CPU_ACCOUNTING }] }]);
});

test('project discovery confines canonical paths and detects native input changes', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-discovery-')),
    project = join(root, 'project'),
    outside = await mkdtemp(join(tmpdir(), 'console-outside-'));
  await mkdir(project);
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  await writeFile(join(project, '.env'), 'VALUE=kept');
  try {
    // Act
    const found = await discover(root, project, ['extra']);
    // Assert
    assert.deepEqual(found.files, ['compose.yaml']);
    assert.deepEqual(found.envFiles, ['.env']);
    assert.deepEqual(found.profiles, ['extra']);
    // Act / Assert
    await symlink(outside, join(root, 'escape'));
    await assert.rejects(discover(root, join(root, 'escape'), []));
    await writeFile(join(project, 'docker-compose.yml'), 'services: {}');
    await assert.rejects(discover(root, project, []));
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  }
});

test('launch configuration generates fresh access without a secret file', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-launch-config-'));
  const path = join(root, 'console.pbtxt');
  await writeFile(path, `listen_address:"127.0.0.1:43187" public_origin:"http://127.0.0.1:43187" project_root:"${root}" data_root:"${root}/data" docker_socket:"/var/run/docker.sock"`);
  try {
    // Act
    const first = await loadConfig(path), second = await loadConfig(path);
    // Assert
    assert.match(first.secret, /^[A-Za-z0-9_-]{43}$/);
    assert.notEqual(first.secret, second.secret);
  } finally { await rm(root, { recursive: true, force: true }); }
});
