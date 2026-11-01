import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { test } from 'node:test';
import { promisify } from 'node:util';
const exec = promisify(execFile);
const server = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const link = 'http://127.0.0.1:43187/#access=test-token';

for (const scenario of [
  { name: 'status', args: ['status'], output: `Open: ${link}\n`, opens: false },
  { name: 'already running', args: ['start'], output: `Open: ${link}\n`, opens: false },
  { name: 'fresh start', args: ['start'], output: `Launch browser: ${link}\n`, opens: true },
  { name: 'default start', args: [], output: `Launch browser: ${link}\n`, opens: true },
  { name: 'headless start', args: ['start', '--ssh', 'user@host'], output: `Launch browser: ${link}\n`, opens: false },
  { name: 'suppressed start', args: ['start', '--no-open'], output: `Launch browser: ${link}\n`, opens: false },
  { name: 'status never launches', args: ['status'], output: `Launch browser: ${link}\n`, opens: false },
]) test(`desktop wrapper: ${scenario.name}`, { skip: process.getuid?.() === 0 }, async () => {
  // Arrange
  const directory = await mkdtemp(join(tmpdir(), 'console-desktop-'));
  const opened = join(directory, 'opened.txt');
  const payload = JSON.stringify(scenario.output);
  await writeFile(join(directory, 'sudo'), `#!${process.execPath} --\nconst output=${payload};\nprocess.stdout.write(output.slice(0,8));\nsetTimeout(()=>{process.stdout.write(output.slice(8));},10);\nsetTimeout(()=>{},200);\n`, { mode: 0o755 });
  await writeFile(join(directory, 'xdg-open'), `#!${process.execPath} --\nrequire('node:fs').appendFileSync(${JSON.stringify(opened)},process.argv[2]+'\\n');\n`, { mode: 0o755 });
  try {
    // Act
    const result = await exec(process.execPath, ['--import', join(server, 'node_modules/tsx/dist/loader.mjs'),
      join(server, 'src/cli.ts'), ...scenario.args], {
      env: { ...process.env, PATH: `${directory}:${process.env.PATH}` }, timeout: 10_000,
    });
    await delay(100);
    // Assert
    assert.equal(result.stdout, scenario.output);
    if (scenario.opens) assert.equal(await readFile(opened, 'utf8'), `${link}\n`);
    else await assert.rejects(readFile(opened), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});
