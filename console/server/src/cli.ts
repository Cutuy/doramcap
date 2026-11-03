import { execFile, spawn } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { readFile, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { capabilities } from './native.js';
import { launchInstructions, portNumber } from './launch.js';
import { doctor, initialize, installRelease, privateState, projectId, readRelease } from './release.js';
import { discover } from './system-apps.js';
const exec = promisify(execFile);
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values, positionals } = parseArgs({ allowPositionals: true, options: {
  state: { type: 'string', default: '/var/lib/doramcap-console' },
  port: { type: 'string' }, 'browser-port': { type: 'string' }, ssh: { type: 'string' },
  project: { type: 'string' }, 'no-open': { type: 'boolean' }, help: { type: 'boolean' },
} });
const command = positionals[0] ?? 'start';
const project = (() => {
  const path = resolve(values.project ?? process.cwd());
  // A deleted project can still be stopped by the path it had.
  try { return realpathSync(path); } catch { return path; }
})();
const stateRoot = join(resolve(values.state!), 'projects');
const id = projectId(project);
const unit = `doramcap-console-${id}.service`;
const state = join(stateRoot, id);
const launchFile = join(state, 'access.txt');
const HELP = `Usage: doramcap-console [COMMAND] [OPTIONS]

Each Compose project has its own Console service, database and recordings.

Commands:
  start    Start this project's service and print its access link (default). Opens a browser on a desktop.
  status   Print the release, start time and access link. Exits 3 when stopped.
  stop     Stop any active run, finalize it and remove the access link.
  list     List running Consoles and their projects.
  doctor   Check host requirements and print a fix for each failure.
  install  Activate this release. The installer runs this.

Options:
  --project PATH       Compose project directory. Default: the current directory.
  --state PATH         Root-owned, mode-0700 state directory. Default: /var/lib/doramcap-console.
  --help               Print this help.

Options for start:
  --ssh USER@HOST      Print a forwarding command to run on the browser computer. Keep it open.
  --port PORT          Use this exact server port.
  --browser-port PORT  Use this port on the browser computer.
  --no-open            Do not open a browser.

Try the sample project: doramcap-console start --project ${join(root, 'playground/cpu-burn')}

The service survives SSH disconnection. Stop it before changing launch options.
Access links and browser sessions expire on restart; keep links private.
A moved project directory starts with a new history; its old state stays behind.

Files, one directory per project under STATE/projects/:
  console.pbtxt  Service configuration, pinned to the release that wrote it. Edit with sudo.
  console.db     Run history.
  runs/          Recordings.

Logs: sudo journalctl -u doramcap-console-<project>.service`;

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
    console.log(HELP);
    return;
  }
  if (!['start', 'stop', 'status', 'list', 'doctor', 'install'].includes(command) || positionals.length > 1)
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
  // Projects run independently; install changes what all of them share.
  const lock = capabilities.lockFile(command === 'install' ? '/run/doramcap-console-cli.lock' : `/run/doramcap-console-cli-${id}.lock`);
  try {
    if (command === 'install') {
      console.log(`Installed: ${await installRelease(root)}`);
      console.log('Run: doramcap-console start');
      return;
    }
    const release = await readRelease(root);
    const socket = release.config!.dockerSocket || '/var/run/docker.sock';
    if (command === 'doctor') return await doctor(socket);
    if (command === 'list') {
      const { stdout } = await exec('systemctl', ['list-units', '--type=service', '--state=active', '--plain', '--no-legend', 'doramcap-console-*']);
      const units = stdout.split('\n').map(line => line.split(' ')[0]).filter(Boolean);
      if (!units.length) console.log('No Console is running.');
      for (const name of units) {
        const path = await readFile(join(stateRoot, name.replace(/^doramcap-console-|\.service$/g, ''), 'project.txt'), 'utf8').catch(() => 'another --state\n');
        console.log(`${name}  ${path.trim()}`);
      }
      return;
    }
    if (command === 'stop') {
      if (!await active()) { console.log(`Console is already stopped for ${project}.`); return; }
      await exec('systemctl', ['stop', unit]);
      await rm(launchFile, { force: true });
      console.log(`Console stopped for ${project}. Recordings remain under ${state}.`);
      return;
    }
    if (command === 'status') {
      // Exit status 3 follows LSB: not running.
      if (!await active()) { console.log(`Console is stopped for ${project}. Run: doramcap-console start`); process.exitCode = 3; return; }
      const { stdout } = await exec('systemctl', ['show', '--value', '--property=ActiveEnterTimestamp', unit]);
      console.log(`Console ${release.version} running since ${stdout.trim()}.\n\n${await access()}`);
      return;
    }
    if (await active()) {
      if (values.port || values['browser-port'] || values.ssh)
        throw new Error('Console is already running; stop it before changing launch options');
      console.log(await access()); return;
    }
    await discover(project, []).catch(error => { throw new Error(`No Compose project at ${project}: ${error.message}`); });
    await privateState(dirname(stateRoot));
    await privateState(stateRoot);
    await privateState(state);
    await doctor(socket);
    const config = await initialize(root, project, state);
    await rm(launchFile, { force: true });
    const since = Math.floor(Date.now() / 1000);
    const args = [join(root, 'server/dist/main.js'), config, join(root, 'frontend'), '--launch-file', launchFile];
    for (const key of ['port', 'browser-port', 'ssh'] as const) if (values[key]) args.push(`--${key}`, values[key]);
    // systemd-run returns once the server reports ready; systemd stops one that never does.
    await exec('systemd-run', ['--unit', unit, '--collect', '--service-type=notify', '--property=NotifyAccess=all',
      '--property=TimeoutStartSec=60', '--property=TimeoutStopSec=130', '--property=KillMode=mixed', '--', process.execPath, '--disable-warning=ExperimentalWarning', ...args])
      .catch(async () => { throw await startupFailure('Console startup failed', since); });
    console.log((await readFile(launchFile, 'utf8')).replace(/^Open: /m, 'Launch browser: '));
  } finally { lock.close(); }
}
main().catch(error => { console.error(`Console: ${error.message}`); process.exitCode = 1; });
