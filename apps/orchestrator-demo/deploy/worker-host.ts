/** Supervise the Docker worker and retain its diagnostics privately. */
import { spawn } from 'node:child_process';
import { constants, createWriteStream } from 'node:fs';
import { lstat, mkdir, open, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
let stopping = false;
const stop = () => { stopping = true; worker?.kill('SIGTERM'); };
process.on('SIGINT', stop); process.on('SIGTERM', stop);
let worker: ReturnType<typeof spawn> | undefined;
async function main() {
  const root = process.env.ORCHESTRATOR_RESULTS_DIR!, prefix = process.env.ORCHESTRATOR_RUN_ID!, runs = resolve(root, 'runs');
  await mkdir(runs, { recursive: true, mode: 0o700 });
  const socket = '/socket/worker.sock', deadline = Date.now() + 60_000;
  while (!(await lstat(socket).catch(() => null))?.isSocket()) {
    if (stopping) return;
    if (Date.now() >= deadline) throw new Error('orchestrator control socket did not become available');
    await delay(100);
  }
  const env = { ...process.env, ORCHESTRATOR_DOCKER_SOCKET: '/var/run/docker.sock', ORCHESTRATOR_RUN_PREFIX: prefix, ORCHESTRATOR_RECOVERY_DIRECTORY: '/recovery', ORCHESTRATOR_PRIVATE_TELEMETRY: '1' };
  const log = createWriteStream('/recovery/worker.log', { flags: 'wx', mode: 0o600 });
  await new Promise<void>((res, rej) => { log.once('open', () => res()); log.once('error', rej); });
  worker = spawn('/usr/local/bin/orchestrator-worker', [socket, process.env.ORCHESTRATOR_MONITOR_IMAGE!, runs], { env, stdio: ['ignore', 'pipe', 'pipe'] });
  worker.stdout!.pipe(log, { end: false }); worker.stderr!.pipe(log, { end: false });
  if (stopping) worker.kill('SIGTERM');
  let timer: ReturnType<typeof setTimeout> | undefined;
  const drain = () => { timer ??= setTimeout(() => worker?.kill('SIGKILL'), 90_000); };
  process.on('SIGINT', drain); process.on('SIGTERM', drain); if (stopping) drain();
  const status = await new Promise<number | null>((res, rej) => { worker!.once('error', rej); worker!.once('close', res); });
  if (timer) clearTimeout(timer); log.end();
  if (status !== 0) throw new Error('orchestrator worker failed; details are retained privately');
  const escaped = prefix.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const names = (await readdir(runs)).filter(name => new RegExp(`^${escaped}-[1-9][0-9]{0,2}$`).test(name));
  if (names.length > 999) throw new Error('invalid nested experiment directory');
  for (const name of names) {
    const dir = resolve(runs, name); if (!(await lstat(dir)).isDirectory()) throw new Error('invalid nested experiment directory');
    const file = await open(resolve(dir, 'report.json'), constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    try {
      const info = await file.stat(); if (!info.isFile() || info.size > 2 * 1024 * 1024) throw new Error('invalid nested experiment report');
      const data = Buffer.alloc(2 * 1024 * 1024 + 1);
      const { bytesRead } = await file.read(data, 0, data.length, 0);
      if (bytesRead > 2 * 1024 * 1024) throw new Error('invalid nested experiment report');
      const report = JSON.parse(data.subarray(0, bytesRead).toString('utf8'));
      if (!report.boundary_clean || !report.recording_finalized || report.execution_error || report.diagnostics) throw new Error('nested experiment is incomplete; details remain private');
    } finally { await file.close(); }
  }
  await writeFile(resolve(root, 'shutdown.complete'), 'complete\n');
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
