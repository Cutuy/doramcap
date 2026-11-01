import { once } from 'node:events';
import { setTimeout as delay } from 'node:timers/promises';
import { createHash, timingSafeEqual } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { extname, resolve, sep } from 'node:path';
import { streamArtifact } from './artifacts.js';
import { wire, type ConsoleBackend } from './native.js';
import { contract, MEDIA_TYPE, LIMITS, ERROR_STATUS, isErrorCode, artifactRoute, frame, type ErrorCode } from './generated/transport.js';

const routes = contract(wire);
export interface HttpOptions { backend: ConsoleBackend; secret: string; origin: string; assets: string }
class HttpError extends Error { constructor(public status: number, public code: ErrorCode) { super(code); } }
const hash = (s: string) => createHash('sha256').update(s).digest();

function authenticate(request: IncomingMessage, secret: string): boolean {
  const authorization = request.headers.authorization ?? '';
  if (!authorization.startsWith('Basic ')) return false;
  let supplied = '';
  try { supplied = Buffer.from(authorization.slice(6), 'base64').toString('utf8'); } catch { return false; }
  return timingSafeEqual(hash(supplied), hash(`console:${secret}`));
}
function reply(response: ServerResponse, status: number, bytes: Uint8Array): void {
  if (bytes.length > LIMITS.frameBytes) throw new HttpError(500, 'Internal');
  response.writeHead(status, { 'content-type': MEDIA_TYPE, 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  response.end(bytes);
}
async function body(request: IncomingMessage): Promise<Buffer> {
  if (request.headers['content-type']?.split(';')[0].trim() !== MEDIA_TYPE || Number(request.headers['content-length'] ?? 0) > LIMITS.bodyBytes) {
    request.resume();
    throw new HttpError(400, 'InvalidInput');
  }
  return new Promise<Buffer>((resolveBody, reject) => {
    const chunks: Buffer[] = [];
    let size = 0; let rejected = false;
    request.on('data', (chunk: Buffer) => {
      if (rejected) return;
      size += chunk.length;
      if (size > LIMITS.bodyBytes) { rejected = true; chunks.length = 0; reject(new HttpError(400, 'InvalidInput')); return; }
      chunks.push(chunk);
    });
    request.on('end', () => { if (!rejected) resolveBody(Buffer.concat(chunks)); });
    request.on('error', reject);
    request.on('aborted', () => reject(new HttpError(400, 'InvalidInput')));
  });
}
function decode<T>(codec: { decode(bytes: Uint8Array): T }, bytes: Uint8Array): T {
  try { return codec.decode(bytes); } catch { throw new HttpError(400, 'InvalidInput'); }
}
function string(value: unknown): string {
  if (typeof value !== 'string' || !value || value.length > LIMITS.stringLength) throw new HttpError(400, 'InvalidInput');
  return value;
}
function nativeError(error: unknown): HttpError {
  if (error instanceof HttpError) return error;
  const value = error as { message?: string; code?: string };
  const code = value?.message ?? '';
  if (isErrorCode(code)) return new HttpError(ERROR_STATUS[code], code);
  if (value?.code === 'QueueFull') return new HttpError(ERROR_STATUS.Busy, 'Busy');
  if (value?.code === 'Closing') return new HttpError(ERROR_STATUS.Closing, 'Closing');
  return new HttpError(500, 'Internal');
}
async function asset(response: ServerResponse, path: string, assets: string): Promise<void> {
  let name: string;
  try { name = decodeURIComponent(path === '/' ? '/index.html' : path); } catch { throw new HttpError(404, 'Missing'); }
  const root = resolve(assets);
  const target = resolve(root, `.${name}`);
  if (!target.startsWith(root + sep)) throw new HttpError(404, 'Missing');
  const info = await stat(target).catch(() => null);
  if (!info?.isFile()) throw new HttpError(404, 'Missing');
  const mime: Record<string, string> = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.svg': 'image/svg+xml', '.ico': 'image/x-icon' };
  response.writeHead(200, { 'content-type': mime[extname(target)] ?? 'application/octet-stream', 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  await new Promise<void>((ok, fail) => { const stream = createReadStream(target); stream.on('error', fail); response.on('close', () => stream.destroy()); stream.on('end', ok); stream.pipe(response); });
}

export function createConsoleServer(config: HttpOptions) {
  const origin = new URL(config.origin);
  if (!['http:', 'https:'].includes(origin.protocol) || origin.origin !== config.origin || !config.secret) throw new Error('invalid server origin or empty secret');
  let streams = 0;
  const server = createServer(async (request, response) => {
    try {
      if (!authenticate(request, config.secret)) {
        response.setHeader('www-authenticate', 'Basic realm="Console", charset="UTF-8"');
        throw new HttpError(401, 'Unauthorized');
      }
      const url = new URL(request.url ?? '/', config.origin);
      if (request.method === 'POST' && request.headers.origin !== config.origin) throw new HttpError(403, 'Unauthorized');
      if (request.method === routes.prepare.method && url.pathname === routes.prepare.path) {
        const input = await body(request), value = decode(routes.prepare.request, input);
        string(value.directory);
        if (value.profiles.length > LIMITS.selectionLength || (value.systemApps?.ids?.length ?? 0) > LIMITS.selectionLength) throw new HttpError(400, 'InvalidInput');
        value.profiles.forEach(string); value.systemApps?.ids?.forEach(string);
        reply(response, routes.prepare.status, await config.backend.prepare(input)); return;
      }
      if (request.method === routes.start.method && url.pathname === routes.start.path) {
        const input = await body(request), value = decode(routes.start.request, input);
        string(value.preparedId); string(value.requestKey);
        reply(response, routes.start.status, await config.backend.start(input)); return;
      }
      if (request.method === routes.live.method && url.pathname === routes.live.path) {
        if (request.headers.origin && request.headers.origin !== config.origin) throw new HttpError(403, 'Unauthorized');
        if (streams >= LIMITS.liveStreams) throw new HttpError(409, 'Busy');
        streams++;
        const controller = new AbortController();
        response.once('close', () => controller.abort());
        response.writeHead(routes.live.status, { 'content-type': MEDIA_TYPE, 'cache-control': 'no-store', 'x-accel-buffering': 'no', 'x-content-type-options': 'nosniff' });
        response.flushHeaders();
        let cursor = Buffer.alloc(0);
        try {
          while (!response.destroyed) {
            const tick = performance.now();
            const update = await config.backend.update(cursor);
            if (response.destroyed) break;
            const writable = response.write(frame(update.data));
            cursor = Buffer.from(update.cursor);
            if (!writable) await once(response, 'drain', {signal: AbortSignal.any([controller.signal, AbortSignal.timeout(2000)])});
            await delay(Math.max(0, LIMITS.liveIntervalMs - (performance.now() - tick)), undefined, {signal: controller.signal});
          }
        } finally { streams--; response.destroy(); }
        return;
      }
      if (request.method === routes.current.method && url.pathname === routes.current.path) {
        reply(response, routes.current.status, await config.backend.status()); return;
      }
      if (request.method === routes.stop.method && url.pathname === routes.stop.path) {
        const input = await body(request), value = decode(routes.stop.request, input);
        string(value.runId);
        reply(response, routes.stop.status, await config.backend.stop(input)); return;
      }
      const artifact = artifactRoute.exec(url.pathname);
      if (request.method === 'GET' && artifact) { await streamArtifact(config.backend, response, artifact[1], artifact[2]); return; }
      if (url.pathname.startsWith('/api/')) throw new HttpError(404, 'Missing');
      if (request.method !== 'GET') throw new HttpError(404, 'Missing');
      await asset(response, url.pathname, config.assets);
    } catch (error) {
      if (response.headersSent) { response.destroy(); return; }
      const failure = nativeError(error);
      reply(response, failure.status, routes.error.encode({ code: failure.code }).finish());
    }
  });
  return server;
}
