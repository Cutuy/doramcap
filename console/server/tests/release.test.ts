import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, cp, lstat, mkdir, mkdtemp, readFile, readlink, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { capabilities, wire } from '../src/native.js';
import { validateConfig } from '../src/config.js';
import { requireMultiArch } from '../src/images.js';
import { doctor, initialize, installRelease, projectId, releaseConfig } from '../src/release.js';
import { stageSystemAppInputs } from '../../scripts/system-app-inputs.mjs';
import { mcapServices } from '../src/system-apps.js';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

test('release configuration resolves native inputs and round-trips canonical pbtxt', () => {
  // Arrange
  const template = capabilities.parseConfig(`project_root:"projects" dora {image:"registry/dora@sha256:abc" graph:"graphs/flow.yml" shm_bytes:134217728}
    system_apps {id:"capture" input {directory:"capture" files:"compose.yaml" files:"../override.yaml"} graph:"capture/flow.yml" launch_profiles {id:"capture" label:"Capture" config:"capture/run.pbtxt" graph:"capture/profile.yml"}}`);
  // Act
  const config = releaseConfig(template, '/state/playground', '/release/system-apps', '/state');
  const decoded = capabilities.parseConfig(capabilities.formatConfig(config));
  // Assert
  assert.equal(decoded.projectRoot, '/state/playground');
  assert.equal(decoded.dataRoot, '/state');
  assert.equal(decoded.dora!.graph, '/state/playground/graphs/flow.yml');
  assert.equal(decoded.systemApps[0].input!.directory, '/release/system-apps/capture');
  assert.equal(decoded.systemApps[0].graph, '/release/system-apps/capture/flow.yml');
  assert.equal(decoded.systemApps[0].launchProfiles![0].graph, '/release/system-apps/capture/profile.yml');
  assert.equal(decoded.downloadMissingImages, true);
  assert.equal(decoded.accessSecretFile, '');
  assert.equal(String(decoded.dora!.shmBytes), '134217728');
  assert.equal(template.projectRoot, 'projects');
  for (const path of ['/host', '../outside']) {
    assert.throws(() => releaseConfig(wire.ConsoleConfig.create({ dora: { graph: path } }), '/playground', '/apps', '/state'));
    assert.throws(() => releaseConfig(wire.ConsoleConfig.create({ systemApps: [{ id: 'app', input: { directory: path, files: ['compose.yaml'] } }] }), '/playground', '/apps', '/state'));
  }
});

test('release manifests expand to the same apps as the inline release configuration', async () => {
  // Arrange
  const read = async (path: string) => capabilities.parseConfig(await readFile(join(repository, path), 'utf8'));
  const systemApps = join(repository, 'console/release/system-apps');
  const manifests = await read('console/release/console.pbtxt');
  const inline = await read('console/server/tests/fixtures/inline-release.pbtxt');
  const apps = (config: ReturnType<typeof releaseConfig>) =>
    capabilities.formatConfig(wire.ConsoleConfig.create({ systemApps: config.systemApps }));
  // Act
  const expanded = releaseConfig(manifests, '/playground', systemApps, '/state');
  // Assert
  assert.equal(apps(expanded), apps(releaseConfig(inline, '/playground', systemApps, '/state')));
  assert.ok(expanded.systemApps.every((app) => !app.manifest));
});

