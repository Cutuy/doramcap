import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { resolveConfigs, resolvedConfig } from '../src/config-resolver.js';
import type { SelectedApp } from '../src/system-apps.js';

const app = (config: string): SelectedApp => ({
  id: 'watchdog',
  input: { directory: tmpdir(), files: ['compose.yaml'] },
  configMount: { service: 'watchdog', target: '/config/watchdog.pbtxt' },
  configResolver: { command: ['/bin/watchdog', '--resolve-config'] },
  profile: { id: 'mcap', label: 'MCAP', config },
});

test('resolvers write their output, report failures and leave no container', async () => {
  // Arrange: a Docker client whose behavior follows the profile's name.
  const root = await mkdtemp(join(tmpdir(), 'console-resolver-')),
    log = join(root, 'calls.jsonl'),
    workspace = join(root, 'workspace'),
    previous = process.env.PATH,
    signal = new AbortController().signal;
  await mkdir(workspace);
  await writeFile(join(root, 'docker'), `#!${process.execPath} --
const fs = require('node:fs'), args = process.argv.slice(2), file = args.at(-1);
fs.appendFileSync(${JSON.stringify(log)}, JSON.stringify(args)+'\\n');
if (args.includes('compose')) console.log(JSON.stringify({services:{watchdog:{image:'registry/watchdog@sha256:a'}}}));
else if (args.includes('rm')) process.exit(0);
else if (file.endsWith('good.pbtxt')) process.stdout.write('app { id: "watchdog" }\\n');
else if (file.endsWith('bad.pbtxt')) { console.error('mcap requested but not compiled'); process.exit(1); }
else if (file.endsWith('huge.pbtxt')) process.stdout.write('x'.repeat(2 * 1024 * 1024));
else if (file.endsWith('hung.pbtxt')) setTimeout(() => {}, 60000);
`, { mode: 0o755 });
  process.env.PATH = `${root}:${previous}`;
  const calls = async (): Promise<string[][]> => (await readFile(log, 'utf8')).trim().split('\n').map((line) => JSON.parse(line));
  try {
    // Act
    await resolveConfigs('/custom.sock', [app('/profiles/good.pbtxt')], workspace, signal);

    // Assert: stdout is the file, readable by the app's own user, from a sandboxed run.
    const path = resolvedConfig(workspace, 'watchdog');
    assert.equal(await readFile(path, 'utf8'), 'app { id: "watchdog" }\n');
    assert.equal((await stat(path)).mode & 0o777, 0o644);
    const run = (await calls()).find((args) => args.includes('run'))!;
    for (const flag of [['--network', 'none'], ['--cap-drop', 'ALL'], ['--security-opt', 'no-new-privileges'], ['--pull', 'never']])
      assert.equal(run[run.indexOf(flag[0]) + 1], flag[1]);
    assert.ok(run.includes('--read-only') && run.includes('--rm'));
    assert.ok(run.includes('type=bind,source=/profiles,target=/config,readonly'));
    assert.deepEqual(run.slice(-4), ['/bin/watchdog', 'registry/watchdog@sha256:a', '--resolve-config', '/config/good.pbtxt']);

    // Act / Assert: a failure carries the resolver's stderr.
    await assert.rejects(resolveConfigs('/custom.sock', [app('/profiles/bad.pbtxt')], workspace, signal), /mcap requested but not compiled/);
    // Act / Assert: oversized output and a hung resolver fail, and remove the container.
    const failures = async () => (await calls()).filter((args) => args.includes('rm')).length;
    const removed = await failures();
    await assert.rejects(resolveConfigs('/custom.sock', [app('/profiles/huge.pbtxt')], workspace, signal), /config resolver failed/);
    await assert.rejects(resolveConfigs('/custom.sock', [app('/profiles/hung.pbtxt')], workspace, signal, 100), /config resolver failed/);
    assert.equal(await failures(), removed + 2);
    // Act / Assert: empty output fails, and cancellation reports Stopped.
    await assert.rejects(resolveConfigs('/custom.sock', [app('/profiles/empty.pbtxt')], workspace, signal), /empty output/);
    const cancel = new AbortController();
    const cancelled = resolveConfigs('/custom.sock', [app('/profiles/hung.pbtxt')], workspace, cancel.signal);
    setTimeout(() => cancel.abort(), 100);
    await assert.rejects(cancelled, /^Error: Stopped$/);

    // Act / Assert: an app without a resolver runs nothing.
    const before = (await calls()).length;
    await resolveConfigs('/custom.sock', [{ ...app('/profiles/good.pbtxt'), configResolver: undefined }], workspace, signal);
    assert.equal((await calls()).length, before);
  } finally {
    process.env.PATH = previous;
    await rm(root, { recursive: true, force: true });
  }
});
