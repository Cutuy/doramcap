/** Interactive CUDA viewer; --hosted keeps operational files private. */
import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdir, open, readFile, readdir, realpath, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { basename, dirname, extname, resolve, sep } from 'node:path';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { doramcap } from './frontend/generated/wire.js';

const pb: any = doramcap.cuda_viewer.v1;
const maxBytes = 4 * 1024 * 1024;
export function splitFrames(data: Buffer): [Buffer[], Buffer] {
  const frames: Buffer[] = []; let offset = 0;
  while (data.length - offset >= 4) {
    const size = data.readUInt32LE(offset);
    if (!size || size > maxBytes) throw new Error('invalid activity batch size');
    if (data.length - offset < 4 + size) break;
    frames.push(data.subarray(offset + 4, offset + 4 + size)); offset += size + 4;
  }
  return [frames, data.subarray(offset)];
}
type Config = { frames: number; slots: number; consumers: number; bytes: number; failure: string; checksum: boolean };
export function validate(config: Partial<Config>): Config {
  if (!config || typeof config !== 'object' || Array.isArray(config)) throw new Error('configuration must be an object');
  const out = { frames: 120, slots: 2, consumers: 3, bytes: 4097, failure: '', checksum: false, ...config };
  for (const [name, min, max] of [['frames', 4, 2000], ['slots', 1, 16], ['consumers', 1, 8], ['bytes', 1, 1048576]] as const) {
    if (!Number.isInteger(out[name]) || out[name] < min || out[name] > max) throw new Error(`${name} must be an integer in ${min}..${max}`);
  }
  if (out.frames <= out.slots) throw new Error('frames must exceed slots to demonstrate reuse');
  if (!['', 'stall', 'duplicate'].includes(out.failure)) throw new Error('invalid failure mode');
  if (typeof out.checksum !== 'boolean') throw new Error('checksum must be boolean');
  return out;
}
type Row = { id: number; role: string; payload: Buffer };
export class Experiment {
  state = 'idle'; run = ''; serial = 0; rows: Row[] = []; bytes = 0; log = ''; config: Partial<Config> = {}; directory = ''; everFailed = false;
  active: Promise<void> | undefined;
  constructor(readonly dora: string, readonly output: string, readonly runner: string, readonly hosted = false) {}
  async start(config: Partial<Config>) {
    const valid = validate(config);
    if (this.state === 'running') throw new Error('an experiment is already running');
    this.run = randomUUID().replaceAll('-', ''); this.directory = resolve(this.output, this.run);
    this.state = 'running'; this.config = valid; this.rows = []; this.bytes = 0; this.serial = 0; this.log = '';
    this.active = this.work(valid);
  }
  async work(config: Config) {
    const offsets = new Map<string, number>(), partial = new Map<string, Buffer>();
    const ingest = async () => {
      for (const name of (await readdir(this.directory)).filter(n => n.endsWith('.activity.pb'))) {
        const file = await open(resolve(this.directory, name), 'r');
        let chunk: Buffer;
        try { const size = (await file.stat()).size, offset = offsets.get(name) ?? 0; chunk = Buffer.alloc(Math.max(0, size - offset)); const { bytesRead } = await file.read(chunk, 0, chunk.length, offset); chunk = chunk.subarray(0, bytesRead); offsets.set(name, offset + bytesRead); } finally { await file.close(); }
        const [frames, tail] = splitFrames(Buffer.concat([partial.get(name) ?? Buffer.alloc(0), chunk])); partial.set(name, tail);
        for (const payload of frames) { this.rows.push({ id: ++this.serial, role: name.split('-')[0], payload }); this.bytes += payload.length;
          while (this.bytes > maxBytes || this.rows.length > 20000) this.bytes -= this.rows.shift()!.payload.length; }
      }
    };
    try {
      await mkdir(this.directory, { recursive: true, mode: 0o700 });
      const args = [this.runner, '--dora', this.dora, ...(['frames', 'slots', 'consumers', 'bytes'] as const).flatMap(k => [`--${k}`, String(config[k])])];
      if (config.failure) args.push('--failure', config.failure);
      const env: NodeJS.ProcessEnv = { ...process.env, CUDA_SHM_TRACE_DIR: this.directory }; delete env.CUDA_SHM_CHECKSUM;
      if (config.checksum) env.CUDA_SHM_CHECKSUM = '1';
      const log = createWriteStream(resolve(this.directory, 'run.log'), { mode: 0o600 });
      const child = spawn(process.execPath, args, { env, stdio: ['ignore', 'pipe', 'pipe'] });
      child.stdout.pipe(log, { end: false }); child.stderr.pipe(log, { end: false });
      let done = false; const result = new Promise<{ code: number | null; error?: Error }>(res => { child.once('error', error => { done = true; res({ code: null, error }); }); child.once('close', code => { done = true; res({ code }); }); });
      while (!done) { await ingest(); await new Promise(res => setTimeout(res, 50)); }
      const status = await result; await ingest(); await new Promise<void>((res, rej) => { log.once('error', rej); log.end(res); });
      if (status.error) throw status.error;
      if ([...partial.values()].some(b => b.length)) throw new Error('truncated final activity batch');
      this.log = (await readFile(resolve(this.directory, 'run.log'), 'utf8')).slice(-100000);
      this.state = status.code === 0 ? 'passed' : 'failed';
    } catch (error) { this.log = String(error); this.state = 'failed'; }
    if (this.state === 'failed') this.everFailed = true;
  }
  async snapshot(since: number, run: string, state: string) {
    const reset = run !== this.run;
    if (!reset && since === this.serial && state === this.state) return null;
    const result: Record<string, unknown> = { version: 1, run: this.run, state: this.state, config: this.config, cursor: this.serial, oldest: this.rows[0]?.id ?? 0, batches: this.rows.filter(row => row.id > (reset ? 0 : since)) };
    if (!this.hosted) { result.artifacts = this.directory ? (await readdir(this.directory).catch(() => [])).sort() : []; if (reset || state !== this.state) result.log = this.log; }
    return result;
  }
}
export function serverFor(experiment: Experiment, assets: string, stopping: () => boolean) {
  return createServer(async (req, res) => {
    const respond = (status: number, data = Buffer.alloc(0), content = 'application/x-protobuf') => { res.writeHead(status, { 'Content-Type': content, 'Content-Length': data.length, 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' }); res.end(data); };
    const reply = (status: number, error = '') => respond(status, Buffer.from(pb.Reply.encode({ error }).finish()));
    try {
      const url = new URL(req.url ?? '/', 'http://local');
      if (req.method === 'GET' && url.pathname === '/api/state') {
        const since = Number(url.searchParams.get('since') ?? '0'); if (!Number.isSafeInteger(since) || since < 0) return reply(400);
        const state = await experiment.snapshot(since, url.searchParams.get('run') ?? '', url.searchParams.get('state') ?? '');
        return state ? respond(200, Buffer.from(pb.Snapshot.encode(state).finish())) : respond(204);
      }
      if (req.method === 'GET' && url.pathname.startsWith('/artifact/')) {
        const name = decodeURIComponent(url.pathname.slice('/artifact/'.length));
        if (experiment.hosted || !experiment.directory || name !== basename(name) || !/\.(mcap|pb|log)$/.test(name)) return reply(404);
        const path = await realpath(resolve(experiment.directory, name));
        if (!path.startsWith(experiment.directory + sep) || !(await stat(path)).isFile()) return reply(404);
        return respond(200, await readFile(path), 'application/octet-stream');
      }
      if (req.method === 'POST') {
        if (stopping()) return reply(503, 'shutting down');
        const scheme = experiment.hosted ? (req.headers['x-forwarded-proto'] ?? 'http') : 'http';
        const host = experiment.hosted ? (req.headers['x-forwarded-host'] ?? req.headers.host) : req.headers.host;
        if (!['http', 'https'].includes(String(scheme)) || req.headers.origin !== `${scheme}://${host}` || req.headers['content-type'] !== 'application/x-protobuf') return reply(403, 'same-origin Protobuf required');
        if (url.pathname !== '/api/run') return reply(404);
        const length = Number(req.headers['content-length']); if (!Number.isInteger(length) || length <= 0 || length > 4096) return reply(400, 'invalid request length');
        const chunks: Buffer[] = []; let size = 0;
        for await (const chunk of req) { size += chunk.length; if (size > length || size > 4096) return reply(400, 'invalid request length'); chunks.push(chunk); }
        if (size !== length || stopping()) return reply(stopping() ? 503 : 400);
        const decoded = pb.RunConfig.toObject(pb.RunConfig.decode(Buffer.concat(chunks)), { defaults: true });
        await experiment.start(decoded); return reply(202);
      }
      if (req.method === 'GET') {
        const path = await realpath(resolve(assets, '.' + (url.pathname === '/' ? '/index.html' : url.pathname)));
        const base = await realpath(assets); if (!path.startsWith(base + sep) || !(await stat(path)).isFile()) return reply(404);
        const content: Record<string, string> = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' };
        return respond(200, await readFile(path), content[extname(path)] ?? 'application/octet-stream');
      }
      reply(404);
    } catch (error) { reply(400, experiment.hosted ? 'invalid request' : error instanceof Error ? error.message : 'invalid request'); }
  });
}

export async function main() {
  const { values } = parseArgs({ options: { hosted: { type: 'boolean' }, bind: { type: 'string' }, port: { type: 'string' }, dora: { type: 'string', default: 'dora' }, output: { type: 'string' }, assets: { type: 'string' } } });
  const hosted = !!values.hosted, port = Number(values.port ?? (hosted ? 8080 : 8768));
  const root = dirname(fileURLToPath(import.meta.url)), runner = resolve(root, 'run.mjs');
  const results = resolve(values.output ?? (hosted ? '/results' : '/tmp/cuda-shm-observations'));
  const assets = resolve(values.assets ?? resolve(root, '../frontend/dist'));
  await mkdir(results, { recursive: true, mode: 0o700 });
  let stopping = false; const stop = () => { stopping = true; }; process.on('SIGTERM', stop); process.on('SIGINT', stop);
  if (hosted) {
    const log = createWriteStream(resolve(results, 'preflight.log'), { flags: 'wx', mode: 0o600 });
    const env = { ...process.env }; delete env.CUDA_SHM_TRACE_DIR;
    const child = spawn(process.execPath, [runner, '--dora', values.dora, '--frames', '4', '--slots', '2', '--consumers', '1'], { env, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.pipe(log, { end: false }); child.stderr.pipe(log, { end: false });
    const timer = setTimeout(() => child.kill('SIGKILL'), 75_000);
    const status = await new Promise<number | null>((res, rej) => { child.once('error', rej); child.once('close', res); }); clearTimeout(timer); log.end();
    if (status !== 0) throw new Error('CUDA preflight failed; details are retained privately');
    if (stopping) { await writeFile(resolve(results, 'shutdown.complete'), 'complete\n'); return; }
  }
  const experiment = new Experiment(values.dora, hosted ? resolve(results, 'runs') : results, runner, hosted);
  const server = serverFor(experiment, assets, () => stopping);
  await new Promise<void>((res, rej) => { server.once('error', rej); server.listen(port, values.bind ?? (hosted ? '0.0.0.0' : '127.0.0.1'), res); });
  console.log(`CUDA SHM viewer listening on ${port}`);
  await new Promise<void>(res => { if (stopping) return res(); process.once('SIGINT', res); process.once('SIGTERM', res); });
  server.close();
  if (experiment.active) await Promise.race([experiment.active, new Promise((_, rej) => setTimeout(() => rej(new Error('active CUDA run did not finish')), 75_000).unref())]);
  if (experiment.everFailed) throw new Error('CUDA run failed; results remain private');
  if (hosted) await writeFile(resolve(results, 'shutdown.complete'), 'complete\n');
}
