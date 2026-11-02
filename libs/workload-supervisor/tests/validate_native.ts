#!/usr/bin/env node
// Run the prebuilt Supervisor native tests with capabilities; never build as root.
import { spawnSync } from 'node:child_process';
import { accessSync, constants, readdirSync, statSync } from 'node:fs';
import { homedir, userInfo } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: { monitor: { type: 'string' } } });
if (!values.monitor) throw new Error('usage: --monitor ABSOLUTE_PATH');
const monitor = resolve(values.monitor), binaries = dirname(monitor);
const executable = (path: string) => { try { accessSync(path, constants.X_OK); return statSync(path).isFile(); } catch { return false; } };
const owner = spawnSync('getent', ['passwd', String(statSync(monitor).uid)], { encoding: 'utf8' }).stdout.split(':')[5] || homedir() || userInfo().homedir;
const runtime = (name: string) => [...(process.env.PATH ?? '').split(':').map((directory) => join(directory, name)),
  join(owner, '.local/share/doramcap-supervisor/bin', name)].find(executable);
const supervisord = runtime('supervisord'), supervisorctl = runtime('supervisorctl');
if (!supervisord || !supervisorctl) {
  console.log('SKIP Supervisor native scenario: provision supervisord and supervisorctl');
  process.exit(0);
}
const fixture = join(binaries, 'supervisor-test-fixture');
if (!executable(fixture)) throw new Error('Build tests/fixture.inc unprivileged as supervisor-test-fixture beside system-monitor');
const test = readdirSync(join(binaries, 'deps'))
  .filter((name) => name.startsWith('native-') && executable(join(binaries, 'deps', name)))
  .map((name) => join(binaries, 'deps', name))
  .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)
  .find((path) => spawnSync(path, ['--list'], { encoding: 'utf8' }).stdout.includes('real_recordings_spaces_and_concurrent_runs:'));
if (!test) throw new Error('Build workload-supervisor --test native unprivileged before native validation');
const result = spawnSync(test, ['--ignored', '--nocapture', '--test-threads=1'], {
  stdio: 'inherit',
  timeout: 120_000,
  env: { ...process.env, DORAMCAP_NATIVE_MONITOR: monitor, DORAMCAP_TEST_FIXTURE: fixture, DORAMCAP_SUPERVISORD: supervisord, DORAMCAP_SUPERVISORCTL: supervisorctl },
});
if (result.status !== 0) throw new Error('Supervisor native tests failed');
console.log('PASS Supervisor native tests');
