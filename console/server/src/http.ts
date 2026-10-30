import { once } from 'node:events';
import { setTimeout as delay } from 'node:timers/promises';
import { createHash, timingSafeEqual } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { extname, resolve, sep } from 'node:path';
import { streamArtifact } from './artifacts.js';
import { wire, type NativeConsole } from './native.js';

const MAX_BODY = 64 * 1024;
const options = { longs: String, enums: String, defaults: true } as const;
export interface HttpOptions { native: NativeConsole; secret: string; origin: string; assets: string }
class HttpError extends Error { constructor(public status: number, public code: string) { super(code); } }
const hash = (s: string) => createHash('sha256').update(s).digest();

function authenticate(request: IncomingMessage, secret: string): boolean {
  const authorization = request.headers.authorization ?? '';
  if (!authorization.startsWith('Basic ')) return false;
  let supplied = '';
  try { supplied = Buffer.from(authorization.slice(6), 'base64').toString('utf8'); } catch { return false; }
  return timingSafeEqual(hash(supplied), hash(`console:${secret}`));
}
function reply(response: ServerResponse, status: number, body: unknown): void {
  response.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  response.end(JSON.stringify(body));
}
async function body(request: IncomingMessage): Promise<Record<string, unknown>> {
  if (Number(request.headers['content-length'] ?? 0) > MAX_BODY) {
    request.resume();
    throw new HttpError(400, 'InvalidInput');
  }
  const bytes = await new Promise<Buffer>((resolveBody, reject) => {
    const chunks: Buffer[] = [];
    let size = 0; let rejected = false;
    request.on('data', (chunk: Buffer) => {
      if (rejected) return;
      size += chunk.length;
      if (size > MAX_BODY) { rejected = true; chunks.length = 0; reject(new HttpError(400, 'InvalidInput')); return; }
      chunks.push(chunk);
    });
    request.on('end', () => { if (!rejected) resolveBody(Buffer.concat(chunks)); });
    request.on('error', reject);
    request.on('aborted', () => reject(new HttpError(400, 'InvalidInput')));
  });
  let value: unknown;
  try { value = JSON.parse(bytes.toString('utf8')); } catch { throw new HttpError(400, 'InvalidInput'); }
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new HttpError(400, 'InvalidInput');
  return value as Record<string, unknown>;
}

function fields(value: Record<string, unknown>, expected: string[]): void {
  if (Object.keys(value).some(key => !expected.includes(key)) || expected.some(key => !(key in value))) throw new HttpError(400, 'InvalidInput');
}
function string(value: unknown): string {
  if (typeof value !== 'string' || !value || value.length > 4096) throw new HttpError(400, 'InvalidInput');
  return value;
}
function nativeError(error: unknown): HttpError {
  if (error instanceof HttpError) return error;
  const value = error as { message?: string; code?: string };
  const code = value?.message ?? '';
  if (code === 'InvalidInput') return new HttpError(400, code);
  if (code === 'Missing') return new HttpError(404, code);
  if (code === 'Conflict' || code === 'StaleReview') return new HttpError(409, code);
  if (value?.code === 'QueueFull' || value?.code === 'Closing') return new HttpError(503, 'Busy');
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
  return createServer(async (request, response) => {
    try {
      if (!authenticate(request, config.secret)) {
        response.setHeader('www-authenticate', 'Basic realm="Console", charset="UTF-8"');
        throw new HttpError(401, 'Unauthorized');
      }
      const url = new URL(request.url ?? '/', config.origin);
      if (request.method === 'POST' && request.headers.origin !== config.origin) throw new HttpError(403, 'Unauthorized');
      if (request.method === 'POST' && url.pathname === '/api/prepare') {
        const value = await body(request); fields(value, ['directory', 'profiles']);
        if (!Array.isArray(value.profiles) || value.profiles.length > 128) throw new HttpError(400, 'InvalidInput');
        const input = wire.PrepareRequest.encode({ directory: string(value.directory), profiles: value.profiles.map(string) }).finish();
        const result = wire.Review.decode(await config.native.prepare(Buffer.from(input)));
        reply(response, 200, wire.Review.toObject(result, options)); return;
      }
      if (request.method === 'POST' && url.pathname === '/api/start') {
        const value = await body(request); fields(value, ['preparedId', 'requestKey']);
        const input = wire.StartRequest.encode({ preparedId: string(value.preparedId), requestKey: string(value.requestKey) }).finish();
        const result = wire.StartResponse.decode(await config.native.start(Buffer.from(input)));
        reply(response, 202, wire.StartResponse.toObject(result, options)); return;
      }
      if (request.method === 'GET' && url.pathname === '/api/live') {
        if (request.headers.origin && request.headers.origin !== config.origin) throw new HttpError(403, 'Unauthorized');
        if (streams >= 4) throw new HttpError(409, 'Busy');
        streams++;
        const controller = new AbortController();
        response.once('close', () => controller.abort());
        response.writeHead(200, { 'content-type': 'application/x-protobuf', 'cache-control': 'no-store', 'x-accel-buffering': 'no', 'x-content-type-options': 'nosniff' });
        response.flushHeaders();
        let cursor = Buffer.alloc(0);
        try {
          while (!response.destroyed) {
            const tick = performance.now();
            const update = await config.native.update(cursor);
            if (response.destroyed) break;
            if (update.data.length > 4 * 1024 * 1024) throw new Error('live frame exceeds limit');
            const header = Buffer.allocUnsafe(4); header.writeUInt32LE(update.data.length);
            const writable = response.write(Buffer.concat([header, update.data]));
            cursor = Buffer.from(update.cursor);
            if (!writable) await once(response, 'drain', {signal: AbortSignal.any([controller.signal, AbortSignal.timeout(2000)])});
            await delay(Math.max(0, 100 - (performance.now() - tick)), undefined, {signal: controller.signal});
          }
        } finally { streams--; response.destroy(); }
        return;
      }
      if (request.method === 'GET' && url.pathname === '/api/current') {
        const result = wire.Status.decode(await config.native.status());
        reply(response, 200, wire.Status.toObject(result, options)); return;
      }
      if (request.method === 'POST' && url.pathname === '/api/stop') {
        const value = await body(request); fields(value, ['runId']);
        const input = wire.StopRequest.encode({ runId: string(value.runId) }).finish();
        const result = wire.StopResponse.decode(await config.native.stop(Buffer.from(input)));
        reply(response, 200, wire.StopResponse.toObject(result, options)); return;
      }
      const artifact = /^\/api\/artifacts\/([a-f0-9]{32})\/(cpu|report)$/.exec(url.pathname);
      if (request.method === 'GET' && artifact) { await streamArtifact(config.native, response, artifact[1], artifact[2]); return; }
      if (url.pathname.startsWith('/api/')) throw new HttpError(404, 'Missing');
      if (request.method !== 'GET') throw new HttpError(404, 'Missing');
      await asset(response, url.pathname, config.assets);
    } catch (error) {
      if (response.headersSent) { response.destroy(); return; }
      const failure = nativeError(error);
      reply(response, failure.status, { error: { code: failure.code } });
    }
  });
}
