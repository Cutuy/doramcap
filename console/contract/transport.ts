import { doramcap } from './console.js';

/** Browser/server transport; payload codecs are generated from console.proto. */
export const MEDIA_TYPE = 'application/x-protobuf';
export const LIMITS = {
  bodyBytes: 64 * 1024,
  frameBytes: 4 * 1024 * 1024,
  stringLength: 4096,
  selectionLength: 128,
  liveStreams: 4,
  liveIntervalMs: 100,
} as const;
export const ERROR_STATUS = {
  Unauthorized: 401,
  InvalidInput: 400,
  Missing: 404,
  Conflict: 409,
  StaleReview: 409,
  Stopped: 409,
  PreflightRejected: 409,
  Busy: 503,
  Closing: 503,
  Internal: 500,
} as const;
export type ErrorCode = keyof typeof ERROR_STATUS;
export const isErrorCode = (code: string): code is ErrorCode => Object.hasOwn(ERROR_STATUS, code);

// Both builds copy this module beside their generated console.js.
const wire = doramcap.console.v1;
export const routes = {
  prepare: { method: 'POST', path: '/api/prepare', status: 200, request: wire.PrepareRequest, response: wire.Review },
  start: { method: 'POST', path: '/api/start', status: 202, request: wire.StartRequest, response: wire.StartResponse },
  stop: { method: 'POST', path: '/api/stop', status: 200, request: wire.StopRequest, response: wire.StopResponse },
  current: { method: 'GET', path: '/api/current', status: 200, response: wire.Status },
  live: { method: 'GET', path: '/api/live', status: 200, response: wire.Status },
  error: wire.ConsoleError,
} as const;
export const artifactPath = (runId: string, kind: string) =>
  `/api/artifacts/${encodeURIComponent(runId)}/${encodeURIComponent(kind)}`;
export const artifactRoute = /^\/api\/artifacts\/([a-f0-9]{32})\/([A-Za-z0-9-]+)$/;

/** Prefix one bounded protobuf payload with its four-byte little-endian length. */
export function frame(payload: Uint8Array): Uint8Array {
  if (!payload.length || payload.length > LIMITS.frameBytes) throw new Error('Invalid live frame length');
  const bytes = new Uint8Array(4 + payload.length);
  new DataView(bytes.buffer).setUint32(0, payload.length, true);
  bytes.set(payload, 4);
  return bytes;
}
/** Decode framed messages across arbitrary HTTP chunk boundaries. */
export class FramedMessages<T> {
  private pending = new Uint8Array(0);
  constructor(private decode: (bytes: Uint8Array) => T) {}
  receive(chunk: Uint8Array, consume: (message: T) => void) {
    if (chunk.length + this.pending.length > LIMITS.frameBytes * 2 + 8)
      throw new Error('Live buffer exceeds limit');
    const bytes = new Uint8Array(this.pending.length + chunk.length);
    bytes.set(this.pending);
    bytes.set(chunk, this.pending.length);
    let offset = 0;
    while (bytes.length - offset >= 4) {
      const length = new DataView(bytes.buffer, bytes.byteOffset + offset, 4).getUint32(0, true);
      if (!length || length > LIMITS.frameBytes) throw new Error('Invalid live frame length');
      if (bytes.length - offset - 4 < length) break;
      consume(this.decode(bytes.subarray(offset + 4, offset + 4 + length)));
      offset += 4 + length;
    }
    this.pending = bytes.slice(offset);
  }
}
