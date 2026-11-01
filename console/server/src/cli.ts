import { execFile, spawn } from 'node:child_process';
import { readFile, rm, stat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { capabilities } from './native.js';
import { launchInstructions, portNumber } from './launch.js';
import { initialize, installRelease, privateState, readRelease } from './release.js';
const exec = promisify(execFile);
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values, positionals } = parseArgs({ allowPositionals: true, options: {
  state: { type: 'string', default: '/var/lib/doramcap-console' },
  port: { type: 'string' }, 'browser-port': { type: 'string' }, ssh: { type: 'string' },
  'no-open': { type: 'boolean' }, help: { type: 'boolean' },
} });
const command = positionals[0] ?? 'start';
const unit = 'doramcap-console.service';
const state = resolve(values.state!);
const launchFile = join(state, 'access.txt');

async function active() {
  return exec('systemctl', ['is-active', '--quiet', unit]).then(() => true, () => false);
}

async function doctor() {
  const release = await readRelease(root);
  const socket = release.config?.dockerSocket || '/var/run/docker.sock';
  for (const [program, args] of [
    ['docker', ['info', '--format', '{{.OSType}}']], ['docker', ['compose', 'version']],
    ['rsync', ['--version']], ['systemctl', ['--version']],
  ] as const) {
    const dockerArgs = program === 'docker' ? ['--host', `unix://${socket}`, ...args] : [...args];
    const { stdout } = await exec(program, dockerArgs, { timeout: 15_000 });
    if (program === 'docker' && args[0] === 'info' && stdout.trim() !== 'linux')
      throw new Error('A local Linux Docker Engine is required');
    console.log(`${program}: ${stdout.split('\n')[0]}`);
  }
  await stat('/sys/fs/cgroup/cgroup.controllers');
  if (!(await readFile('/proc/filesystems', 'utf8')).includes('overlay'))
    throw new Error('Linux OverlayFS support is required');
}

async function main() {
  if (values.help) {
    console.log('Usage: doramcap-console [start|stop|status|doctor|install] [--state PATH] [--ssh user@host] [--port PORT] [--browser-port PORT] [--no-open]');
    return;
  }
  if (!['start', 'stop', 'status', 'doctor', 'install'].includes(command) || positionals.length > 1)
    throw new Error('Unknown command; use --help');
  if (process.platform !== 'linux') throw new Error('Console runs on a Linux Docker host');
  if (values.port) portNumber(values.port);
  if (values['browser-port']) portNumber(values['browser-port']);
  if (values.ssh) launchInstructions('http://127.0.0.1:43187', 43187, '', values.ssh);
  if (process.getuid!() !== 0) {
    if (command === 'install') throw new Error('Run the release installer with sudo');
    // Keep browser opening in the invoking desktop user's process.
    const child = spawn('sudo', ['--', process.execPath, fileURLToPath(import.meta.url), ...process.argv.slice(2)],
      { stdio: ['inherit', 'pipe', 'inherit'] });
    let pending = '', opened = false;
    child.stdout.on('data', (chunk: Buffer) => {
      process.stdout.write(chunk);
      pending += chunk.toString();
      const lines = pending.split('\n'); pending = lines.pop()!;
      for (const line of lines) if (!opened && !values.ssh && !values['no-open'] && line.startsWith('Open: ')) {
        opened = true;
        const browser = spawn('xdg-open', [line.slice(6)], { stdio: 'ignore' });
        browser.on('error', () => {});
        browser.unref();
      }
    });
    await new Promise<void>((resolve, reject) => {
      child.on('error', reject);
      child.on('exit', code => { process.exitCode = code ?? 1; resolve(); });
    });
    return;
  }
  const lock = capabilities.lockFile('/run/doramcap-console-cli.lock');
  try {
    if (command === 'install') {
      if (await active()) throw new Error('Stop Console before installing another release');
      console.log(`Installed: ${await installRelease(root)}`);
      console.log('Run: doramcap-console start');
      return;
    }
    if (command === 'doctor') return doctor();
    if (command === 'stop') { await exec('systemctl', ['stop', unit]); console.log('Console stopped.'); return; }
    await privateState(state);
    if (command === 'status') {
      if (!await active()) throw new Error('Console is stopped. Run doramcap-console start');
      console.log(await readFile(launchFile, 'utf8')); return;
    }
    if (await active()) {
      if (values.port || values['browser-port'] || values.ssh)
        throw new Error('Console is already running; stop it before changing launch options');
      console.log(await readFile(launchFile, 'utf8')); return;
    }
    await doctor();
    const config = await initialize(root, state);
    await rm(launchFile, { force: true });
    const args = [join(root, 'server/dist/main.js'), config, join(root, 'frontend'), '--launch-file', launchFile];
    for (const key of ['port', 'browser-port', 'ssh'] as const) if (values[key]) args.push(`--${key}`, values[key]);
    await exec('systemd-run', ['--unit', unit, '--collect', '--service-type=exec',
      '--property=TimeoutStopSec=130', '--property=KillMode=mixed', '--', process.execPath, ...args]);
    for (let attempt = 0; attempt < 100; attempt++) {
      try { console.log(await readFile(launchFile, 'utf8')); return; }
      catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
      if (!await active()) throw new Error(`Console startup failed. Inspect: sudo journalctl -u ${unit}`);
      await delay(100);
    }
    throw new Error(`Console startup timed out. Inspect: sudo journalctl -u ${unit}`);
  } finally { lock.close(); }
}
main().catch(error => { console.error(`Console: ${error.message}`); process.exitCode = 1; });
