import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, cp, rm, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, basename, isAbsolute } from 'node:path';
import { promisify } from 'node:util';

const exec = promisify(execFile);
const [cli, metadataPath] = process.argv.slice(2);
assert.ok(cli && metadataPath && isAbsolute(cli) && isAbsolute(metadataPath),
  'usage: node application-view-identity.mts /absolute/holoscan /absolute/application/metadata.json');
const temporary = await mkdtemp(join(tmpdir(), 'holoscan-native-view-'));
try {
  const bin = join(temporary, 'bin'), home = join(temporary, 'home');
  const project = join(temporary, '-native-project');
  const named = join(temporary, 'views', 'merged', basename(project));
  const renamed = join(temporary, 'views', 'merged', '0');
  await Promise.all([bin, home, project].map(path => mkdir(path, { recursive: true })));
  const guard = join(temporary, 'docker-called');
  const shellQuote = (text: string) => `'${text.replaceAll("'", "'\\''")}'`;
  await writeFile(join(bin, 'docker'), `#!/bin/sh\nprintf called > ${shellQuote(guard)}\nexit 97\n`, { mode: 0o700 });
  const metadata = JSON.parse(await readFile(metadataPath, 'utf8'));
  delete metadata.application.requirements;
  metadata.application.default_mode = '-probe';
  metadata.application.modes = {
    '-probe': { description: 'view identity fixture', run: { command: 'true', workdir: 'holohub_bin' } },
  };
  await writeFile(join(project, 'metadata.json'), JSON.stringify(metadata));
  await writeFile(join(project, 'Dockerfile'), 'FROM ubuntu:24.04\n');
  await writeFile(join(project, 'CMakeLists.txt'), 'cmake_minimum_required(VERSION 3.22)\nproject(view_probe NONE)\n');
  await cp(project, named, { recursive: true });
  await cp(project, renamed, { recursive: true });
  const environment = {
    PATH: `${bin}:/usr/bin:/bin`, HOME: home, USER: 'fixture', LOGNAME: 'fixture',
    LANG: 'C.UTF-8', TERM: 'dumb', HOLOSCAN_CLI_BUILD_LOCAL: '0',
  };
  const inspect = async (root: string) => {
    const { stdout: list } = await exec(cli, ['--project-root', root, 'list', '--json'],
      { cwd: root, env: environment });
    const catalog = JSON.parse(list);
    const application = catalog.projects.find((project: any) => project.project_type === 'application')?.name;
    assert.ok(application, 'application is discoverable');
    const { stdout, stderr } = await exec(cli, ['--project-root', root, 'run',
      '--language', 'python', '--dryrun', '--no-docker-build', '--no-local-build', '--persistent',
      '--base-img', 'ubuntu:24.04', '--cuda', '12', '--', application, '-probe'], { cwd: root, env: environment });
    const text = `${stdout}${stderr}`.replace(/\x1b\[[0-9;]*m/g, '');
    const image = text.split('\n').find(line => /^\s*run image:\s/.test(line))?.trim();
    assert.ok(image, `native image diagnostic is present: ${text}`);
    return { application, image };
  };
  const built = await inspect(project), overlaid = await inspect(named), changed = await inspect(renamed);
  assert.deepEqual(overlaid, built, 'named project view preserves native application and default image');
  assert.notDeepEqual(changed, built, 'changed basename exercises a real native identity distinction');
  await assert.rejects(access(guard), { code: 'ENOENT' });
  console.log('Native CLI5 project basename preserves catalog identity and default image without --img: passed');
} finally {
  await rm(temporary, { recursive: true, force: true });
}