test('manifests reject inline launch fields, escaping paths and unsupported forms', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-manifest-'));
  const config = (text: string) => capabilities.parseConfig(text);
  try {
    await mkdir(join(directory, 'app'));
    await writeFile(join(directory, 'app/app.pbtxt'), 'id:"app" compose {files:"compose.yaml"}');
    await writeFile(join(directory, 'app/escape.pbtxt'), 'id:"app" compose {files:"../compose.yaml"}');
    // Act / Assert: a valid manifest expands relative to its directory.
    const valid = releaseConfig(config('system_apps {manifest:"app/app.pbtxt"}'), '/playground', directory, '/state');
    assert.equal(valid.systemApps[0].input!.directory, join(directory, 'app'));
    // Act / Assert: launch facts live only in the manifest.
    assert.throws(() => releaseConfig(config('system_apps {manifest:"app/app.pbtxt" graph:"flow.yml"}'), '/playground', directory, '/state'), /belong in the manifest/);
    // Act / Assert: manifest paths stay beside the manifest.
    assert.throws(() => releaseConfig(config('system_apps {manifest:"app/escape.pbtxt"}'), '/playground', directory, '/state'), /stay beside/);
    // Act / Assert: only Compose launches today.
    assert.throws(() => releaseConfig(config('form: SYSTEMD'), '/playground', directory, '/state'), /only COMPOSE/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('versioned installation preserves old releases and refuses replacement of unrelated launchers', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-install-'));
  const source = join(directory, 'source'), prefix = join(directory, 'opt'), bin = join(directory, 'bin/console');
  await mkdir(join(source, 'bin'), { recursive: true });
  await writeFile(join(source, 'bin/doramcap-console'), '#!/bin/sh\n', { mode: 0o755 });
  await symlink('doramcap-console', join(source, 'bin/relative-link'));
  const manifest = (version: string) => wire.ReleaseManifest.encode({ version, platform: `linux-${process.arch}`, config: {} }).finish();
  try {
    await writeFile(join(source, 'release.pb'), manifest('0.1.0'));
    // Act
    const first = await installRelease(source, prefix, bin);
    // Assert
    assert.equal(await readlink(join(prefix, 'current')), 'releases/0.1.0');
    assert.ok((await lstat(join(first, 'bin/doramcap-console'))).mode & 0o111);
    assert.equal(await readlink(join(first, 'bin/relative-link')), 'doramcap-console');
    await assert.rejects(installRelease(source, prefix, bin), /already installed/);
    // Act
    await writeFile(join(source, 'release.pb'), manifest('0.2.0'));
    await installRelease(source, prefix, bin);
    // Assert
    assert.equal(await readlink(join(prefix, 'current')), 'releases/0.2.0');
    assert.equal((await lstat(first)).isDirectory(), true);
    const unrelated = join(directory, 'other');
    await writeFile(unrelated, 'keep');
    await writeFile(join(source, 'release.pb'), manifest('0.3.0'));
    await assert.rejects(installRelease(source, prefix, unrelated), /Refusing/);
    assert.equal(await readFile(unrelated, 'utf8'), 'keep');
    assert.equal(await readlink(join(prefix, 'current')), 'releases/0.2.0');
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('generated installer selects both architectures and rejects corruption before sudo', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-bootstrap-'));
  const commands = join(directory, 'commands'), payload = join(directory, 'payload');
  await mkdir(commands); await mkdir(join(payload, 'package'), { recursive: true });
  await writeFile(join(payload, 'package/release.pb'), 'release');
  const archive = join(directory, 'release.tar.gz');
  assert.equal(spawnSync('tar', ['-czf', archive, '-C', payload, 'package']).status, 0);
  const hash = createHash('sha256').update(await readFile(archive)).digest('hex');
  const marker = join(directory, 'sudo.txt'), installer = join(directory, 'install.sh');
  const executable = async (name: string, code: string) => writeFile(join(commands, name), `#!${process.execPath}\n${code}`, { mode: 0o755 });
  await executable('id', 'console.log(1000)');
  await executable('curl', `require('node:fs').copyFileSync(${JSON.stringify(archive)},process.argv[process.argv.indexOf('-o')+1]);`);
  await executable('sudo', `require('node:fs').writeFileSync(${JSON.stringify(marker)},process.argv.slice(2).join(' '));`);
  const generated = spawnSync(process.execPath, [join(repository, 'console/scripts/installer.mts'),
    '--base-url', 'https://downloads.example.com/0.1.0', '--version', '0.1.0',
    '--amd64-sha256', hash, '--arm64-sha256', hash, '--output', installer], { encoding: 'utf8' });
  assert.equal(generated.status, 0, generated.stderr);
  try {
    for (const architecture of ['x86_64', 'aarch64']) {
      await executable('uname', `console.log(process.argv[2]==='-s'?'Linux':${JSON.stringify(architecture)})`);
      // Act
      const result = spawnSync('sh', [installer], { env: { ...process.env, PATH: `${commands}:${process.env.PATH}` }, encoding: 'utf8' });
      // Assert
      assert.equal(result.status, 0, result.stderr);
      assert.ok(result.stdout.includes(`for linux-${architecture === 'x86_64' ? 'amd64' : 'arm64'}`));
      assert.match(await readFile(marker, 'utf8'), /package\/runtime\/node .*cli.js install/);
      await rm(marker);
    }
    // Act
    await writeFile(archive, 'corrupted');
    const corrupt = spawnSync('sh', [installer], { env: { ...process.env, PATH: `${commands}:${process.env.PATH}` }, encoding: 'utf8' });
    // Assert
    assert.notEqual(corrupt.status, 0);
    await assert.rejects(lstat(marker), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('project IDs are readable, unique per path and safe in unit names', () => {
  // Act / Assert
  assert.match(projectId('/home/alice/robot'), /^robot-[0-9a-f]{12}$/);
  assert.notEqual(projectId('/home/alice/robot'), projectId('/home/bob/robot'));
  assert.equal(projectId('/home/alice/robot'), projectId('/home/alice/robot'));
  assert.match(projectId('/work/my repo.v2!'), /^my-repo-v2-[0-9a-f]{12}$/);
  assert.match(projectId('/'), /^project-[0-9a-f]{12}$/);
  assert.ok(projectId(`/${'x'.repeat(300)}`).length <= 53);
});

test('root setup pins one project config and preserves later edits',
  { skip: process.getuid?.() !== 0 }, async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-root-setup-'));
  const source = join(directory, 'source'), state = join(directory, 'state'), project = join(directory, 'project');
  await mkdir(source);
  await mkdir(project);
  await writeFile(join(source, 'release.pb'), wire.ReleaseManifest.encode({
    version: '0.1.0', platform: `linux-${process.arch}`,
    config: { projectRoot: '.', systemApps: [{ id: 'capture', input: { directory: 'capture', files: ['compose.yaml'] } }] },
  }).finish());
  try {
    // Act
    const path = await initialize(source, project, state);
    // Assert
    const config = capabilities.parseConfig(await readFile(path, 'utf8'));
    assert.equal(config.projectRoot, project);
    assert.equal(config.dataRoot, state);
    assert.equal(config.systemApps[0].input!.directory, join(source, 'system-apps/capture'));
    assert.equal((await lstat(path)).mode & 0o777, 0o600);
    assert.equal(await readFile(join(state, 'project.txt'), 'utf8'), `${project}\n`);
    // Act
    await writeFile(path, 'operator configuration');
    await initialize(source, project, state);
    // Assert
    assert.equal(await readFile(path, 'utf8'), 'operator configuration');
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('the repository release template resolves to existing current app inputs', async () => {
  // Arrange
  const playground = join(repository, 'console/release/playground');
  const systemApps = join(await mkdtemp(join(tmpdir(), 'release-template-')), 'system-apps');
  await cp(join(repository, 'console/release/system-apps'), systemApps, { recursive: true });
  stageSystemAppInputs(repository, systemApps);
  const template = capabilities.parseConfig(await readFile(join(repository, 'console/release/console.pbtxt'), 'utf8'));
  // Act
  const config = releaseConfig(template, playground, systemApps, '/state');
  validateConfig(config);
  // Assert
  assert.deepEqual(config.systemApps.filter(app => app.defaultSelected).map(app => app.id), ['live-bridge', 'cpu-monitor']);
  assert.equal(config.systemApps.filter(app => app.liveRelay).map(app => app.id).join(), 'live-bridge');
  assert.ok(capabilities.parseConfig(capabilities.formatConfig(config)).systemApps.find(app => app.id === 'live-bridge')!.liveRelay);
  await access(config.projectRoot);
  for (const app of config.systemApps) {
    for (const file of app.input!.files!) await access(join(app.input!.directory!, file));
    if (app.liveRelay) continue;
    await access(app.graph!);
    const profiles = app.launchProfiles ?? [];
    for (const profile of profiles) {
      await access(profile.config!);
      if (profile.graph) await access(profile.graph);
    }
    // An app's default launch produces something the user can see or download.
    assert.ok(app.recordings!.length + app.liveOutputs!.length + (profiles[0]?.recordings?.length ?? 0) > 0, app.id!);
  }
  assert.deepEqual(config.systemApps.map(app => app.id), ['live-bridge', 'cpu-monitor', 'data-recorder', 'fault-watchdog']);
  const profiles = config.systemApps.find(app => app.id === 'cpu-monitor')!.launchProfiles!;
  assert.deepEqual(profiles.map(profile => [profile.id, profile.recordings!.length]), [['record', 1], ['live', 0]]);
  const live = await readFile(join(systemApps, 'cpu-monitor/live.pbtxt'), 'utf8');
  assert.match(live, /^dora \{\}/m);
  assert.doesNotMatch(live, /recording/);
  const monitor = await readFile(join(systemApps, 'cpu-monitor/monitor.pbtxt'), 'utf8');
  assert.match(monitor, /^dora \{\}/m);
  assert.match(monitor, /path: "\/results\/cpu.mcap"/);
  const watchdog = config.systemApps.find(app => app.id === 'fault-watchdog')!;
  assert.equal(watchdog.defaultSelected, false);
  assert.equal(watchdog.healthInputs, true);
  assert.deepEqual(watchdog.launchProfiles!.map(profile => [profile.id, profile.recordings!.length]), [['live', 0], ['mcap', 1], ['otel', 0]]);
  await rm(dirname(systemApps), { recursive: true });
});

test('only recording apps name MCAP writer services', () => {
  // Arrange
  const mount = [{ service: 'writer', target: '/results' }];
  // Act / Assert
  assert.deepEqual(mcapServices([{ outputMounts: mount, recordings: [{ id: 'a', path: 'a.mcap' }] }]), ['writer']);
  assert.deepEqual(mcapServices([{ outputMounts: mount, recordings: [] }]), []);
});

test('doctor reports every missing requirement with its remediation', async () => {
  // Arrange
  const empty = await mkdtemp(join(tmpdir(), 'console-doctor-')), previous = process.env.PATH, lines: string[] = [];
  process.env.PATH = empty;
  try {
    // Act / Assert
    await assert.rejects(doctor('/custom.sock', line => lines.push(line)), /required checks failed/);
    const report = lines.join('\n');
    assert.match(report, /FAIL  Docker CLI: not found\n\s+Fix: Install Docker Engine/);
    assert.match(report, /FAIL  rsync: not found\n\s+Fix: Install it with: sudo apt-get install rsync/);
    assert.match(report, /FAIL  systemd: not found/);
    assert.match(report, /listen on \/custom.sock/);
  } finally {
    process.env.PATH = previous;
    await rm(empty, { recursive: true, force: true });
  }
});

test('release images must offer both architectures', () => {
  // Arrange
  const index = (...architectures: string[]) => JSON.stringify({ manifests: architectures.map(architecture => ({ platform: { os: 'linux', architecture } })) });
  // Act / Assert
  requireMultiArch('image', index('amd64', 'arm64', 'unknown'));
  assert.throws(() => requireMultiArch('image', index('amd64')), /linux\/arm64: image/);
  assert.throws(() => requireMultiArch('image', index('arm64')), /linux\/amd64: image/);
  assert.throws(() => requireMultiArch('image', JSON.stringify({ config: {}, layers: [] })), /multi-arch/);
});
