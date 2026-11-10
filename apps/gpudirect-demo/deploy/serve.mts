import { spawn } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { access, mkdir, open } from 'node:fs/promises';
import { constants } from 'node:fs';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { isAbsolute, join } from 'node:path';
import { pathToFileURL } from 'node:url';

const MODES = ['mapped', 'staged', 'io-only'] as const;
const PAYLOAD_BYTES = 1024 * 1024;
const MESSAGES = 32;
const SLABS = 4;
type Measurement = { mode: string; throughput: number; cpu: number; seconds: number; line: string };

async function record(binary: string, directory: string, mode: string): Promise<Measurement> {
  const child = spawn(binary, [join(directory, `${mode}.mcap`), mode, String(PAYLOAD_BYTES), String(MESSAGES), String(SLABS)], { stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '', stderr = '';
  child.stdout.setEncoding('utf8').on('data', chunk => { stdout += chunk; });
  child.stderr.setEncoding('utf8').on('data', chunk => { stderr += chunk; });
  const [code, signal] = await once(child, 'close');
  if (code !== 0 || !stdout.includes('verified=true')) throw new Error(`${mode}: ${stderr.trim() || signal || `exit ${code}`}`);
  const metric = (key: string) => {
    const value = Number(stdout.match(new RegExp(`(?:^| )${key}=([^ \n]+)`))?.[1]);
    if (!Number.isFinite(value)) throw new Error(`Missing metric: ${key}`);
    return value;
  };
  return { mode, throughput: metric('payload_mib_s'), cpu: metric('cpu_pct_one_core'), seconds: metric('wall_s'), line: stdout.trim() };
}

/** Run fixed, finite CUDA recordings; readiness follows independent byte verification. */
export async function start(binary: string, directory: string, port = 8080) {
  if (!isAbsolute(binary) || !isAbsolute(directory)) throw new Error('Binary and results paths must be absolute');
  await access(binary, constants.X_OK);
  const diagram = new URL('../architecture.svg', import.meta.url);
  await access(diagram);
  await mkdir(directory, { recursive: true });
  const measurements: Measurement[] = [];
  let ready = false, stopping = false;
  const downloads = new Set([...MODES.map(mode => `${mode}.mcap`), 'stats.txt']);
  const server = createServer((request, response) => {
    const path = (request.url || '/').split('?')[0];
    if (path === '/health') {
      response.writeHead(ready ? 200 : 503).end(ready ? 'verified' : 'recording');
    } else if (path === '/architecture.svg') {
      response.writeHead(200, { 'content-type': 'image/svg+xml' });
      createReadStream(diagram).on('error', error => response.destroy(error)).pipe(response);
    } else if (!ready) {
      response.writeHead(503).end('Recording GPU output…');
    } else if (path === '/') {
      const rows = measurements.map(m => `<tr><td><a href="./${m.mode}.mcap">${m.mode}</a></td><td>${m.throughput.toFixed(3)}</td><td>${m.cpu.toFixed(3)}%</td><td>${m.seconds.toFixed(3)}</td></tr>`).join('');
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' }).end(`<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>GPU storage demo</title>
<style>body{font:16px system-ui;color:#0f172a;background:#f8fafc;margin:0}main{max-width:1120px;margin:auto;padding:24px}h1{font-size:26px}img{width:100%;height:auto}table{width:100%;border-collapse:collapse;background:white}th,td{padding:12px;text-align:right;border-bottom:1px solid #cbd5e1}th:first-child,td:first-child{text-align:left}a{color:#155e75}p{color:#475569}@media(max-width:540px){main{padding:12px}th,td{padding:8px;font-size:13px}}</style></head>
<body><main><h1>GPU storage demo</h1><img src="./architecture.svg" alt="CUDA and io_uring share pinned slabs; GPU completion precedes writing and I/O completion permits reuse">
<p>${MESSAGES} × ${PAYLOAD_BYTES / 1048576} MiB per mode · ${SLABS} reusable slabs · Every payload byte verified</p>
<table><thead><tr><th>Mode / MCAP</th><th>Payload MiB/s</th><th>CPU, one core</th><th>Seconds</th></tr></thead><tbody>${rows}</tbody></table><p><a href="./stats.txt">Full measurements</a></p></main></body></html>`);
    } else if (downloads.has(path.slice(1))) {
      response.writeHead(200, { 'content-type': 'application/octet-stream', 'content-disposition': `attachment; filename="${path.slice(1)}"` });
      createReadStream(join(directory, path.slice(1))).on('error', error => response.destroy(error)).pipe(response);
    } else response.writeHead(404).end();
  });
  server.listen(port, '0.0.0.0');
  await once(server, 'listening');
  async function writeNew(name: string, content: string) {
    const file = await open(join(directory, name), 'wx', 0o600);
    try { await file.writeFile(content); await file.sync(); } finally { await file.close(); }
  }
  const completion = (async () => {
    for (const mode of MODES) {
      if (stopping) return;
      measurements.push(await record(binary, directory, mode));
    }
    await writeNew('stats.txt', measurements.map(m => m.line).join('\n') + '\n');
    ready = !stopping;
  })();
  let stopPromise: Promise<void> | undefined;
  function stop() {
    return stopPromise ??= (async () => {
      stopping = true;
      // Let the active finite CUDA job finish before closing its process and output.
      try { await completion; } finally {
        ready = false;
        server.closeAllConnections();
        await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
      }
      if (measurements.length === MODES.length) await writeNew('shutdown.complete', 'verified\n');
    })();
  }
  return { server, completion, stop };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.length !== 4) throw new Error('usage: node serve.mts BINARY RESULTS_DIRECTORY');
  const app = await start(process.argv[2], process.argv[3]);
  const fail = (error: unknown) => { console.error(error); process.exitCode = 1; };
  const stop = () => { void app.stop().catch(fail); };
  process.once('SIGTERM', stop);
  process.once('SIGINT', stop);
  void app.completion.catch(error => { fail(error); void app.stop().catch(() => {}); });
}
