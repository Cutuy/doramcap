import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstat, mkdir, mkdtemp, readFile, readlink, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { capabilities, wire } from '../src/native.js';
import { initialize, installRelease, releaseConfig } from '../src/release.js';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

test('release configuration resolves native inputs and round-trips canonical pbtxt', () => {
  // Arrange
  const template = capabilities.parseConfig(`project_root:"projects" dora {image:"registry/dora@sha256:abc" graph:"graphs/flow.yml" shm_bytes:134217728}
    system_apps {id:"capture" input {directory:"apps/capture" files:"compose.yaml" files:"../override.yaml"} graph:"graphs/capture.yml"}`);
  // Act
  const config = releaseConfig(template, '/state/playground', '/state');
  const decoded = capabilities.parseConfig(capabilities.formatConfig(config));
  // Assert
  assert.equal(decoded.projectRoot, '/state/playground/projects');
  assert.equal(decoded.dora!.graph, '/state/playground/graphs/flow.yml');
  assert.equal(decoded.systemApps[0].input!.directory, '/state/playground/apps/capture');
  assert.equal(decoded.downloadMissingImages, true);
  assert.equal(decoded.accessSecretFile, '');
  assert.equal(String(decoded.dora!.shmBytes), '134217728');
  assert.equal(template.projectRoot, 'projects');
  for (const path of ['/host', '../outside']) {
    assert.throws(() => releaseConfig(wire.ConsoleConfig.create({ projectRoot: path }), '/playground', '/state'));
  }
});

test('versioned installation preserves old releases and refuses replacement of unrelated launchers', async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-install-'));
  const source = join(directory, 'source'), prefix = join(directory, 'opt'), bin = join(directory, 'bin/console');
  await mkdir(join(source, 'bin'), { recursive: true });
  await writeFile(join(source, 'bin/doramcap-console'), '#!/bin/sh\n', { mode: 0o755 });
  const manifest = (version: string) => wire.ReleaseManifest.encode({ version, platform: `linux-${process.arch}`, config: {} }).finish();
  try {
    await writeFile(join(source, 'release.pb'), manifest('0.1.0'));
    // Act
    const first = await installRelease(source, prefix, bin);
    // Assert
    assert.equal(await readlink(join(prefix, 'current')), 'releases/0.1.0');
    assert.ok((await lstat(join(first, 'bin/doramcap-console'))).mode & 0o111);
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

test('root setup seeds the playground once and preserves edited projects and configuration',
  { skip: process.getuid?.() !== 0 }, async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-root-setup-'));
  const source = join(directory, 'source'), state = join(directory, 'state');
  await mkdir(join(source, 'playground/project'), { recursive: true });
  await writeFile(join(source, 'playground/project/compose.yaml'), 'services: {}');
  await writeFile(join(source, 'release.pb'), wire.ReleaseManifest.encode({
    version: '0.1.0', platform: `linux-${process.arch}`, config: { projectRoot: '.' },
  }).finish());
  try {
    // Act
    const path = await initialize(source, state);
    // Assert
    const config = capabilities.parseConfig(await readFile(path, 'utf8'));
    assert.equal(config.projectRoot, join(state, 'playground'));
    assert.equal((await lstat(path)).mode & 0o777, 0o600);
    assert.equal(await readFile(join(state, 'playground/project/compose.yaml'), 'utf8'), 'services: {}');
    // Act
    await writeFile(path, 'operator configuration');
    await writeFile(join(state, 'playground/project/compose.yaml'), 'edited project');
    await initialize(source, state);
    // Assert
    assert.equal(await readFile(path, 'utf8'), 'operator configuration');
    assert.equal(await readFile(join(state, 'playground/project/compose.yaml'), 'utf8'), 'edited project');
  } finally { await rm(directory, { recursive: true, force: true }); }
});
