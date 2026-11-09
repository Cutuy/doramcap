import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { loadConfig, validateConfig } from '../src/config.js';
import { capabilities, launch, live, wire } from '../src/native.js';
import { selectApps, validateApps, appOptions, discover } from '../src/system-apps.js';
import { mkdtemp, writeFile, mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const config = (extra = '') =>
  capabilities.parseConfig(
    `host_shm_access: PASSTHROUGH listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"/projects" data_root:"/data" docker_socket:"/docker.sock" access_secret_file:"/secret" ${extra}`,
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

test('systemd Consoles need no Docker socket and reject one', () => {
  // Arrange
  const systemd = (socket: string) =>
    capabilities.parseConfig(
      `form: SYSTEMD host_shm_access: PASSTHROUGH listen_address:"127.0.0.1:8080" public_origin:"http://127.0.0.1:8080" project_root:"/projects" data_root:"/data" ${socket}`,
    );
  // Act / Assert
  validateConfig(systemd(''));
  assert.throws(() => validateConfig(systemd('docker_socket:"/docker.sock"')), /only to COMPOSE/);
  // Act / Assert: Compose still requires the socket.
  assert.throws(() => validateConfig(wire.ConsoleConfig.create({ ...config(), dockerSocket: '' })), /docker_socket must be absolute/);
});

test('host shm access requires an explicit supported enum value', () => {
  // Arrange
  const base = config();
  // Act / Assert
  for (const hostShmAccess of [launch.HostShmAccess.PASSTHROUGH, launch.HostShmAccess.BLOCKED])
    assert.doesNotThrow(() => validateConfig(wire.ConsoleConfig.create({ ...base, hostShmAccess })));
  for (const hostShmAccess of [0, -1, 99])
    assert.throws(
      () => validateConfig(wire.ConsoleConfig.create({ ...base, hostShmAccess })),
      /host_shm_access must be PASSTHROUGH or BLOCKED/,
    );
  assert.throws(
    () => validateConfig(wire.ConsoleConfig.create({ ...base, hostShmAccess: undefined })),
    /host_shm_access must be PASSTHROUGH or BLOCKED/,
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
  assert.equal(appOptions(apps, []).filter((app) => app.required && app.selected).length, 0);
  const communication = [{ id: 'coordinator' }, { id: 'daemon' }];
  assert.deepEqual(
    appOptions([...communication, ...apps], [], communication.map((app) => app.id))
      .filter((app) => app.required && app.selected).map((app) => app.id),
    ['coordinator', 'daemon'],
  );
  assert.throws(() => validateApps([{ ...apps[0], recordings: [{ id: 'messages', path: '../outside' }] }]));
  const relay = { ...apps[0], recordings: [], configMount: { service: 'recorder', target: '/config/bridge.pbtxt' }, liveRelay: {} };
  validateApps([relay]);
  assert.throws(() => validateApps([relay, { ...relay, id: 'second' }]));
  assert.throws(() => validateApps([{ ...relay, configMount: undefined }]));
  assert.throws(() => validateApps([{ ...relay, liveRelay: { port: 65536 } }]));
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
    project = join(root, 'project');
  await mkdir(project);
  await writeFile(join(project, 'compose.yaml'), 'services: {}');
  await writeFile(join(project, '.env'), 'VALUE=kept');
  try {
    // Act
    const found = await discover(project, ['extra']);
    // Assert
    assert.deepEqual(found.files, ['compose.yaml']);
    assert.deepEqual(found.envFiles, ['.env']);
    assert.deepEqual(found.profiles, ['extra']);
    // Act / Assert
    await writeFile(join(project, 'docker-compose.yml'), 'services: {}');
    await assert.rejects(discover(project, []));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('launch configuration generates fresh access without a secret file', async () => {
  // Arrange
  const root = await mkdtemp(join(tmpdir(), 'console-launch-config-'));
  const path = join(root, 'console.pbtxt');
  await writeFile(path, `host_shm_access: PASSTHROUGH listen_address:"127.0.0.1:43187" public_origin:"http://127.0.0.1:43187" project_root:"${root}" data_root:"${root}/data" docker_socket:"/var/run/docker.sock"`);
  try {
    // Act
    const first = await loadConfig(path), second = await loadConfig(path);
    // Assert
    assert.match(first.secret, /^[A-Za-z0-9_-]{43}$/);
    assert.notEqual(first.secret, second.secret);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('launch profiles choose one config and add its outputs', () => {
  // Arrange
  const app = {
    id: 'monitor',
    input: { directory: '/apps', files: ['compose.yaml'] },
    liveOutputs: [{ source: 'monitor/snapshots', kind: live.Kind.CPU_ACCOUNTING }],
    configMount: { service: 'monitor', target: '/config/monitor.pbtxt' },
    launchProfiles: [
      { id: 'record', label: 'Record', config: '/apps/record.pbtxt', recordings: [{ id: 'cpu', path: 'cpu.mcap' }] },
      { id: 'live', label: 'Live', config: '/apps/live.pbtxt' },
    ],
  };
  // Act
  validateApps([app]);
  const fallback = selectApps([app], ['monitor'])[0],
    chosen = selectApps([app], ['monitor'], { monitor: 'live' })[0];
  // Assert
  assert.equal(fallback.profile?.id, 'record');
  assert.deepEqual(fallback.recordings?.map((output) => output.id), ['cpu']);
  assert.equal(fallback.liveOutputs?.length, 1);
  assert.equal(chosen.profile?.id, 'live');
  assert.deepEqual(chosen.recordings, []);
  assert.deepEqual(appOptions([app], [chosen]).at(-1), { id: 'monitor', label: undefined, selected: true, required: false, profiles: [{ id: 'record', label: 'Record' }, { id: 'live', label: 'Live' }], profile: 'live' });
  assert.equal(appOptions([app], []).at(-1)?.selected, false);

  // Act / Assert: unknown profiles and unselected or profile-less apps are rejected.
  assert.throws(() => selectApps([app], ['monitor'], { monitor: 'missing' }));
  assert.throws(() => selectApps([app], [], { monitor: 'live' }));
  assert.throws(() => selectApps([{ ...app, launchProfiles: [] }], ['monitor'], { monitor: 'live' }));

  // Act / Assert: invalid definitions are rejected.
  assert.throws(() => validateApps([{ ...app, configMount: undefined }]));
  assert.throws(() => validateApps([{ ...app, configMount: { service: 'monitor', target: 'relative' } }]));
  const [record, other] = app.launchProfiles;
  assert.throws(() => validateApps([{ ...app, launchProfiles: [record, { ...other, id: 'record' }] }]));
  assert.throws(() => validateApps([{ ...app, launchProfiles: [{ ...other, config: 'relative.pbtxt' }] }]));
  assert.throws(() => validateApps([{ ...app, launchProfiles: [{ ...other, label: '' }] }]));
  assert.throws(() => validateApps([{ ...app, launchProfiles: [{ ...other, recordings: [{ id: 'cpu', path: '../outside' }] }] }]));
  // Profiles of one app may share a recording ID; another app may not reuse it.
  validateApps([{ ...app, launchProfiles: [record, { ...other, recordings: record.recordings }] }]);
  assert.throws(() => validateApps([app, { ...app, id: 'second' }]));
});
