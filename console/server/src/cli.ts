import { execFile, spawn } from 'node:child_process';
import { readFile, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { capabilities } from './native.js';
import { launchInstructions, portNumber } from './launch.js';
import { doctor, initialize, installRelease, privateState, readRelease } from './release.js';
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

/** Current access instructions; a running service without them is starting or uses another state root. */
async function access() {
  return readFile(launchFile, 'utf8').catch(error => {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    throw new Error(`Console is running without an access link at ${launchFile}; it is still starting or uses another --state`);
  });
}

/** Startup failure with the service's own recent output; a collected unit keeps its journal. */
async function startupFailure(summary: string, since: number) {
  const log = await exec('journalctl', ['-u', unit, '--since', `@${since}`, '-n', '10', '--no-pager', '-o', 'cat'], { timeout: 5_000 })
    .then(({ stdout }) => stdout.trim(), () => '');
  return new Error(`${summary}.${log ? `\n${log}` : ''}\nInspect: sudo journalctl -u ${unit}`);
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
      for (const line of lines) if (command === 'start' && !opened && !values.ssh && !values['no-open'] && line.startsWith('Launch browser: ')) {
        opened = true;
        const browser = spawn('xdg-open', [line.slice('Launch browser: '.length)], { stdio: 'ignore' });
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
    const release = await readRelease(root);
    const socket = release.config!.dockerSocket || '/var/run/docker.sock';
    if (command === 'doctor') return await doctor(socket);
    if (command === 'stop') {
      if (!await active()) { console.log('Console is already stopped.'); return; }
      await exec('systemctl', ['stop', unit]);
      await rm(launchFile, { force: true });
      console.log(`Console stopped. Recordings remain under ${state}.`);
      return;
    }
    if (command === 'status') {
      // Exit status 3 follows LSB: not running.
      if (!await active()) { console.log('Console is stopped. Run: doramcap-console start'); process.exitCode = 3; return; }
      const { stdout } = await exec('systemctl', ['show', '--value', '--property=ActiveEnterTimestamp', unit]);
      console.log(`Console ${release.version} running since ${stdout.trim()}.\n\n${await access()}`);
      return;
    }
    await privateState(state);
    if (await active()) {
      if (values.port || values['browser-port'] || values.ssh)
        throw new Error('Console is already running; stop it before changing launch options');
      console.log(await access()); return;
    }
    await doctor(socket);
    const config = await initialize(root, state);
    await rm(launchFile, { force: true });
    const since = Math.floor(Date.now() / 1000);
    const args = [join(root, 'server/dist/main.js'), config, join(root, 'frontend'), '--launch-file', launchFile];
    for (const key of ['port', 'browser-port', 'ssh'] as const) if (values[key]) args.push(`--${key}`, values[key]);
    await exec('systemd-run', ['--unit', unit, '--collect', '--service-type=exec',
      '--property=TimeoutStopSec=130', '--property=KillMode=mixed', '--', process.execPath, ...args]);
    for (let attempt = 0; attempt < 100; attempt++) {
      try {
        const instructions = await readFile(launchFile, 'utf8');
        console.log(instructions.replace(/^Open: /m, 'Launch browser: '));
        return;
      } catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
      if (!await active()) throw await startupFailure('Console startup failed', since);
      await delay(100);
    }
    throw await startupFailure('Console startup timed out', since);
  } finally { lock.close(); }
}
main().catch(error => { console.error(`Console: ${error.message}`); process.exitCode = 1; });
